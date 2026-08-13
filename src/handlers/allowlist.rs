//! Admin Discord allowlist CRUD (REQ-015).

use askama::Template;
use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::{auth::session::AdminUser, db, models::AllowlistMember, state::AppState};

#[derive(Template)]
#[template(path = "partials/allowlist_list.html")]
struct AllowlistListTemplate {
    allowlist: Vec<AllowlistMember>,
}

#[derive(Debug, Deserialize)]
pub struct AddAllowlistForm {
    /// Snowflake ID **or** Discord username handle.
    pub identity: String,
    #[serde(default)]
    pub is_admin: Option<String>,
}

fn html_error_only(status: StatusCode, message: &str) -> Response {
    (
        status,
        Html(format!(
            r#"<div id="allowlist-out"><p class="error">{message}</p></div>"#
        )),
    )
        .into_response()
}

async fn list_with_feedback(
    state: &AppState,
    status: StatusCode,
    kind: &str,
    message: &str,
) -> Response {
    match render_list(state).await {
        Ok(list) => (
            status,
            Html(format!(
                r#"{list}<div id="allowlist-out" hx-swap-oob="innerHTML"><p class="{kind}">{message}</p></div>"#
            )),
        )
            .into_response(),
        Err(_) => html_error_only(status, message),
    }
}

async fn render_list(state: &AppState) -> Result<String, Response> {
    let entries = db::list_allowlist_members(&state.db).await.map_err(|err| {
        tracing::error!(?err, "failed to list allowlist");
        html_error_only(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Allowlist konnte nicht geladen werden.",
        )
    })?;
    AllowlistListTemplate { allowlist: entries }
        .render()
        .map_err(|err| {
            tracing::error!(?err, "failed to render allowlist");
            html_error_only(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Allowlist konnte nicht geladen werden.",
            )
        })
}

pub async fn add_allowlist(
    _admin: AdminUser,
    State(state): State<AppState>,
    Form(form): Form<AddAllowlistForm>,
) -> Response {
    let Some(token) = db::parse_allowlist_token(&form.identity) else {
        return list_with_feedback(
            &state,
            StatusCode::BAD_REQUEST,
            "error",
            "Bitte Discord-Username (z. B. oliver) oder numerische Snowflake-ID eingeben.",
        )
        .await;
    };
    let is_admin = matches!(
        form.is_admin.as_deref().map(str::trim),
        Some("1") | Some("on") | Some("true") | Some("yes")
    );

    if let Err(err) = db::add_allowlist_member(&state.db, token, is_admin, "admin").await {
        tracing::error!(?err, "failed to add allowlist entry");
        return list_with_feedback(
            &state,
            StatusCode::INTERNAL_SERVER_ERROR,
            "error",
            "Eintrag konnte nicht gespeichert werden.",
        )
        .await;
    }

    list_with_feedback(&state, StatusCode::OK, "notice", "Eintrag gespeichert.").await
}

pub async fn toggle_allowlist_admin(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    let members = match db::list_allowlist_members(&state.db).await {
        Ok(m) => m,
        Err(err) => {
            tracing::error!(?err, "failed to read allowlist admin");
            return list_with_feedback(
                &state,
                StatusCode::INTERNAL_SERVER_ERROR,
                "error",
                "Admin-Status konnte nicht geändert werden.",
            )
            .await;
        }
    };
    let Some(member) = members.iter().find(|m| m.key == key) else {
        return list_with_feedback(
            &state,
            StatusCode::NOT_FOUND,
            "error",
            "Eintrag nicht gefunden.",
        )
        .await;
    };
    let is_admin = !member.is_admin;

    match db::set_allowlist_member_admin(&state.db, &key, is_admin).await {
        Ok(true) => {}
        Ok(false) => {
            return list_with_feedback(
                &state,
                StatusCode::NOT_FOUND,
                "error",
                "Eintrag nicht gefunden.",
            )
            .await;
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("last allowlist admin") {
                return list_with_feedback(
                    &state,
                    StatusCode::BAD_REQUEST,
                    "error",
                    "Der letzte Admin kann nicht entfernt werden.",
                )
                .await;
            }
            tracing::error!(?err, "failed to toggle allowlist admin");
            return list_with_feedback(
                &state,
                StatusCode::INTERNAL_SERVER_ERROR,
                "error",
                "Admin-Status konnte nicht geändert werden.",
            )
            .await;
        }
    }

    match render_list(&state).await {
        Ok(list) => Html(list).into_response(),
        Err(resp) => resp,
    }
}

pub async fn delete_allowlist(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    match db::delete_allowlist_member(&state.db, &key).await {
        Ok(true) => {}
        Ok(false) => {
            return list_with_feedback(
                &state,
                StatusCode::NOT_FOUND,
                "error",
                "Eintrag nicht gefunden.",
            )
            .await;
        }
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("last allowlist admin") {
                return list_with_feedback(
                    &state,
                    StatusCode::BAD_REQUEST,
                    "error",
                    "Der letzte Admin kann nicht entfernt werden.",
                )
                .await;
            }
            tracing::error!(?err, "failed to delete allowlist entry");
            return list_with_feedback(
                &state,
                StatusCode::INTERNAL_SERVER_ERROR,
                "error",
                "Eintrag konnte nicht gelöscht werden.",
            )
            .await;
        }
    }

    list_with_feedback(&state, StatusCode::OK, "notice", "Eintrag gelöscht.").await
}
