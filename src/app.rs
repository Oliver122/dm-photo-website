//! HTTP app assembly — used by `main` and system tests.

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{delete, get, post},
};
use time::Duration;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, SessionManagerLayer, cookie::Key};
use tower_sessions_sqlx_store::SqliteStore;

use crate::{
    auth::discord::oauth_client,
    config::Config,
    db,
    handlers,
    state::AppState,
};

/// Build the full site router (routes + session layer + static files).
pub fn build_router(state: AppState, session_secret: &[u8], static_dir: impl AsRef<std::path::Path>) -> Router {
    let session_store = SqliteStore::new(state.db.clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)))
        .with_signed(Key::from(session_secret));

    Router::new()
        .route("/", get(handlers::pages::index))
        .route("/login", get(handlers::pages::login_page))
        .route("/logout", post(handlers::pages::logout))
        .route("/auth/discord", get(handlers::pages::discord_start))
        .route(
            "/auth/discord/callback",
            get(handlers::pages::discord_callback),
        )
        .route("/admin/login", get(handlers::pages::admin_login_page))
        .route("/admin/login", post(handlers::pages::admin_login_submit))
        .route("/admin/logout", post(handlers::pages::admin_logout))
        .route("/admin", get(handlers::pages::admin_dashboard))
        .route(
            "/admin/tickets/refresh",
            post(handlers::api::refresh_tickets),
        )
        .route(
            "/admin/tickets",
            delete(handlers::api::delete_all_tickets),
        )
        .route(
            "/admin/tickets/simulate",
            post(handlers::api::simulate_ticket),
        )
        .route("/api/me", get(handlers::api::me))
        .route("/api/dm/me", post(handlers::api::dm_me))
        .route("/api/order/check", post(handlers::api::check_order))
        .route("/api/tickets", post(handlers::api::create_ticket_manual))
        .route("/api/tickets/:id", delete(handlers::api::delete_my_ticket))
        .route("/api/tickets/:id/label", post(handlers::api::rename_my_ticket))
        .route(
            "/api/analog/ingest",
            get(handlers::analog_ingest::list_ingest_jobs),
        )
        .route(
            "/api/analog/ingest",
            post(handlers::analog_ingest::create_ingest_job),
        )
        .route(
            "/api/analog/ingest/:id",
            delete(handlers::analog_ingest::delete_ingest_job),
        )
        .route(
            "/api/analog/ingest/:id/preview",
            get(handlers::analog_ingest::preview_gallery),
        )
        .route(
            "/api/analog/ingest/:id/preview/file",
            get(handlers::analog_ingest::preview_image),
        )
        .route(
            "/api/analog/ingest/:id/preview/rotate",
            post(handlers::analog_ingest::preview_rotate),
        )
        .route(
            "/api/analog/ingest/:id/preview/confirm",
            post(handlers::analog_ingest::preview_confirm),
        )
        .route(
            "/api/analog/ingest/:id/preview/cancel",
            post(handlers::analog_ingest::preview_cancel),
        )
        .route("/api/users", get(handlers::api::list_users))
        .route("/api/users/:id", delete(handlers::api::delete_user))
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Migrate session tables then return AppState for the given config + pool.
pub async fn build_state(config: Config, pool: sqlx::SqlitePool) -> Result<AppState> {
    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .context("session store migration failed")?;

    let oauth = oauth_client(&config).context("building discord oauth client")?;
    let http = reqwest::Client::builder()
        .user_agent("dm-photo-website/0.1")
        .build()
        .context("building reqwest client")?;

    Ok(AppState {
        db: pool,
        config: Arc::new(config),
        oauth: Arc::new(oauth),
        http,
    })
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use crate::config::PhotoPrismConfig;
    use std::path::PathBuf;

    pub fn test_config(admin_password: &str, ingest_dir: PathBuf) -> Config {
        Config {
            server_addr: "127.0.0.1:0".into(),
            database_url: "sqlite://:memory:".into(),
            discord_client_id: "test-client-id".into(),
            discord_client_secret: "test-client-secret".into(),
            discord_redirect_uri: "http://localhost:8080/auth/discord/callback".into(),
            discord_bot_token: None,
            dm_message: "test".into(),
            dm_key_account_id: "1320".into(),
            admin_password: admin_password.into(),
            session_secret: b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_vec(),
            photoprism: PhotoPrismConfig {
                base_url: None,
                username: None,
                app_password: None,
                user_uid: None,
                default_album: None,
                verify_tls: true,
            },
            analog_ingest_dir: ingest_dir,
        }
    }

    pub async fn test_app(
        admin_password: &str,
    ) -> (tempfile::TempDir, axum::Router, Config) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("sys.db");
        let ingest = dir.path().join("ingest");
        std::fs::create_dir_all(&ingest).unwrap();

        let mut config = test_config(admin_password, ingest);
        config.database_url = format!("sqlite://{}", db_path.display());

        let pool = db::init_pool(&config.database_url)
            .await
            .expect("init_pool");
        let secret = config.session_secret.clone();
        let state = build_state(config.clone(), pool).await.expect("state");
        let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
        let app = build_router(state, &secret, static_dir);
        (dir, app, config)
    }
}
