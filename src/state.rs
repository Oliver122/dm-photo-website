use std::sync::Arc;

use axum::extract::FromRef;
use oauth2::basic::BasicClient;
use sqlx::SqlitePool;

use crate::config::Config;
use crate::preview_rotations::PreviewRotationStore;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub oauth: Arc<BasicClient>,
    pub http: reqwest::Client,
    /// Preview rotate offsets (RAM); flushed to JPEG on confirm.
    pub preview_rotations: PreviewRotationStore,
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for Arc<Config> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
