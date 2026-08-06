use anyhow::{Context, Result};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

use crate::models::{IngestJob, IngestJobStatus, User};

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

pub async fn find_done_ingest_job(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
) -> Result<Option<IngestJob>> {
    let job = sqlx::query_as::<_, IngestJob>(
        r#"
        SELECT id, user_id, order_number, secure_id, camera_label, album_name,
               status, error_text, created_at, updated_at
        FROM analog_ingest_jobs
        WHERE user_id = ?1 AND order_number = ?2 AND status = 'done'
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(order_number)
    .fetch_optional(pool)
    .await
    .context("failed to query done ingest job")?;
    Ok(job)
}

pub async fn create_ingest_job(
    pool: &SqlitePool,
    user_id: i64,
    order_number: &str,
    secure_id: &str,
    camera_label: &str,
    album_name: Option<&str>,
) -> Result<IngestJob> {
    let result = sqlx::query(
        r#"
        INSERT INTO analog_ingest_jobs (user_id, order_number, secure_id, camera_label, album_name, status)
        VALUES (?1, ?2, ?3, ?4, ?5, 'queued')
        "#,
    )
    .bind(user_id)
    .bind(order_number)
    .bind(secure_id)
    .bind(camera_label)
    .bind(album_name)
    .execute(pool)
    .await
    .context("failed to insert ingest job")?;

    find_ingest_job_by_id(pool, result.last_insert_rowid())
        .await?
        .context("ingest job vanished after insert")
}

pub async fn find_ingest_job_by_id(pool: &SqlitePool, id: i64) -> Result<Option<IngestJob>> {
    let job = sqlx::query_as::<_, IngestJob>(
        r#"
        SELECT id, user_id, order_number, secure_id, camera_label, album_name,
               status, error_text, created_at, updated_at
        FROM analog_ingest_jobs
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("failed to query ingest job by id")?;
    Ok(job)
}

pub async fn list_ingest_jobs_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<IngestJob>> {
    let jobs = sqlx::query_as::<_, IngestJob>(
        r#"
        SELECT id, user_id, order_number, secure_id, camera_label, album_name,
               status, error_text, created_at, updated_at
        FROM analog_ingest_jobs
        WHERE user_id = ?1
        ORDER BY id DESC
        LIMIT 50
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("failed to list ingest jobs")?;
    Ok(jobs)
}

pub async fn claim_next_ingest_job(pool: &SqlitePool) -> Result<Option<IngestJob>> {
    let mut tx = pool.begin().await.context("failed to begin claim tx")?;

    let candidate: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT id FROM analog_ingest_jobs
        WHERE status = 'queued'
        ORDER BY id ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *tx)
    .await
    .context("failed to select queued ingest job")?;

    let Some(job_id) = candidate else {
        tx.commit().await.ok();
        return Ok(None);
    };

    sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET status = 'downloading', updated_at = datetime('now')
        WHERE id = ?1 AND status = 'queued'
        "#,
    )
    .bind(job_id)
    .execute(&mut *tx)
    .await
    .context("failed to claim ingest job")?;

    tx.commit().await.context("failed to commit claim tx")?;

    find_ingest_job_by_id(pool, job_id).await
}

pub async fn set_ingest_job_status(
    pool: &SqlitePool,
    job_id: i64,
    status: IngestJobStatus,
    error_text: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET status = ?1, error_text = ?2, updated_at = datetime('now')
        WHERE id = ?3
        "#,
    )
    .bind(status.as_str())
    .bind(error_text)
    .bind(job_id)
    .execute(pool)
    .await
    .context("failed to update ingest job status")?;
    Ok(())
}

pub async fn clear_ingest_secure_id(pool: &SqlitePool, job_id: i64) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE analog_ingest_jobs
        SET secure_id = NULL, updated_at = datetime('now')
        WHERE id = ?1
        "#,
    )
    .bind(job_id)
    .execute(pool)
    .await
    .context("failed to clear ingest secure id")?;
    Ok(())
}
