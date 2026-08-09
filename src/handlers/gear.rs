use askama::Template;
use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::session::{AuthUser, user_has_admin_access},
    camera_exif::{self, CameraExifError},
    db,
    models::{User, UserCamera, UserLens},
    state::AppState,
};

#[derive(Template)]
#[template(path = "gear.html")]
struct GearTemplate {
    current_user: Option<User>,
    is_admin: bool,
    cameras: Vec<UserCamera>,
    lenses: Vec<UserLens>,
}

#[derive(Template)]
#[template(path = "partials/gear_cameras_list.html")]
struct CamerasListTemplate {
    cameras: Vec<UserCamera>,
}

#[derive(Template)]
#[template(path = "partials/gear_lenses_list.html")]
struct LensesListTemplate {
    lenses: Vec<UserLens>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCameraForm {
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateLensForm {
    pub name: String,
    pub focal_mm: String,
    pub aperture: String,
}

pub async fn gear_page(
    user: AuthUser,
    State(state): State<AppState>,
    session: Session,
) -> impl IntoResponse {
    let cameras = db::list_user_cameras(&state.db, user.0.id)
        .await
        .unwrap_or_default();
    let lenses = db::list_user_lenses(&state.db, user.0.id)
        .await
        .unwrap_or_default();
    let is_admin = user_has_admin_access(&state.db, &session, &user.0).await;

    GearTemplate {
        current_user: Some(user.0),
        is_admin,
        cameras,
        lenses,
    }
}

async fn render_cameras_list_html(user_id: i64, pool: &sqlx::SqlitePool) -> Result<String, Response> {
    let cameras = db::list_user_cameras(pool, user_id).await.map_err(|err| {
        tracing::error!(?err, "failed to list user cameras");
        html_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Kameras konnten nicht geladen werden.",
        )
    })?;
    CamerasListTemplate { cameras }
        .render()
        .map_err(|err| {
            tracing::error!(?err, "failed to render cameras list");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Kameras konnten nicht angezeigt werden.",
            )
        })
}

async fn render_lenses_list_html(user_id: i64, pool: &sqlx::SqlitePool) -> Result<String, Response> {
    let lenses = db::list_user_lenses(pool, user_id).await.map_err(|err| {
        tracing::error!(?err, "failed to list user lenses");
        html_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Objektive konnten nicht geladen werden.",
        )
    })?;
    LensesListTemplate { lenses }
        .render()
        .map_err(|err| {
            tracing::error!(?err, "failed to render lenses list");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Objektive konnten nicht angezeigt werden.",
            )
        })
}

fn with_cameras_oob(list: String, out_html: String) -> String {
    format!(
        r#"{out_html}<div id="gear-cameras-list" hx-swap-oob="innerHTML">{list}</div>"#
    )
}

fn with_lenses_oob(list: String, out_html: String) -> String {
    format!(
        r#"{out_html}<div id="gear-lenses-list" hx-swap-oob="innerHTML">{list}</div>"#
    )
}

fn camera_error_message(err: &anyhow::Error) -> String {
    let msg = err.to_string();
    if msg.contains("camera label already exists") {
        return "Diese Kamera ist bereits gespeichert.".to_string();
    }
    if msg.contains("camera label must not be empty") {
        return "Bitte eine Kamera-Bezeichnung eingeben.".to_string();
    }
    tracing::error!(?err, "unexpected camera create error");
    "Kamera konnte nicht gespeichert werden.".to_string()
}

fn lens_error_message(err: &anyhow::Error) -> String {
    let msg = err.to_string();
    if msg.contains("lens name already exists") {
        return "Dieses Objektiv ist bereits gespeichert.".to_string();
    }
    if msg.contains("lens name must not be empty") {
        return "Bitte einen Namen für das Objektiv eingeben.".to_string();
    }
    if msg.contains("focal length must be greater than zero") {
        return "Brennweite muss größer als 0 sein.".to_string();
    }
    if msg.contains("aperture must be greater than zero") {
        return "Blende muss größer als 0 sein.".to_string();
    }
    tracing::error!(?err, "unexpected lens create error");
    "Objektiv konnte nicht gespeichert werden.".to_string()
}

