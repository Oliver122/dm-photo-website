use askama::Template;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    auth::session::AuthUser,
    camera_exif::CameraLabel,
    db,
    dm_analog,
    models::IngestJob,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateIngestForm {
    pub order_number: String,
    pub secure_id: String,
    pub camera_label: String,
    pub album_name: Option<String>,
}

#[derive(Template)]
#[template(path = "partials/analog_ingest_list.html")]
struct IngestListTemplate {
    jobs: Vec<IngestJob>,
}

pub async fn create_ingest_job(
    user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<CreateIngestForm>,
) -> Response {
    let order_number = form.order_number.trim().to_string();
    let secure_id = form.secure_id.trim().to_string();
    let camera_label = form.camera_label.trim().to_string();
    let album_name = form
        .album_name
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());

    if !dm_analog::validate_order_number(&order_number) {
        return html_error(
            StatusCode::BAD_REQUEST,
            "Ungültige Auftragsnummer. Format: 123456-123456",
        );
    }

    if !dm_analog::validate_secure_id(&secure_id) {
        return html_error(
            StatusCode::BAD_REQUEST,
            "Ungültige Secure-ID. 8 Zeichen, Großbuchstaben und Ziffern.",
        );
    }

    if let Err(err) = CameraLabel::from_user_label(&camera_label) {
        return html_error(
            StatusCode::BAD_REQUEST,
            &format!("Ungültige Kamera-Bezeichnung: {err}"),
        );
    }

    if !state.config.photoprism.is_configured() {
        return html_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "PhotoPrism ist noch nicht konfiguriert. Bitte den Administrator informieren.",
        );
    }

    match db::find_done_ingest_job(&state.db, user.0.id, &order_number).await {
        Ok(Some(_)) => {
            return html_error(
                StatusCode::CONFLICT,
                "Dieser Auftrag wurde bereits erfolgreich importiert.",
            );
        }
        Ok(None) => {}
        Err(err) => {
            tracing::error!(?err, "failed to check existing ingest job");
            return html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Datenbankfehler. Bitte später erneut versuchen.",
            );
        }
    }

    match db::create_ingest_job(
        &state.db,
        user.0.id,
        &order_number,
        &secure_id,
        &camera_label,
        album_name.as_deref(),
    )
    .await
    {
        Ok(job) => {
            tracing::info!(
                job_id = job.id,
                user_id = user.0.id,
                order = %order_number,
                "analog ingest job queued"
            );
            list_ingest_jobs(user, State(state)).await
        }
        Err(err) => {
            tracing::error!(?err, "failed to create ingest job");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Auftrag konnte nicht angelegt werden.",
            )
        }
    }
}

pub async fn list_ingest_jobs(user: AuthUser, State(state): State<AppState>) -> Response {
    match db::list_ingest_jobs_for_user(&state.db, user.0.id).await {
        Ok(jobs) => IngestListTemplate { jobs }.into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to list ingest jobs");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Importliste konnte nicht geladen werden.",
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
