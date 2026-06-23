use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde_json::json;

use crate::{
    auth::session::{AdminUser, AuthUser},
    db,
    discord_bot::{self, BotError},
    state::AppState,
};

pub async fn me(user: AuthUser) -> Json<crate::models::User> {
    Json(user.0)
}

pub async fn list_users(_admin: AdminUser, State(state): State<AppState>) -> Response {
    match db::list_users(&state.db).await {
        Ok(users) => Json(users).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to list users");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "db_error" })),
            )
                .into_response()
        }
    }
}

/// HTMX endpoint that DMs the currently logged-in Discord user. Returns a
/// short HTML fragment intended to be swapped into a result element.
pub async fn dm_me(user: AuthUser, State(state): State<AppState>) -> Response {
    let bot_token = state.config.discord_bot_token.as_deref();
    let message = state.config.dm_message.as_str();

    match discord_bot::send_dm(&state.http, bot_token, &user.0.discord_id, message).await {
        Ok(message_id) => {
            tracing::info!(user_id = user.0.id, %message_id, "discord DM sent");
            Html(format!(
                r#"<p class="success">Sent! Check your Discord DMs (message id <code>{message_id}</code>).</p>"#
            ))
            .into_response()
        }
        Err(err @ BotError::NotConfigured) => {
            tracing::warn!(?err, "DM requested but bot token missing");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(
                    r#"<p class="error">DMs are not configured. The site admin needs to set <code>DISCORD_BOT_TOKEN</code>.</p>"#
                        .to_string(),
                ),
            )
                .into_response()
        }
        Err(err @ BotError::OpenChannel { status: 403, .. }) => {
            tracing::warn!(?err, "discord refused to open DM");
            (
                StatusCode::BAD_GATEWAY,
                Html(
                    r#"<p class="error">Discord refused to open a DM. Make sure you share a server with the bot and that your privacy settings allow DMs from server members.</p>"#
                        .to_string(),
                ),
            )
                .into_response()
        }
        Err(err) => {
            tracing::error!(?err, "DM send failed");
            (
                StatusCode::BAD_GATEWAY,
                Html(format!(r#"<p class="error">Could not send DM: {err}</p>"#)),
            )
                .into_response()
        }
    }
}

pub async fn delete_user(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match db::delete_user(&state.db, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not_found" })),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to delete user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "db_error" })),
            )
                .into_response()
        }
    }
}
