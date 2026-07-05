mod auth;
mod config;
mod db;
mod discord_bot;
mod dm_order;
mod handlers;
mod jobs;
mod models;
mod state;

use std::{path::PathBuf, sync::Arc};

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
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{auth::discord::oauth_client, config::Config, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")))
        .with(fmt::layer())
        .init();

    let config = Config::from_env().context("loading config")?;
    tracing::info!(addr = %config.server_addr, "starting dm-photo-website");

    let pool = db::init_pool(&config.database_url).await?;

    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .context("session store migration failed")?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // flip to true behind HTTPS
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)))
        .with_signed(Key::from(&config.session_secret));

    let oauth = oauth_client(&config).context("building discord oauth client")?;
    let http = reqwest::Client::builder()
        .user_agent("dm-photo-website/0.1")
        .build()
        .context("building reqwest client")?;

    let state = AppState {
        db: pool,
        config: Arc::new(config.clone()),
        oauth: Arc::new(oauth),
        http,
    };

    // Background task: re-check open tickets every 3 hours, complete them when
    // the dm order becomes available, and DM the owner.
    jobs::spawn_ticket_refresher(state.clone());

    // Resolve static/ from crate root so CSS/JS load even when the binary
    // is started from another working directory.
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    tracing::info!(path = %static_dir.display(), "serving static files");

    let app = Router::new()
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
        .route("/api/users", get(handlers::api::list_users))
        .route("/api/users/:id", delete(handlers::api::delete_user))
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.server_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.server_addr))?;
    tracing::info!("listening on http://{}", config.server_addr);
    axum::serve(listener, app)
        .await
        .context("axum server crashed")?;

    Ok(())
}
