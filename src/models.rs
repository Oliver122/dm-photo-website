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