fn exif_validation_message(err: CameraExifError) -> String {
    match err {
        CameraExifError::InvalidFocalMm { .. } => "Brennweite muss größer als 0 sein.".to_string(),
        CameraExifError::InvalidAperture { .. } => "Blende muss größer als 0 sein.".to_string(),
        other => other.to_string(),
    }
}

fn parse_lens_numbers(focal_raw: &str, aperture_raw: &str) -> Result<(f64, f64), String> {
    let focal_mm = focal_raw
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| "Bitte eine gültige Brennweite in mm eingeben.".to_string())?;
    let aperture = aperture_raw
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| "Bitte eine gültige Blendenzahl eingeben.".to_string())?;

    if let Err(err) = camera_exif::validate_focal_mm(focal_mm) {
        return Err(exif_validation_message(err));
    }
    if let Err(err) = camera_exif::validate_aperture(aperture) {
        return Err(exif_validation_message(err));
    }

    Ok((focal_mm, aperture))
}

pub async fn create_camera(
    user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<CreateCameraForm>,
) -> Response {
    match db::create_user_camera(&state.db, user.0.id, &form.label).await {
        Ok(_) => match render_cameras_list_html(user.0.id, &state.db).await {
            Ok(list) => {
                let out = r#"<div id="gear-cameras-out"><p class="notice">Kamera gespeichert.</p></div>"#;
                Html(with_cameras_oob(list, out.to_string())).into_response()
            }
            Err(resp) => resp,
        }
        Err(err) => {
            let message = camera_error_message(&err);
            Html(format!(
                r#"<div id="gear-cameras-out"><p class="error">{message}</p></div>"#
            ))
                .into_response()
        }
    }
}

pub async fn delete_camera(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match db::delete_user_camera_for_user(&state.db, id, user.0.id).await {
        Ok(true) => match render_cameras_list_html(user.0.id, &state.db).await {
            Ok(list) => Html(list).into_response(),
            Err(resp) => resp,
        }
        Ok(false) => html_error(StatusCode::NOT_FOUND, "Kamera nicht gefunden."),
        Err(err) => {
            tracing::error!(?err, "failed to delete user camera");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Kamera konnte nicht gelöscht werden.",
            )
        }
    }
}

pub async fn create_lens(
    user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<CreateLensForm>,
) -> Response {
    let (focal_mm, aperture) = match parse_lens_numbers(&form.focal_mm, &form.aperture) {
        Ok(v) => v,
        Err(message) => {
            return Html(format!(
                r#"<div id="gear-lenses-out"><p class="error">{message}</p></div>"#
            ))
                .into_response();
        }
    };

    match db::create_user_lens(&state.db, user.0.id, &form.name, focal_mm, aperture).await {
        Ok(_) => match render_lenses_list_html(user.0.id, &state.db).await {
            Ok(list) => {
                let out = r#"<div id="gear-lenses-out"><p class="notice">Objektiv gespeichert.</p></div>"#;
                Html(with_lenses_oob(list, out.to_string())).into_response()
            }
            Err(resp) => resp,
        }
        Err(err) => {
            let message = lens_error_message(&err);
            Html(format!(
                r#"<div id="gear-lenses-out"><p class="error">{message}</p></div>"#
            ))
                .into_response()
        }
    }
}

pub async fn delete_lens(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match db::delete_user_lens_for_user(&state.db, id, user.0.id).await {
        Ok(true) => match render_lenses_list_html(user.0.id, &state.db).await {
            Ok(list) => Html(list).into_response(),
            Err(resp) => resp,
        }
        Ok(false) => html_error(StatusCode::NOT_FOUND, "Objektiv nicht gefunden."),
        Err(err) => {
            tracing::error!(?err, "failed to delete user lens");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Objektiv konnte nicht gelöscht werden.",
            )
        }
    }
}

fn html_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Html(format!(r#"<p class="error">{message}</p>"#)),
    )
        .into_response()
}
