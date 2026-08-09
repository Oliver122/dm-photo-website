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

pub async fn is_password_admin_session(session: &Session) -> bool {
    session
        .get::<bool>(ADMIN_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Password session flag or Discord allowlist `is_admin`.
pub async fn user_has_admin_access(pool: &SqlitePool, session: &Session, user: &User) -> bool {
    if is_password_admin_session(session).await {
        return true;
    }
    db::is_discord_allowlist_admin(pool, &user.discord_id)
        .await
        .unwrap_or(false)
}

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
            Ok(Some(user)) => {
                // Fail closed: revoked allowlist entries lose the session immediately.
                match db::is_discord_allowlisted(&pool, &user.discord_id).await {
                    Ok(true) => Ok(AuthUser(user)),
                    Ok(false) => {
                        let _ = session.flush().await;
                        Err(AuthRejection::new(parts))
                    }
                    Err(_) => {
                        let _ = session.remove::<i64>(USER_ID_KEY).await;
                        Err(AuthRejection::new(parts))
                    }
                }
            }
            _ => {
                // Stale session id, clear it so the user is forced to log in again.
                let _ = session.remove::<i64>(USER_ID_KEY).await;
                Err(AuthRejection::new(parts))
            }
        }
    }
}

/// Extractor that requires a Discord session **and** admin capability
/// (password-elevated `ADMIN_KEY` or allowlist `is_admin`).
pub struct AdminUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    SqlitePool: FromRef<S>,
{
    type Rejection = AdminRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AdminRejection::unauthenticated(parts))?;

        let user_id: Option<i64> = session.get(USER_ID_KEY).await.unwrap_or(None);
        let Some(user_id) = user_id else {
            return Err(AdminRejection::unauthenticated(parts));
        };

        let pool = SqlitePool::from_ref(state);
        let user = match db::find_by_id(&pool, user_id).await {
            Ok(Some(user)) => user,
            _ => {
                let _ = session.remove::<i64>(USER_ID_KEY).await;
                return Err(AdminRejection::unauthenticated(parts));
            }
        };

        match db::is_discord_allowlisted(&pool, &user.discord_id).await {
            Ok(true) => {}
            Ok(false) => {
                let _ = session.flush().await;
                return Err(AdminRejection::unauthenticated(parts));
            }
            Err(_) => {
                let _ = session.remove::<i64>(USER_ID_KEY).await;
                return Err(AdminRejection::unauthenticated(parts));
            }
        }

        let password_admin = session
            .get::<bool>(ADMIN_KEY)
            .await
            .unwrap_or(None)
            .unwrap_or(false);
        let discord_admin = db::is_discord_allowlist_admin(&pool, &user.discord_id)
            .await
            .unwrap_or(false);

        if password_admin || discord_admin {
            Ok(AdminUser(user))
        } else {
            Err(AdminRejection::forbidden(parts))
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
    /// True when there is no Discord session (send to /login).
    unauthenticated: bool,
}

impl AdminRejection {
    fn unauthenticated(parts: &Parts) -> Self {
        Self {
            htmx: is_htmx_request(parts),
            unauthenticated: true,
        }
    }

    fn forbidden(parts: &Parts) -> Self {
        Self {
            htmx: is_htmx_request(parts),
            unauthenticated: false,
        }
    }
}

impl IntoResponse for AdminRejection {
    fn into_response(self) -> Response {
        if self.htmx {
            let status = if self.unauthenticated {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::FORBIDDEN
            };
            return (status, "admin required").into_response();
        }
        if self.unauthenticated {
            Redirect::to("/login").into_response()
        } else {
            Redirect::to("/admin/login").into_response()
        }
    }
}
