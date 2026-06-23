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
