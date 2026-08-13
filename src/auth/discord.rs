use anyhow::{Context, Result, anyhow};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, RequestTokenError,
    Scope, TokenUrl, basic::BasicClient, reqwest::async_http_client,
};
use serde::Deserialize;

use crate::config::Config;

const AUTH_URL: &str = "https://discord.com/api/oauth2/authorize";
const TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const USER_URL: &str = "https://discord.com/api/users/@me";

#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)] // available for future avatar rendering
    pub avatar: Option<String>,
}

impl DiscordUser {
    /// Prefer Discord's `global_name` (the new display name) and fall back to
    /// the legacy `username`.
    pub fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.username)
    }
}

/// Builds a [`BasicClient`] configured against Discord's OAuth endpoints.
pub fn oauth_client(config: &Config) -> Result<BasicClient> {
    let auth_url = AuthUrl::new(AUTH_URL.to_string()).context("invalid auth url")?;
    let token_url = TokenUrl::new(TOKEN_URL.to_string()).context("invalid token url")?;
    let redirect_url =
        RedirectUrl::new(config.discord_redirect_uri.clone()).context("invalid redirect uri")?;

    Ok(BasicClient::new(
        ClientId::new(config.discord_client_id.clone()),
        Some(ClientSecret::new(config.discord_client_secret.clone())),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(redirect_url))
}

/// Build the Discord authorize URL and return it alongside the CSRF token we
/// should stash in the session for verification on the callback.
pub fn build_authorize_url(client: &BasicClient) -> (url::Url, CsrfToken) {
    let (url, csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    (url, csrf)
}

/// Exchange the authorization code we received on the callback for an access
/// token, then call `users/@me` and return the resulting user.
pub async fn exchange_and_fetch(
    client: &BasicClient,
    code: String,
    http: &reqwest::Client,
) -> Result<DiscordUser> {
    use oauth2::TokenResponse;

    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
        .map_err(|err| match err {
            // Surface Discord's actual JSON error payload (e.g. invalid_client,
            // invalid_grant) so misconfigured credentials are obvious.
            RequestTokenError::ServerResponse(resp) => anyhow!(
                "discord rejected token exchange: error={:?}, description={:?}, uri={:?}",
                resp.error(),
                resp.error_description(),
                resp.error_uri(),
            ),
            other => anyhow!(other).context("failed to exchange code for token"),
        })?;

    let access_token = token.access_token().secret();
    let user = http
        .get(USER_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .context("failed to call discord users/@me")?
        .error_for_status()
        .context("discord users/@me returned error")?
        .json::<DiscordUser>()
        .await
        .context("failed to decode discord user")?;

    Ok(user)
}
