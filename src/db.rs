use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

use crate::models::{
    AnalogIngestJob, Ticket, User, UserCamera, UserLens, ANALOG_INGEST_STATUS_DONE,
    ANALOG_INGEST_STATUS_DOWNLOADING, ANALOG_INGEST_STATUS_FAILED, ANALOG_INGEST_STATUS_LABELING,
    ANALOG_INGEST_STATUS_PREVIEW, ANALOG_INGEST_STATUS_QUEUED, ANALOG_INGEST_STATUS_UPLOADING,
};

const TICKET_COLUMNS: &str = r#"
    id, user_id, order_number, label, customer_no, shop_no, order_no,
    summary_state_code, summary_state_text, status, completed,
    created_at, last_updated, completed_at, camera_id, lens_id, film_iso
"#;

const ANALOG_INGEST_JOB_COLUMNS: &str = r#"
    id, user_id, order_number, secure_id, camera_label, album, status, error_text,
    created_at, updated_at, camera_id, lens_id, film_iso
"#;

const USER_CAMERA_COLUMNS: &str = "id, user_id, label, created_at";
const USER_LENS_COLUMNS: &str = "id, user_id, name, focal_mm, aperture, created_at";

fn is_unique_violation(err: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = err {
        return db_err.is_unique_violation();
    }
    err.to_string().contains("UNIQUE")
}

/// Trim whitespace; reject empty camera labels.
pub fn normalize_camera_label(label: &str) -> Result<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        anyhow::bail!("camera label must not be empty");
    }
    Ok(trimmed.to_string())
}

/// Trim whitespace; reject empty lens names.
pub fn normalize_lens_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        anyhow::bail!("lens name must not be empty");
    }
    Ok(trimmed.to_string())
}

pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    if let Some(path) = sqlite_path_from_url(database_url) {
        if let Some(parent) = Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.with_context(|| {
                    format!("failed to create database parent dir {}", parent.display())
                })?;
            }
        }
    }

    let options = SqliteConnectOptions::from_str(database_url)
        .with_context(|| format!("invalid DATABASE_URL: {database_url}"))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .context("failed to open sqlite database")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;

    Ok(pool)
}

fn sqlite_path_from_url(url: &str) -> Option<String> {
    let trimmed = url.strip_prefix("sqlite://").unwrap_or(url);
    let trimmed = trimmed.strip_prefix("sqlite:").unwrap_or(trimmed);
    if trimmed.is_empty() || trimmed == ":memory:" {
        return None;
    }
    let path_only = trimmed.split('?').next().unwrap_or(trimmed);
    Some(path_only.to_string())
}

pub async fn upsert_discord_user(
    pool: &SqlitePool,
    discord_id: &str,
    username: &str,
) -> Result<User> {
    sqlx::query(
        r#"
        INSERT INTO users (discord_id, username)
        VALUES (?1, ?2)
        ON CONFLICT(discord_id) DO UPDATE SET
            username   = excluded.username,
            last_login = datetime('now')
        "#,
    )
    .bind(discord_id)
    .bind(username)
    .execute(pool)
    .await
    .context("failed to upsert discord user")?;

    find_by_discord_id(pool, discord_id)
        .await?
        .context("user vanished after upsert")
}

/// Return the user for `discord_id`, creating a minimal record with
/// `fallback_username` if none exists. Unlike [`upsert_discord_user`] this does
/// not overwrite an existing user's username.
pub async fn ensure_user(
    pool: &SqlitePool,
    discord_id: &str,
    fallback_username: &str,
) -> Result<User> {
    if let Some(user) = find_by_discord_id(pool, discord_id).await? {
        return Ok(user);
    }
    sqlx::query("INSERT INTO users (discord_id, username) VALUES (?1, ?2)")
        .bind(discord_id)
        .bind(fallback_username)
        .execute(pool)
        .await
        .context("failed to insert user")?;
    find_by_discord_id(pool, discord_id)
        .await?
        .context("user vanished after insert")
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, discord_id, username, created_at, last_login FROM users WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("failed to query user by id")?;
    Ok(user)
}

pub async fn find_by_discord_id(pool: &SqlitePool, discord_id: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, discord_id, username, created_at, last_login FROM users WHERE discord_id = ?1",
    )
    .bind(discord_id)
    .fetch_optional(pool)
    .await
    .context("failed to query user by discord id")?;
    Ok(user)
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, discord_id, username, created_at, last_login FROM users ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .context("failed to list users")?;
    Ok(users)
}

