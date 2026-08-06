use askama::Template;
use axum::{
    Form,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use oauth2::CsrfToken;
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::{
        admin::verify_password,
        discord::{build_authorize_url, exchange_and_fetch},
        session::{ADMIN_KEY, AdminUser, AuthUser, OAUTH_STATE_KEY, USER_ID_KEY},
    },
    db,
    models::{AnalogIngestJob, Ticket, User},
    state::AppState,
};

use super::tickets;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    current_user: Option<User>,
    is_admin: bool,
    photoprism_configured: bool,
    tickets: Vec<Ticket>,
    archived_tickets: Vec<Ticket>,
    jobs: Vec<AnalogIngestJob>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    current_user: Option<User>,
    is_admin: bool,
}

#[derive(Template)]
#[template(path = "admin_login.html")]
struct AdminLoginTemplate {
    current_user: Option<User>,
    is_admin: bool,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminTemplate {
    current_user: Option<User>,
    is_admin: bool,
    users: Vec<User>,
}

async fn load_current_user(state: &AppState, session: &Session) -> Option<User> {
    let user_id: Option<i64> = session.get(USER_ID_KEY).await.ok().flatten();
    match user_id {
        Some(id) => db::find_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    }
}

async fn is_admin_session(session: &Session) -> bool {
    session.get::<bool>(ADMIN_KEY).await.ok().flatten().unwrap_or(false)
}

pub async fn index(State(state): State<AppState>, session: Session) -> impl IntoResponse {
    let current_user = load_current_user(&state, &session).await;
    let is_admin = is_admin_session(&session).await;
    let photoprism_configured = state.config.photoprism.is_configured();
    let (archived_tickets, tickets) = match &current_user {
        Some(user) => tickets::load_user_ticket_lists(&state.db, user.id).await,
        None => (Vec::new(), Vec::new()),
    };
    let jobs = match &current_user {
        Some(user) => db::list_analog_ingest_jobs_for_user(&state.db, user.id)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };

    IndexTemplate {
        current_user,
        is_admin,
        photoprism_configured,
        tickets,
        archived_tickets,
        jobs,
    }
}

pub async fn login_page(State(state): State<AppState>, session: Session) -> impl IntoResponse {
    let current_user = load_current_user(&state, &session).await;
    let is_admin = is_admin_session(&session).await;
    LoginTemplate { current_user, is_admin }
}

pub async fn admin_login_page(
    State(state): State<AppState>,
    session: Session,
) -> impl IntoResponse {
    let current_user = load_current_user(&state, &session).await;
    let is_admin = is_admin_session(&session).await;
    AdminLoginTemplate {
        current_user,
        is_admin,
        error: None,
    }
}

#[derive(Debug, Deserialize)]
pub struct AdminLoginForm {
    pub password: String,
}

pub async fn admin_login_submit(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<AdminLoginForm>,
) -> Response {
    if verify_password(&state.config.admin_password, &form.password) {
        if let Err(err) = session.insert(ADMIN_KEY, true).await {
            tracing::error!(?err, "failed to set admin session");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "session error",
            )
                .into_response();
        }
        Redirect::to("/admin").into_response()
    } else {
        let current_user = load_current_user(&state, &session).await;
        let is_admin = is_admin_session(&session).await;
        AdminLoginTemplate {
            current_user,
            is_admin,
            error: Some("Invalid password".to_string()),
        }
        .into_response()
    }
}

pub async fn admin_logout(session: Session) -> Redirect {
    let _ = session.remove::<bool>(ADMIN_KEY).await;
    Redirect::to("/")
}

pub async fn admin_dashboard(
    _admin: AdminUser,
    State(state): State<AppState>,
    session: Session,
) -> Response {
    let current_user = load_current_user(&state, &session).await;
    let users = match db::list_users(&state.db).await {
        Ok(u) => u,
        Err(err) => {
            tracing::error!(?err, "failed to list users");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "could not load users",
            )
                .into_response();
        }
    };
    AdminTemplate {
        current_user,
        is_admin: true,
        users,
    }
    .into_response()
}

pub async fn logout(session: Session) -> Redirect {
    let _ = session.flush().await;
    Redirect::to("/")
}

pub async fn discord_start(State(state): State<AppState>, session: Session) -> Response {
    let (url, csrf) = build_authorize_url(&state.oauth);
    if let Err(err) = session.insert(OAUTH_STATE_KEY, csrf.secret().to_string()).await {
        tracing::error!(?err, "failed to write oauth state");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "session error",
        )
            .into_response();
    }
    Redirect::to(url.as_str()).into_response()
}

#[derive(Debug, Deserialize)]
pub struct DiscordCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub async fn discord_callback(
    State(state): State<AppState>,
    session: Session,
    Query(params): Query<DiscordCallbackParams>,
) -> Response {
    if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or_default();
        tracing::warn!(error = %err, description = %desc, "discord oauth denied");
        return Redirect::to("/login").into_response();
    }

    let Some(code) = params.code else {
        return (axum::http::StatusCode::BAD_REQUEST, "missing code").into_response();
    };
    let Some(received_state) = params.state else {
        return (axum::http::StatusCode::BAD_REQUEST, "missing state").into_response();
    };
    let stored_state: Option<String> = session.remove(OAUTH_STATE_KEY).await.unwrap_or(None);
    let valid_state = stored_state
        .as_deref()
        .map(|s| CsrfToken::new(s.to_string()).secret() == &received_state)
        .unwrap_or(false);
    if !valid_state {
        tracing::warn!("oauth state mismatch");
        return (axum::http::StatusCode::BAD_REQUEST, "state mismatch").into_response();
    }

    let discord_user = match exchange_and_fetch(&state.oauth, code, &state.http).await {
        Ok(u) => u,
        Err(err) => {
            tracing::error!(?err, "discord exchange failed");
            return (
                axum::http::StatusCode::BAD_GATEWAY,
                "could not complete discord login",
            )
                .into_response();
        }
    };

    let user = match db::upsert_discord_user(
        &state.db,
        &discord_user.id,
        discord_user.display_name(),
    )
    .await
    {
        Ok(u) => u,
        Err(err) => {
            tracing::error!(?err, "failed to upsert user");
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    if let Err(err) = session.insert(USER_ID_KEY, user.id).await {
        tracing::error!(?err, "failed to write user session");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "session error",
        )
            .into_response();
    }

    Redirect::to("/").into_response()
}

#[allow(dead_code)]
pub async fn me_page(user: AuthUser) -> Response {
    axum::Json(user.0).into_response()
}
