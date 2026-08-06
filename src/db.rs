use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

use crate::models::{
    AnalogIngestJob, ANALOG_INGEST_STATUS_DOWNLOADING, ANALOG_INGEST_STATUS_QUEUED, User,
};

const ANALOG_INGEST_JOB_COLUMNS: &str = r#"
    id, user_id, order_number, secure_id, camera_label, album, status, error_text, created_at, updated_at
"#;

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

pub async fn create_job(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
    secure_id: &str,
    camera_label: &str,
    album: Option<&str>,
) -> Result<AnalogIngestJob> {
    let result = sqlx::query(
        r#"
        INSERT INTO analog_ingest_jobs (user_id, order_number, secure_id, camera_label, album, status)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
    )
    .bind(user_id)
    .bind(order_number)
    .bind(secure_id)
    .bind(camera_label)
    .bind(album)
    .bind(ANALOG_INGEST_STATUS_QUEUED)
    .execute(pool)
    .await
    .context("failed to create analog ingest job")?;

    get_job(pool, result.last_insert_rowid())
        .await?
        .context("analog ingest job vanished after insert")
}

pub async fn get_job(pool: &SqlitePool, id: i64) -> Result<Option<AnalogIngestJob>> {
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

pub async fn list_jobs_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<AnalogIngestJob>> {
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

pub async fn claim_next_queued_job(pool: &SqlitePool) -> Result<Option<AnalogIngestJob>> {
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

    get_job(pool, job.id).await
}

pub async fn update_job_status(
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

pub async fn clear_secure_id(pool: &SqlitePool, id: i64) -> Result<bool> {
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