pub async fn delete_user(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .context("failed to delete user")?;
    Ok(result.rows_affected() > 0)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_ticket(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
    label: Option<&str>,
    customer_no: Option<&str>,
    shop_no: Option<&str>,
    order_no: Option<&str>,
    summary_state_code: &str,
    summary_state_text: Option<&str>,
) -> Result<Ticket> {
    let id = sqlx::query(
        r#"
        INSERT INTO tickets
            (user_id, order_number, label, customer_no, shop_no, order_no,
             summary_state_code, summary_state_text, last_updated)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))
        "#,
    )
    .bind(user_id)
    .bind(order_number)
    .bind(label)
    .bind(customer_no)
    .bind(shop_no)
    .bind(order_no)
    .bind(summary_state_code)
    .bind(summary_state_text)
    .execute(pool)
    .await
    .context("failed to create ticket")?
    .last_insert_rowid();

    find_ticket_by_id(pool, id)
        .await?
        .context("ticket vanished after insert")
}

/// Latest open (not completed) ticket for this user and order number, if any.
pub async fn find_open_ticket_for_user_order(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
) -> Result<Option<Ticket>> {
    let query = format!(
        "SELECT {TICKET_COLUMNS} FROM tickets \
         WHERE user_id = ?1 AND order_number = ?2 AND completed = 0 \
         ORDER BY id DESC LIMIT 1"
    );
    let ticket = sqlx::query_as::<_, Ticket>(&query)
    .bind(user_id)
    .bind(order_number)
    .fetch_optional(pool)
    .await
    .context("failed to find open ticket")?;
    Ok(ticket)
}

/// Create a new ticket or refresh the existing open one for this user/order.
/// Returns `(ticket, created)` where `created` is true when a new row was inserted.
pub async fn ensure_ticket_for_user(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
    label: Option<&str>,
    customer_no: Option<&str>,
    shop_no: Option<&str>,
    order_no: Option<&str>,
    summary_state_code: &str,
    summary_state_text: Option<&str>,
    completed: bool,
) -> Result<(Ticket, bool)> {
    if let Some(existing) = find_open_ticket_for_user_order(pool, user_id, order_number).await? {
        if label.is_some() {
            update_ticket_label(pool, existing.id, label).await?;
        }
        refresh_ticket(
            pool,
            existing.id,
            summary_state_code,
            summary_state_text,
            completed,
        )
        .await?;
        let ticket = find_ticket_by_id(pool, existing.id)
            .await?
            .context("ticket vanished after refresh")?;
        return Ok((ticket, false));
    }

    let ticket = create_ticket(
        pool,
        user_id,
        order_number,
        label,
        customer_no,
        shop_no,
        order_no,
        summary_state_code,
        summary_state_text,
    )
    .await?;

    if completed {
        refresh_ticket(
            pool,
            ticket.id,
            summary_state_code,
            summary_state_text,
            true,
        )
        .await?;
        let ticket = find_ticket_by_id(pool, ticket.id)
            .await?
            .context("ticket vanished after complete")?;
        return Ok((ticket, true));
    }

    Ok((ticket, true))
}

pub async fn find_ticket_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Ticket>> {
    let query = format!("SELECT {TICKET_COLUMNS} FROM tickets WHERE id = ?1");
    let ticket = sqlx::query_as::<_, Ticket>(&query)
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("failed to query ticket by id")?;
    Ok(ticket)
}

pub async fn list_tickets_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Ticket>> {
    let query = format!(
        "SELECT {TICKET_COLUMNS} FROM tickets WHERE user_id = ?1 ORDER BY id DESC"
    );
    let tickets = sqlx::query_as::<_, Ticket>(&query)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("failed to list tickets for user")?;
    Ok(tickets)
}

/// All tickets that have not been marked completed yet, oldest first.
pub async fn list_uncompleted_tickets(pool: &SqlitePool) -> Result<Vec<Ticket>> {
    let query = format!(
        "SELECT {TICKET_COLUMNS} FROM tickets WHERE completed = 0 ORDER BY id ASC"
    );
    let tickets = sqlx::query_as::<_, Ticket>(&query)
    .fetch_all(pool)
    .await
    .context("failed to list uncompleted tickets")?;
    Ok(tickets)
}

