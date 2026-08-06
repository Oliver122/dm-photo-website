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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestJobStatus {
    Queued,
    Downloading,
    Labeling,
    Uploading,
    Done,
    Failed,
}

impl IngestJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Downloading => "downloading",
            Self::Labeling => "labeling",
            Self::Uploading => "uploading",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    pub fn label_de(self) -> &'static str {
        match self {
            Self::Queued => "Warteschlange",
            Self::Downloading => "Download läuft",
            Self::Labeling => "Metadaten werden geschrieben",
            Self::Uploading => "Upload zu PhotoPrism",
            Self::Done => "Fertig",
            Self::Failed => "Fehlgeschlagen",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "downloading" => Self::Downloading,
            "labeling" => Self::Labeling,
            "uploading" => Self::Uploading,
            "done" => Self::Done,
            "failed" => Self::Failed,
            _ => Self::Queued,
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct IngestJob {
    pub id: i64,
    pub user_id: i64,
    pub order_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_id: Option<String>,
    pub camera_label: String,
    pub album_name: Option<String>,
    pub status: String,
    pub error_text: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl IngestJob {
    pub fn status_enum(&self) -> IngestJobStatus {
        IngestJobStatus::from_db(&self.status)
    }
}
