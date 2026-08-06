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
    camera_exif,
    db,
    dm_analog,
    models::AnalogIngestJob,
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
    jobs: Vec<AnalogIngestJob>,
}

pub async fn create_ingest_job(
    user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<CreateIngestForm>,
) -> Response {
    let order_number = form.order_number.trim().to_string();
    let secure_id = form.secure_id.trim().to_string();
    let camera_label = form.camera_label.trim().to_string();
    let album = form
        .album_name
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());

    if let Err(err) = dm_analog::validate_order_id(&order_number) {
        return html_error(StatusCode::BAD_REQUEST, &err.to_string());
    }

    if let Err(err) = dm_analog::validate_secure_id(&secure_id) {
        return html_error(StatusCode::BAD_REQUEST, &err.to_string());
    }

    if let Err(err) = camera_exif::label_to_make_model(&camera_label) {
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

    match db::find_done_job(&state.db, user.0.id, &order_number).await {
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

    match db::create_job(
        &state.db,
        user.0.id,
        &order_number,
        &secure_id,
        &camera_label,
        album.as_deref(),
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
            let message = if err.to_string().contains("UNIQUE") {
                "Für diesen Auftrag gibt es bereits einen Import."
            } else {
                "Auftrag konnte nicht angelegt werden."
            };
            html_error(StatusCode::INTERNAL_SERVER_ERROR, message)
        }
    }
}

pub async fn list_ingest_jobs(user: AuthUser, State(state): State<AppState>) -> Response {
    match db::list_jobs_for_user(&state.db, user.0.id).await {
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
