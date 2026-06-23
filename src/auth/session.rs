use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Redirect, Response},
};
use sqlx::SqlitePool;
use tower_sessions::Session;

use crate::{db, models::User};

pub const USER_ID_KEY: &str = "user_id";
pub const ADMIN_KEY: &str = "is_admin";
pub const OAUTH_STATE_KEY: &str = "oauth_state";

/// Whether the current request appears to come from HTMX. We use this to
/// decide between a 401 fragment and a full-page redirect.
pub fn is_htmx_request(parts: &Parts) -> bool {
    parts
        .headers
        .get("HX-Request")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Extractor that requires a logged-in Discord user.
pub struct AuthUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    SqlitePool: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthRejection::new(parts))?;
        let user_id: Option<i64> = session
            .get(USER_ID_KEY)
            .await
            .map_err(|_| AuthRejection::new(parts))?;
        let Some(user_id) = user_id else {
            return Err(AuthRejection::new(parts));
        };

        let pool = SqlitePool::from_ref(state);
        match db::find_by_id(&pool, user_id).await {
            Ok(Some(user)) => Ok(AuthUser(user)),
            _ => {
                // Stale session id, clear it so the user is forced to log in again.
                let _ = session.remove::<i64>(USER_ID_KEY).await;
                Err(AuthRejection::new(parts))
            }
        }
    }
}

/// Extractor that requires an admin session.
pub struct AdminUser;

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AdminRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AdminRejection::new(parts))?;
        let is_admin: bool = session.get(ADMIN_KEY).await.unwrap_or(None).unwrap_or(false);
        if is_admin {
            Ok(AdminUser)
        } else {
            Err(AdminRejection::new(parts))
        }
    }
}

pub struct AuthRejection {
    htmx: bool,
}

impl AuthRejection {
    fn new(parts: &Parts) -> Self {
        Self {
            htmx: is_htmx_request(parts),
        }
    }
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        if self.htmx {
            (StatusCode::UNAUTHORIZED, "not logged in").into_response()
        } else {
            Redirect::to("/login").into_response()
        }
    }
}

pub struct AdminRejection {
    htmx: bool,
}

impl AdminRejection {
    fn new(parts: &Parts) -> Self {
        Self {
            htmx: is_htmx_request(parts),
        }
    }
}

impl IntoResponse for AdminRejection {
    fn into_response(self) -> Response {
        if self.htmx {
            (StatusCode::FORBIDDEN, "admin required").into_response()
        } else {
            Redirect::to("/admin/login").into_response()
        }
    }
}
