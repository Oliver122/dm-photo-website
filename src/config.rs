use anyhow::{Context, Result};
use std::{env, path::PathBuf};

#[derive(Debug, Clone)]
pub struct PhotoPrismConfig {
    pub base_url: Option<String>,
    pub app_password: Option<String>,
    pub user_uid: Option<String>,
    pub default_album: Option<String>,
    pub verify_tls: bool,
}

impl PhotoPrismConfig {
    pub fn is_configured(&self) -> bool {
        self.base_url
            .as_ref()
            .is_some_and(|u| !u.is_empty())
            && self
                .app_password
                .as_ref()
                .is_some_and(|p| !p.is_empty())
            && self.user_uid.as_ref().is_some_and(|u| !u.is_empty())
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_uri: String,
    pub discord_bot_token: Option<String>,
    pub dm_message: String,
    pub admin_password: String,
    pub session_secret: Vec<u8>,
    pub photoprism: PhotoPrismConfig,
    pub analog_ingest_dir: PathBuf,
}

const DEFAULT_DM_MESSAGE: &str =
    "Hello from dm-photo-website! This is a test message triggered from the site.";

impl Config {
    pub fn from_env() -> Result<Self> {
        match dotenvy::dotenv() {
            Ok(path) => tracing::debug!(env_file = %path.display(), "loaded .env"),
            Err(dotenvy::Error::Io(err)) if err.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(".env not found, relying solely on process env");
            }
            Err(err) => {
                return Err(anyhow::Error::new(err))
                    .context("failed to parse .env file (likely a malformed line)");
            }
        }

        let server_addr =
            env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/app.db".to_string());
        let discord_client_id =
            require("DISCORD_CLIENT_ID")?;
        let discord_client_secret =
            require("DISCORD_CLIENT_SECRET")?;
        let discord_redirect_uri = env::var("DISCORD_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:8080/auth/discord/callback".to_string());
        let discord_bot_token = env::var("DISCORD_BOT_TOKEN")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let dm_message = env::var("DM_MESSAGE")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_DM_MESSAGE.to_string());

        let admin_password = require("ADMIN_PASSWORD")?;

        let session_secret_raw = require("SESSION_SECRET")?;
        if session_secret_raw.len() < 64 {
            anyhow::bail!(
                "SESSION_SECRET must be at least 64 bytes long (got {} bytes). \
                Generate one with: openssl rand -base64 64",
                session_secret_raw.len()
            );
        }

        let photoprism = PhotoPrismConfig {
            base_url: optional_env("PHOTOPRISM_BASE_URL"),
            app_password: optional_env("PHOTOPRISM_APP_PASSWORD"),
            user_uid: optional_env("PHOTOPRISM_USER_UID"),
            default_album: optional_env("PHOTOPRISM_DEFAULT_ALBUM"),
            verify_tls: env::var("PHOTOPRISM_VERIFY_TLS")
                .ok()
                .map(|v| !matches!(v.trim(), "0" | "false" | "no"))
                .unwrap_or(true),
        };
        let analog_ingest_dir = env::var("ANALOG_INGEST_DIR")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data/ingest"));

        Ok(Self {
            server_addr,
            database_url,
            discord_client_id,
            discord_client_secret,
            discord_redirect_uri,
            discord_bot_token,
            dm_message,
            admin_password,
            session_secret: session_secret_raw.into_bytes(),
            photoprism,
            analog_ingest_dir,
        })
    }
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn require(key: &str) -> Result<String> {
    let value = env::var(key).with_context(|| format!("missing required env var {key}"))?;
    if value.trim().is_empty() {
        anyhow::bail!("env var {key} must not be empty");
    }
    Ok(value)
}