/// Record a refresh of a ticket's order state. Always bumps `last_updated`.
/// When `completed` is true the ticket is marked completed and `completed_at`
/// is stamped (only the first time it transitions).
pub async fn refresh_ticket(
    pool: &SqlitePool,
    id: i64,
    summary_state_code: &str,
    summary_state_text: Option<&str>,
    completed: bool,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE tickets SET
            summary_state_code = ?1,
            summary_state_text = ?2,
            completed          = ?3,
            completed_at       = CASE
                                     WHEN ?3 = 1 AND completed_at IS NULL
                                     THEN datetime('now')
                                     ELSE completed_at
                                 END,
            last_updated       = datetime('now')
        WHERE id = ?4
        "#,
    )
    .bind(summary_state_code)
    .bind(summary_state_text)
    .bind(completed)
    .bind(id)
    .execute(pool)
    .await
    .context("failed to refresh ticket")?;
    Ok(result.rows_affected() > 0)
}

/// Delete a ticket only if it belongs to `user_id`. Returns `true` if a row
/// was removed (i.e. it existed and was owned by the user).
pub async fn delete_ticket_for_user(pool: &SqlitePool, id: i64, user_id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM tickets WHERE id = ?1 AND user_id = ?2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to delete ticket for user")?;
    Ok(result.rows_affected() > 0)
}

/// Delete every ticket (admin action). Returns the number of rows removed.
pub async fn delete_all_tickets(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM tickets")
        .execute(pool)
        .await
        .context("failed to delete all tickets")?;
    Ok(result.rows_affected())
}

/// Set or clear a ticket's user-visible label. Returns `true` if updated.
pub async fn update_ticket_label(
    pool: &SqlitePool,
    id: i64,
    label: Option<&str>,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE tickets SET
            label        = ?1,
            last_updated = datetime('now')
        WHERE id = ?2
        "#,
    )
    .bind(label)
    .bind(id)
    .execute(pool)
    .await
    .context("failed to update ticket label")?;
    Ok(result.rows_affected() > 0)
}

/// Rename a ticket owned by `user_id`. Returns `true` if the row existed.
pub async fn rename_ticket_for_user(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
    label: Option<&str>,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE tickets SET
            label        = ?1,
            last_updated = datetime('now')
        WHERE id = ?2 AND user_id = ?3
        "#,
    )
    .bind(label)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .context("failed to rename ticket")?;
    Ok(result.rows_affected() > 0)
}

/// Set a ticket's `completed` flag, stamping `completed_at`/`last_updated`.
/// Returns `true` if a row was updated.
#[allow(dead_code)]
pub async fn set_ticket_completed(pool: &SqlitePool, id: i64, completed: bool) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE tickets SET
            completed    = ?1,
            completed_at = CASE
                               WHEN ?1 = 1 AND completed_at IS NULL
                               THEN datetime('now')
                               ELSE completed_at
                           END,
            last_updated = datetime('now')
        WHERE id = ?2
        "#,
    )
    .bind(completed)
    .bind(id)
    .execute(pool)
    .await
    .context("failed to update ticket completed flag")?;
    Ok(result.rows_affected() > 0)
}

pub async fn create_user_camera(
    pool: &SqlitePool,
    user_id: i64,
    label: &str,
) -> Result<UserCamera> {
    let label = normalize_camera_label(label)?;
    let result = sqlx::query(
        "INSERT INTO user_cameras (user_id, label) VALUES (?1, ?2)",
    )
    .bind(user_id)
    .bind(&label)
    .execute(pool)
    .await;

    if let Err(ref err) = result {
        if is_unique_violation(err) {
            anyhow::bail!("camera label already exists for this user");
        }
    }
    let result = result.context("failed to create user camera")?;

    find_user_camera_by_id(pool, result.last_insert_rowid(), user_id)
        .await?
        .context("user camera vanished after insert")
}

