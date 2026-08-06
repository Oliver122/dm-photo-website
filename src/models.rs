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

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Ticket {
    pub id: i64,
    pub user_id: i64,
    pub order_number: String,
    pub label: Option<String>,
    pub customer_no: Option<String>,
    pub shop_no: Option<String>,
    pub order_no: Option<String>,
    pub summary_state_code: String,
    pub summary_state_text: Option<String>,
    pub status: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Ticket {
    /// `true` when the ticket was completed more than `days` days ago. Used to
    /// move stale completed tickets into the archive/hidden tab.
    pub fn completed_before(&self, days: i64) -> bool {
        match self.completed_at {
            Some(at) => at < Utc::now() - chrono::Duration::days(days),
            None => false,
        }
    }
}

pub const ANALOG_INGEST_STATUS_QUEUED: &str = "queued";
pub const ANALOG_INGEST_STATUS_DOWNLOADING: &str = "downloading";
pub const ANALOG_INGEST_STATUS_PREVIEW: &str = "preview";
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

    pub fn status_label_de(&self) -> &'static str {
        match self.status.as_str() {
            ANALOG_INGEST_STATUS_QUEUED => "Warteschlange",
            ANALOG_INGEST_STATUS_DOWNLOADING => "Download",
            ANALOG_INGEST_STATUS_PREVIEW => "Vorschau",
            ANALOG_INGEST_STATUS_LABELING => "Metadaten",
            ANALOG_INGEST_STATUS_UPLOADING => "Upload",
            ANALOG_INGEST_STATUS_DONE => "Fertig",
            ANALOG_INGEST_STATUS_FAILED => "Fehler",
            _ => "Unbekannt",
        }
    }
}

pub fn is_valid_analog_ingest_status(status: &str) -> bool {
    matches!(
        status,
        ANALOG_INGEST_STATUS_QUEUED
            | ANALOG_INGEST_STATUS_DOWNLOADING
            | ANALOG_INGEST_STATUS_PREVIEW
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ticket(completed_at: Option<DateTime<Utc>>) -> Ticket {
        Ticket {
            id: 1,
            user_id: 1,
            order_number: "544850-103396".into(),
            label: None,
            customer_no: None,
            shop_no: None,
            order_no: None,
            summary_state_code: "DELIVERED".into(),
            summary_state_text: None,
            status: "open".into(),
            completed: completed_at.is_some(),
            created_at: Utc::now(),
            last_updated: None,
            completed_at,
        }
    }

    #[test]
    fn ticket_completed_before_false_when_incomplete() {
        assert!(!sample_ticket(None).completed_before(7));
    }

    #[test]
    fn ticket_completed_before_true_when_older_than_cutoff() {
        let old = Utc::now() - chrono::Duration::days(10);
        assert!(sample_ticket(Some(old)).completed_before(7));
    }

    #[test]
    fn ticket_completed_before_false_when_recent() {
        let recent = Utc::now() - chrono::Duration::days(1);
        assert!(!sample_ticket(Some(recent)).completed_before(7));
    }

    #[test]
    fn analog_status_labels_de() {
        let mut job = AnalogIngestJob {
            id: 1,
            user_id: 1,
            order_number: "544850-103396".into(),
            secure_id: None,
            camera_label: "Holga".into(),
            album: None,
            status: ANALOG_INGEST_STATUS_QUEUED.into(),
            error_text: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(job.status_label_de(), "Warteschlange");
        job.status = ANALOG_INGEST_STATUS_DONE.into();
        assert_eq!(job.status_label_de(), "Fertig");
        job.status = ANALOG_INGEST_STATUS_FAILED.into();
        assert_eq!(job.status_label_de(), "Fehler");
        assert!(job.is_terminal());
        assert!(is_valid_analog_ingest_status(ANALOG_INGEST_STATUS_UPLOADING));
    }
}
