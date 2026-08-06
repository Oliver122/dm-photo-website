use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub discord_id: String,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub last_login: DateTime<Utc>,
}

pub const ANALOG_INGEST_STATUS_QUEUED: &str = "queued";
pub const ANALOG_INGEST_STATUS_DOWNLOADING: &str = "downloading";
pub const ANALOG_INGEST_STATUS_LABELING: &str = "labeling";
pub const ANALOG_INGEST_STATUS_UPLOADING: &str = "uploading";
pub const ANALOG_INGEST_STATUS_DONE: &str = "done";
pub const ANALOG_INGEST_STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AnalogIngestJob {
    pub id: i64,
    pub user_id: i64,
    pub order_number: String,
    pub secure_id: Option<String>,
    pub camera_label: String,
    pub album: Option<String>,
    pub status: String,
    pub error_text: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AnalogIngestJob {
    pub fn is_terminal(&self) -> bool {
        is_terminal_analog_ingest_status(&self.status)
    }
}

pub fn is_valid_analog_ingest_status(status: &str) -> bool {
    matches!(
        status,
        ANALOG_INGEST_STATUS_QUEUED
            | ANALOG_INGEST_STATUS_DOWNLOADING
            | ANALOG_INGEST_STATUS_LABELING
            | ANALOG_INGEST_STATUS_UPLOADING
            | ANALOG_INGEST_STATUS_DONE
            | ANALOG_INGEST_STATUS_FAILED
    )
}

pub fn is_terminal_analog_ingest_status(status: &str) -> bool {
    matches!(
        status,
        ANALOG_INGEST_STATUS_DONE | ANALOG_INGEST_STATUS_FAILED
    )
}
