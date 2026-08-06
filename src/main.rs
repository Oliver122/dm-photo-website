mod analog_workdir;
mod app;
mod auth;
mod camera_exif;
mod image_rotate;
mod config;
mod db;
mod discord_bot;
mod dm_analog;
mod dm_order;
mod handlers;
mod jobs;
mod models;
mod photoprism;
mod state;

#[cfg(test)]
mod system_tests;

use anyhow::{Context, Result};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")))
        .with(fmt::layer())
        .init();

    let config = Config::from_env().context("loading config")?;
    tracing::info!(addr = %config.server_addr, "starting dm-photo-website");

    if let Some(parent) = config.analog_ingest_dir.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }

    let pool = db::init_pool(&config.database_url).await?;
    let session_secret = config.session_secret.clone();
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".into());
    let listen_addr = config.server_addr.clone();

    let state = app::build_state(config, pool).await?;

    crate::jobs::spawn_ticket_refresher(state.clone());
    crate::jobs::spawn_analog_ingest_worker(state.clone());

    let app = app::build_router(state, &session_secret, static_dir);

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .with_context(|| format!("failed to bind {listen_addr}"))?;
    tracing::info!("listening on http://{listen_addr}");
    axum::serve(listener, app)
        .await
        .context("axum server crashed")?;

    Ok(())
}