pub async fn list_user_cameras(pool: &SqlitePool, user_id: i64) -> Result<Vec<UserCamera>> {
    let query = format!(
        "SELECT {USER_CAMERA_COLUMNS} FROM user_cameras WHERE user_id = ?1 ORDER BY label ASC"
    );
    let cameras = sqlx::query_as::<_, UserCamera>(&query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("failed to list user cameras")?;
    Ok(cameras)
}

pub async fn find_user_camera_by_id(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<Option<UserCamera>> {
    let query = format!(
        "SELECT {USER_CAMERA_COLUMNS} FROM user_cameras WHERE id = ?1 AND user_id = ?2"
    );
    let camera = sqlx::query_as::<_, UserCamera>(&query)
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("failed to query user camera by id")?;
    Ok(camera)
}

pub async fn delete_user_camera_for_user(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<bool> {
    let result = sqlx::query("DELETE FROM user_cameras WHERE id = ?1 AND user_id = ?2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to delete user camera")?;
    Ok(result.rows_affected() > 0)
}

pub async fn create_user_lens(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    focal_mm: f64,
    aperture: f64,
) -> Result<UserLens> {
    if focal_mm <= 0.0 {
        anyhow::bail!("focal length must be greater than zero");
    }
    if aperture <= 0.0 {
        anyhow::bail!("aperture must be greater than zero");
    }
    let name = normalize_lens_name(name)?;
    let result = sqlx::query(
        "INSERT INTO user_lenses (user_id, name, focal_mm, aperture) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(user_id)
    .bind(&name)
    .bind(focal_mm)
    .bind(aperture)
    .execute(pool)
    .await;

    if let Err(ref err) = result {
        if is_unique_violation(err) {
            anyhow::bail!("lens name already exists for this user");
        }
    }
    let result = result.context("failed to create user lens")?;

    find_user_lens_by_id(pool, result.last_insert_rowid(), user_id)
        .await?
        .context("user lens vanished after insert")
}

pub async fn list_user_lenses(pool: &SqlitePool, user_id: i64) -> Result<Vec<UserLens>> {
    let query = format!(
        "SELECT {USER_LENS_COLUMNS} FROM user_lenses WHERE user_id = ?1 ORDER BY name ASC"
    );
    let lenses = sqlx::query_as::<_, UserLens>(&query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("failed to list user lenses")?;
    Ok(lenses)
}

pub async fn find_user_lens_by_id(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<Option<UserLens>> {
    let query = format!(
        "SELECT {USER_LENS_COLUMNS} FROM user_lenses WHERE id = ?1 AND user_id = ?2"
    );
    let lens = sqlx::query_as::<_, UserLens>(&query)
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("failed to query user lens by id")?;
    Ok(lens)
}

pub async fn delete_user_lens_for_user(pool: &SqlitePool, id: i64, user_id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM user_lenses WHERE id = ?1 AND user_id = ?2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("failed to delete user lens")?;
    Ok(result.rows_affected() > 0)
}

/// Persist camera, lens, and film ISO on a ticket owned by `user_id`.
pub async fn update_ticket_gear_for_user(
    pool: &SqlitePool,
    ticket_id: i64,
    user_id: i64,
    camera_id: Option<i64>,
    lens_id: Option<i64>,
    film_iso: Option<i32>,
) -> Result<bool> {
    if let Some(camera_id) = camera_id {
        if find_user_camera_by_id(pool, camera_id, user_id)
            .await?
            .is_none()
        {
            anyhow::bail!("camera not found for user");
        }
    }
    if let Some(lens_id) = lens_id {
        if find_user_lens_by_id(pool, lens_id, user_id)
            .await?
            .is_none()
        {
            anyhow::bail!("lens not found for user");
        }
    }

    let result = sqlx::query(
        r#"
        UPDATE tickets SET
            camera_id    = ?1,
            lens_id      = ?2,
            film_iso     = ?3,
            last_updated = datetime('now')
        WHERE id = ?4 AND user_id = ?5
        "#,
    )
    .bind(camera_id)
    .bind(lens_id)
    .bind(film_iso)
    .bind(ticket_id)
    .bind(user_id)
    .execute(pool)
    .await
    .context("failed to update ticket gear")?;
    Ok(result.rows_affected() > 0)
}

pub async fn create_analog_ingest_job(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
    secure_id: &str,
    camera_label: &str,
    album: Option<&str>,
    camera_id: Option<i64>,
    lens_id: Option<i64>,
    film_iso: Option<i32>,
) -> Result<AnalogIngestJob> {
    let result = sqlx::query(
        r#"
        INSERT INTO analog_ingest_jobs
            (user_id, order_number, secure_id, camera_label, album, status, camera_id, lens_id, film_iso)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
    )
    .bind(user_id)
    .bind(order_number)
    .bind(secure_id)
    .bind(camera_label)
    .bind(album)
    .bind(ANALOG_INGEST_STATUS_QUEUED)
    .bind(camera_id)
    .bind(lens_id)
    .bind(film_iso)
    .execute(pool)
    .await
    .context("failed to create analog ingest job")?;

    get_analog_ingest_job(pool, result.last_insert_rowid())
        .await?
        .context("analog ingest job vanished after insert")
}

pub async fn get_analog_ingest_job(pool: &SqlitePool, id: i64) -> Result<Option<AnalogIngestJob>> {
    let query = format!(
        "SELECT {ANALOG_INGEST_JOB_COLUMNS} FROM analog_ingest_jobs WHERE id = ?1"
    );
    let job = sqlx::query_as::<_, AnalogIngestJob>(&query)
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("failed to query analog ingest job by id")?;
    Ok(job)
}

pub async fn list_analog_ingest_jobs_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<AnalogIngestJob>> {
    let query = format!(
        "SELECT {ANALOG_INGEST_JOB_COLUMNS} FROM analog_ingest_jobs WHERE user_id = ?1 ORDER BY created_at DESC"
    );
    let jobs = sqlx::query_as::<_, AnalogIngestJob>(&query)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("failed to list analog ingest jobs for user")?;
    Ok(jobs)
}

pub async fn claim_next_queued_analog_ingest_job(pool: &SqlitePool) -> Result<Option<AnalogIngestJob>> {
    let mut tx = pool.begin().await.context("failed to begin claim transaction")?;

    let select_query = format!(
        "SELECT {ANALOG_INGEST_JOB_COLUMNS} FROM analog_ingest_jobs WHERE status = ?1 ORDER BY created_at ASC LIMIT 1"
    );
    let job = sqlx::query_as::<_, AnalogIngestJob>(&select_query)
        .bind(ANALOG_INGEST_STATUS_QUEUED)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to query next queued analog ingest job")?;

    let Some(job) = job else {
        tx.rollback().await.ok();
        return Ok(None);
    };

    let result = sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET status = ?1, updated_at = datetime('now')
        WHERE id = ?2 AND status = ?3
        "#,
    )
    .bind(ANALOG_INGEST_STATUS_DOWNLOADING)
    .bind(job.id)
    .bind(ANALOG_INGEST_STATUS_QUEUED)
    .execute(&mut *tx)
    .await
    .context("failed to claim analog ingest job")?;

    if result.rows_affected() == 0 {
        tx.rollback().await.ok();
        return Ok(None);
    }

    tx.commit()
        .await
        .context("failed to commit analog ingest job claim")?;

    get_analog_ingest_job(pool, job.id).await
}

pub async fn claim_next_labeling_analog_ingest_job(pool: &SqlitePool) -> Result<Option<AnalogIngestJob>> {
    let mut tx = pool.begin().await.context("failed to begin claim transaction")?;

    let select_query = format!(
        "SELECT {ANALOG_INGEST_JOB_COLUMNS} FROM analog_ingest_jobs WHERE status = ?1 ORDER BY created_at ASC LIMIT 1"
    );
    let job = sqlx::query_as::<_, AnalogIngestJob>(&select_query)
        .bind(ANALOG_INGEST_STATUS_LABELING)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to query next labeling analog ingest job")?;

    let Some(job) = job else {
        tx.rollback().await.ok();
        return Ok(None);
    };

    let result = sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET status = ?1, updated_at = datetime('now')
        WHERE id = ?2 AND status = ?3
        "#,
    )
    .bind(ANALOG_INGEST_STATUS_UPLOADING)
    .bind(job.id)
    .bind(ANALOG_INGEST_STATUS_LABELING)
    .execute(&mut *tx)
    .await
    .context("failed to claim labeling analog ingest job")?;

    if result.rows_affected() == 0 {
        tx.rollback().await.ok();
        return Ok(None);
    }

    tx.commit()
        .await
        .context("failed to commit labeling analog ingest job claim")?;

    get_analog_ingest_job(pool, job.id).await
}

pub async fn confirm_analog_ingest_job(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET status = ?1, updated_at = datetime('now')
        WHERE id = ?2 AND user_id = ?3 AND status = ?4
        "#,
    )
    .bind(ANALOG_INGEST_STATUS_LABELING)
    .bind(id)
    .bind(user_id)
    .bind(ANALOG_INGEST_STATUS_PREVIEW)
    .execute(pool)
    .await
    .context("failed to confirm analog ingest job")?;
    Ok(result.rows_affected() > 0)
}

pub async fn cancel_analog_ingest_job(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET status = ?1, secure_id = NULL, error_text = ?2, updated_at = datetime('now')
        WHERE id = ?3 AND user_id = ?4 AND status = ?5
        "#,
    )
    .bind(ANALOG_INGEST_STATUS_FAILED)
    .bind("Abgebrochen")
    .bind(id)
    .bind(user_id)
    .bind(ANALOG_INGEST_STATUS_PREVIEW)
    .execute(pool)
    .await
    .context("failed to cancel analog ingest job")?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_analog_ingest_job_status(
    pool: &SqlitePool,
    id: i64,
    status: &str,
    error_text: Option<&str>,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET status = ?1, error_text = ?2, updated_at = datetime('now')
        WHERE id = ?3
        "#,
    )
    .bind(status)
    .bind(error_text)
    .bind(id)
    .execute(pool)
    .await
    .context("failed to update analog ingest job status")?;
    Ok(result.rows_affected() > 0)
}

pub async fn clear_analog_ingest_secure_id(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET secure_id = NULL, updated_at = datetime('now')
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .context("failed to clear analog ingest job secure_id")?;
    Ok(result.rows_affected() > 0)
}

pub async fn find_done_analog_ingest_job(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
) -> Result<Option<AnalogIngestJob>> {
    let query = format!(
        "SELECT {ANALOG_INGEST_JOB_COLUMNS} FROM analog_ingest_jobs \
         WHERE user_id = ?1 AND order_number = ?2 AND status = ?3 LIMIT 1"
    );
    let job = sqlx::query_as::<_, AnalogIngestJob>(&query)
        .bind(user_id)
        .bind(order_number)
        .bind(ANALOG_INGEST_STATUS_DONE)
        .fetch_optional(pool)
        .await
        .context("failed to query done analog ingest job")?;
    Ok(job)
}

/// Delete an ingest job owned by `user_id` when status allows redo/delete.
pub async fn delete_analog_ingest_job_for_user(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> Result<bool> {
    let result = sqlx::query(
        r#"
        DELETE FROM analog_ingest_jobs
        WHERE id = ?1
          AND user_id = ?2
          AND status IN (?3, ?4, ?5, ?6)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(ANALOG_INGEST_STATUS_QUEUED)
    .bind(ANALOG_INGEST_STATUS_PREVIEW)
    .bind(ANALOG_INGEST_STATUS_DONE)
    .bind(ANALOG_INGEST_STATUS_FAILED)
    .execute(pool)
    .await
    .context("failed to delete analog ingest job")?;
    Ok(result.rows_affected() > 0)
}

/// Alias for UI slice naming (same as `confirm_analog_ingest_job`).
pub async fn confirm_analog_ingest_preview_for_user(
    pool: &SqlitePool,
    job_id: i64,
    user_id: i64,
) -> Result<bool> {
    confirm_analog_ingest_job(pool, job_id, user_id).await
}

/// Alias for UI slice naming (same as `cancel_analog_ingest_job`).
pub async fn cancel_analog_ingest_preview_for_user(
    pool: &SqlitePool,
    job_id: i64,
    user_id: i64,
) -> Result<bool> {
    cancel_analog_ingest_job(pool, job_id, user_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ANALOG_INGEST_STATUS_FAILED, ANALOG_INGEST_STATUS_PREVIEW,
    };

    async fn test_pool() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        let url = format!("sqlite://{}", path.display());
        let pool = init_pool(&url).await.expect("init_pool");
        (dir, pool)
    }

    #[tokio::test]
    async fn migrations_apply_on_fresh_db() {
        let (_dir, pool) = test_pool().await;
        let tables: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'tickets', 'analog_ingest_jobs', 'user_cameras', 'user_lenses')",
        )
        .fetch_one(&pool)
        .await
        .expect("count tables");
        assert_eq!(tables.0, 5);
    }

    #[tokio::test]
    async fn user_camera_lens_crud_and_unique_reject() {
        let (_dir, pool) = test_pool().await;
        let user = upsert_discord_user(&pool, "gear-1", "gear-user")
            .await
            .expect("user");

        let camera = create_user_camera(&pool, user.id, "  Canon AE-1  ")
            .await
            .expect("create camera");
        assert_eq!(camera.label, "Canon AE-1");

        let dup = create_user_camera(&pool, user.id, "canon ae-1").await;
        assert!(dup.is_err());
        assert!(
            dup.unwrap_err()
                .to_string()
                .contains("camera label already exists")
        );

        let cameras = list_user_cameras(&pool, user.id).await.unwrap();
        assert_eq!(cameras.len(), 1);
        assert_eq!(cameras[0].id, camera.id);

        let lens = create_user_lens(&pool, user.id, "  Nifty Fifty  ", 50.0, 1.8)
            .await
            .expect("create lens");
        assert_eq!(lens.name, "Nifty Fifty");

        let dup_lens = create_user_lens(&pool, user.id, "Nifty Fifty", 50.0, 1.8).await;
        assert!(dup_lens.is_err());
        assert!(
            dup_lens
                .unwrap_err()
                .to_string()
                .contains("lens name already exists")
        );

        let lenses = list_user_lenses(&pool, user.id).await.unwrap();
        assert_eq!(lenses.len(), 1);

        assert!(
            delete_user_lens_for_user(&pool, lens.id, user.id)
                .await
                .unwrap()
        );
        assert!(
            delete_user_camera_for_user(&pool, camera.id, user.id)
                .await
                .unwrap()
        );
        assert!(list_user_cameras(&pool, user.id).await.unwrap().is_empty());
        assert!(list_user_lenses(&pool, user.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn update_ticket_gear_persists_and_create_job_with_gear() {
        let (_dir, pool) = test_pool().await;
        let user = upsert_discord_user(&pool, "gear-2", "ticket-gear")
            .await
            .expect("user");

        let camera = create_user_camera(&pool, user.id, "Holga")
            .await
            .expect("camera");
        let lens = create_user_lens(&pool, user.id, "Plastic", 60.0, 8.0)
            .await
            .expect("lens");

        let ticket = create_ticket(
            &pool,
            user.id,
            "544850-103401",
            None,
            None,
            None,
            None,
            "PROCESSING",
            None,
        )
        .await
        .expect("ticket");

        assert!(
            update_ticket_gear_for_user(
                &pool,
                ticket.id,
                user.id,
                Some(camera.id),
                Some(lens.id),
                Some(400),
            )
            .await
            .unwrap()
        );

        let updated = find_ticket_by_id(&pool, ticket.id)
            .await
            .unwrap()
            .expect("ticket");
        assert_eq!(updated.camera_id, Some(camera.id));
        assert_eq!(updated.lens_id, Some(lens.id));
        assert_eq!(updated.film_iso, Some(400));

        let job = create_analog_ingest_job(
            &pool,
            user.id,
            "544850-103401",
            "H5GGX3TB",
            "Holga",
            None,
            Some(camera.id),
            Some(lens.id),
            Some(400),
        )
        .await
        .expect("job with gear");
        assert_eq!(job.camera_id, Some(camera.id));
        assert_eq!(job.lens_id, Some(lens.id));
        assert_eq!(job.film_iso, Some(400));
        assert_eq!(job.camera_label, "Holga");
    }

    #[tokio::test]
    async fn analog_ingest_job_create_find_done_and_clear_secure_id() {
        let (_dir, pool) = test_pool().await;
        let user = upsert_discord_user(&pool, "123", "tester")
            .await
            .expect("user");

        let job = create_analog_ingest_job(
            &pool,
            user.id,
            "544850-103396",
            "H5GGX3T5",
            "Canon AE-1",
            Some("Album"),
            None,
            None,
            None,
        )
        .await
        .expect("create job");
        assert_eq!(job.status, ANALOG_INGEST_STATUS_QUEUED);
        assert_eq!(job.secure_id.as_deref(), Some("H5GGX3T5"));
        assert!(
            find_done_analog_ingest_job(&pool, user.id, "544850-103396")
                .await
                .unwrap()
                .is_none()
        );

        update_analog_ingest_job_status(&pool, job.id, ANALOG_INGEST_STATUS_FAILED, Some("boom"))
            .await
            .unwrap();
        // Failed jobs are not "done" — re-import allowed.
        assert!(
            find_done_analog_ingest_job(&pool, user.id, "544850-103396")
                .await
                .unwrap()
                .is_none()
        );

        update_analog_ingest_job_status(&pool, job.id, ANALOG_INGEST_STATUS_DONE, None)
            .await
            .unwrap();
        clear_analog_ingest_secure_id(&pool, job.id).await.unwrap();

        let done = find_done_analog_ingest_job(&pool, user.id, "544850-103396")
            .await
            .unwrap()
            .expect("done job");
        assert!(done.secure_id.is_none());
        assert_eq!(done.status, ANALOG_INGEST_STATUS_DONE);
    }

    #[tokio::test]
    async fn analog_ingest_preview_confirm_moves_to_labeling() {
        let (_dir, pool) = test_pool().await;
        let user = upsert_discord_user(&pool, "456", "preview-user")
            .await
            .expect("user");

        let job = create_analog_ingest_job(
            &pool,
            user.id,
            "544850-103397",
            "H5GGX3T6",
            "Holga",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create job");

        update_analog_ingest_job_status(&pool, job.id, ANALOG_INGEST_STATUS_PREVIEW, None)
            .await
            .unwrap();

        assert!(
            !confirm_analog_ingest_job(&pool, job.id, user.id + 1)
                .await
                .unwrap()
        );

        assert!(confirm_analog_ingest_job(&pool, job.id, user.id).await.unwrap());

        let updated = get_analog_ingest_job(&pool, job.id)
            .await
            .unwrap()
            .expect("job");
        assert_eq!(updated.status, ANALOG_INGEST_STATUS_LABELING);

        let claimed = claim_next_labeling_analog_ingest_job(&pool)
            .await
            .unwrap()
            .expect("claimed");
        assert_eq!(claimed.id, job.id);
        assert_eq!(claimed.status, ANALOG_INGEST_STATUS_UPLOADING);
    }

    #[tokio::test]
    async fn analog_ingest_cancel_from_preview_marks_failed_and_clears_secure_id() {
        let (_dir, pool) = test_pool().await;
        let user = upsert_discord_user(&pool, "789", "cancel-user")
            .await
            .expect("user");

        let job = create_analog_ingest_job(
            &pool,
            user.id,
            "544850-103398",
            "H5GGX3T7",
            "Pentax",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create job");

        update_analog_ingest_job_status(&pool, job.id, ANALOG_INGEST_STATUS_PREVIEW, None)
            .await
            .unwrap();

        assert!(cancel_analog_ingest_job(&pool, job.id, user.id).await.unwrap());

        let updated = get_analog_ingest_job(&pool, job.id)
            .await
            .unwrap()
            .expect("job");
        assert_eq!(updated.status, ANALOG_INGEST_STATUS_FAILED);
        assert!(updated.secure_id.is_none());
        assert_eq!(updated.error_text.as_deref(), Some("Abgebrochen"));
    }

    #[tokio::test]
    async fn analog_ingest_delete_done_allows_reimport() {
        let (_dir, pool) = test_pool().await;
        let user = upsert_discord_user(&pool, "790", "delete-user")
            .await
            .expect("user");

        let job = create_analog_ingest_job(
            &pool,
            user.id,
            "544850-103399",
            "H5GGX3T8",
            "Nikon FM2",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create job");

        update_analog_ingest_job_status(&pool, job.id, ANALOG_INGEST_STATUS_DONE, None)
            .await
            .unwrap();
        assert!(
            find_done_analog_ingest_job(&pool, user.id, "544850-103399")
                .await
                .unwrap()
                .is_some()
        );

        assert!(
            delete_analog_ingest_job_for_user(&pool, job.id, user.id)
                .await
                .unwrap()
        );
        assert!(
            get_analog_ingest_job(&pool, job.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            find_done_analog_ingest_job(&pool, user.id, "544850-103399")
                .await
                .unwrap()
                .is_none()
        );

        let again = create_analog_ingest_job(
            &pool,
            user.id,
            "544850-103399",
            "H5GGX3T9",
            "Nikon FM2",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("recreate after delete");
        assert_ne!(again.id, job.id);
    }

    #[tokio::test]
    async fn analog_ingest_delete_rejects_uploading() {
        let (_dir, pool) = test_pool().await;
        let user = upsert_discord_user(&pool, "791", "busy-user")
            .await
            .expect("user");

        let job = create_analog_ingest_job(
            &pool,
            user.id,
            "544850-103400",
            "H5GGX3TA",
            "Olympus",
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create job");

        update_analog_ingest_job_status(&pool, job.id, ANALOG_INGEST_STATUS_UPLOADING, None)
            .await
            .unwrap();

        assert!(
            !delete_analog_ingest_job_for_user(&pool, job.id, user.id)
                .await
                .unwrap()
        );
        assert!(
            get_analog_ingest_job(&pool, job.id)
                .await
                .unwrap()
                .is_some()
        );
    }
}
