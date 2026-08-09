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
pub struct UserCamera {
    pub id: i64,
    pub user_id: i64,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserLens {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub focal_mm: f64,
    pub aperture: f64,
    pub created_at: DateTime<Utc>,
}

impl UserLens {
    /// Compact display for lists, e.g. `50 mm · f/2.4`.
    pub fn spec_label(&self) -> String {
        format!("{} mm · f/{}", format_lens_number(self.focal_mm), format_lens_number(self.aperture))
    }
}

fn format_lens_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{:.0}", value)
    } else {
        let s = format!("{:.1}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
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
    pub camera_id: Option<i64>,
    pub lens_id: Option<i64>,
    pub film_iso: Option<i32>,
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

    pub fn has_camera(&self, camera_id: &i64) -> bool {
        self.camera_id == Some(*camera_id)
    }

    pub fn has_lens(&self, lens_id: &i64) -> bool {
        self.lens_id == Some(*lens_id)
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
    pub camera_id: Option<i64>,
    pub lens_id: Option<i64>,
    pub film_iso: Option<i32>,
}

fn format_lens_focal_mm(focal_mm: f64) -> String {
    if focal_mm.fract().abs() < f64::EPSILON {
        format!("{:.0}", focal_mm)
    } else {
        format!("{focal_mm}")
    }
}

fn format_lens_aperture(aperture: f64) -> String {
    if (aperture * 10.0).fract().abs() < f64::EPSILON {
        format!("{:.1}", aperture)
    } else {
        format!("{aperture}")
    }
}

impl AnalogIngestJob {
    /// Compact gear summary for job lists, e.g. `Canon AE-1 · ISO 400 · 50mm f/2.4`.
    pub fn gear_line(&self, lens: Option<&UserLens>) -> Option<String> {
        if self.film_iso.is_none() && lens.is_none() {
            return None;
        }

        let mut parts = vec![self.camera_label.clone()];
        if let Some(iso) = self.film_iso {
            parts.push(format!("ISO {iso}"));
        }
        if let Some(lens) = lens {
            parts.push(format!(
                "{}mm f/{}",
                format_lens_focal_mm(lens.focal_mm),
                format_lens_aperture(lens.aperture)
            ));
        }
        Some(parts.join(" · "))
    }

    pub fn is_terminal(&self) -> bool {
        is_terminal_analog_ingest_status(&self.status)
    }

    /// Jobs that are safe to remove so the order can be imported again.
    /// Active download/label/upload steps are blocked to avoid racing the worker.
    pub fn can_delete(&self) -> bool {
        matches!(
            self.status.as_str(),
            ANALOG_INGEST_STATUS_QUEUED
                | ANALOG_INGEST_STATUS_PREVIEW
                | ANALOG_INGEST_STATUS_DONE
                | ANALOG_INGEST_STATUS_FAILED
        )
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

    /// Rank for the ingest UI stepper (0..=5). Failed shares the terminal slot with done.
    pub fn step_rank(&self) -> u8 {
        match self.status.as_str() {
            ANALOG_INGEST_STATUS_QUEUED => 0,
            ANALOG_INGEST_STATUS_DOWNLOADING => 1,
            ANALOG_INGEST_STATUS_PREVIEW => 2,
            ANALOG_INGEST_STATUS_LABELING => 3,
            ANALOG_INGEST_STATUS_UPLOADING => 4,
            ANALOG_INGEST_STATUS_DONE | ANALOG_INGEST_STATUS_FAILED => 5,
            _ => 0,
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
            camera_id: None,
            lens_id: None,
            film_iso: None,
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
            camera_id: None,
            lens_id: None,
            film_iso: None,
        };
        assert_eq!(job.status_label_de(), "Warteschlange");
        job.status = ANALOG_INGEST_STATUS_DONE.into();
        assert_eq!(job.status_label_de(), "Fertig");
        job.status = ANALOG_INGEST_STATUS_FAILED.into();
        assert_eq!(job.status_label_de(), "Fehler");
        assert!(job.is_terminal());
        assert!(is_valid_analog_ingest_status(ANALOG_INGEST_STATUS_UPLOADING));
        job.status = ANALOG_INGEST_STATUS_PREVIEW.into();
        assert_eq!(job.status_label_de(), "Vorschau");
        assert!(!job.is_terminal());
        assert!(is_valid_analog_ingest_status(ANALOG_INGEST_STATUS_PREVIEW));
    }

    #[test]
    fn analog_job_gear_line_formats_iso_and_lens() {
        let lens = UserLens {
            id: 1,
            user_id: 1,
            name: "Nifty".into(),
            focal_mm: 50.0,
            aperture: 2.4,
            created_at: Utc::now(),
        };
        let job = AnalogIngestJob {
            id: 1,
            user_id: 1,
            order_number: "544850-103396".into(),
            secure_id: None,
            camera_label: "Canon AE-1".into(),
            album: None,
            status: ANALOG_INGEST_STATUS_QUEUED.into(),
            error_text: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            camera_id: Some(1),
            lens_id: Some(1),
            film_iso: Some(400),
        };
        assert_eq!(
            job.gear_line(Some(&lens)).as_deref(),
            Some("Canon AE-1 · ISO 400 · 50mm f/2.4")
        );
        assert_eq!(
            job.gear_line(None).as_deref(),
            Some("Canon AE-1 · ISO 400")
        );
        assert!(AnalogIngestJob {
            film_iso: None,
            lens_id: None,
            ..job
        }
        .gear_line(None)
        .is_none());
    }
}
