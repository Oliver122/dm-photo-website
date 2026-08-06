use askama::Template;
use axum::{
    Form,
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    analog_workdir::{
        WorkdirPathError, content_type_for_path, job_work_dir, list_preview_images,
        remove_job_workdir, resolve_workdir_file,
    },
    auth::session::AuthUser,
    camera_exif,
    db,
    dm_analog,
    image_rotate,
    models::{AnalogIngestJob, ANALOG_INGEST_STATUS_PREVIEW},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateIngestForm {
    pub order_number: String,
    pub secure_id: String,
    pub camera_label: String,
    pub album: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RotateForm {
    pub file: String,
    pub direction: String,
}

#[derive(Debug, Deserialize)]
pub struct PreviewFileQuery {
    pub path: String,
    /// Ignored; clients send `t=` for cache-busting after rotate.
    #[serde(default)]
    pub t: Option<String>,
}

#[derive(Template)]
#[template(path = "partials/analog_ingest_list.html")]
struct IngestListTemplate {
    jobs: Vec<AnalogIngestJob>,
}

#[derive(Template)]
#[template(path = "partials/analog_ingest_preview.html")]
struct PreviewTemplate {
    job: AnalogIngestJob,
    images: Vec<PreviewImage>,
    /// Cache-bust query for `<img src>` after rotate.
    cache_bust: i64,
}

struct PreviewImage {
    relative_path: String,
    file_name: String,
    /// CSS degrees for instant preview (disk unchanged until confirm).
    rotate_deg: i32,
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
        .album
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

    match db::find_done_analog_ingest_job(&state.db, user.0.id, &order_number).await {
        Ok(Some(_)) => {
            return html_error(
                StatusCode::CONFLICT,
                "Dieser Auftrag wurde bereits importiert. Lösche den alten Eintrag, um erneut zu importieren.",
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

    match db::create_analog_ingest_job(
        &state.db,
        user.0.id,
        &order_number,
        &secure_id,
        &camera_label,
        album.as_deref(),
        None,
        None,
        None,
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
            if err.to_string().contains("UNIQUE") {
                return html_error(
                    StatusCode::CONFLICT,
                    "Dieser Auftrag wurde bereits erfolgreich importiert.",
                );
            }
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Auftrag konnte nicht angelegt werden.",
            )
        }
    }
}

pub async fn list_ingest_jobs(user: AuthUser, State(state): State<AppState>) -> Response {
    match db::list_analog_ingest_jobs_for_user(&state.db, user.0.id).await {
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

async fn list_ingest_jobs_clearing_preview(user: AuthUser, state: AppState) -> Response {
    match db::list_analog_ingest_jobs_for_user(&state.db, user.0.id).await {
        Ok(jobs) => {
            let list = IngestListTemplate { jobs }.render().unwrap_or_else(|_| {
                r#"<p class="error">Importliste konnte nicht geladen werden.</p>"#.into()
            });
            Html(format!(
                r#"{list}<div id="analog-preview-panel" class="analog-preview-panel" hx-swap-oob="innerHTML"></div>"#
            ))
            .into_response()
        }
        Err(err) => {
            tracing::error!(?err, "failed to list ingest jobs");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Importliste konnte nicht geladen werden.",
            )
        }
    }
}

pub async fn delete_ingest_job(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> Response {
    let existing = match db::get_analog_ingest_job(&state.db, job_id).await {
        Ok(Some(job)) if job.user_id == user.0.id => job,
        Ok(Some(_)) | Ok(None) => {
            return html_error(StatusCode::NOT_FOUND, "Import-Auftrag nicht gefunden.");
        }
        Err(err) => {
            tracing::error!(?err, job_id, "failed to load ingest job for delete");
            return html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Import konnte nicht gelöscht werden.",
            );
        }
    };

    if !existing.can_delete() {
        return html_error(
            StatusCode::CONFLICT,
            "Import läuft noch — bitte warten oder später löschen.",
        );
    }

    match db::delete_analog_ingest_job_for_user(&state.db, job_id, user.0.id).await {
        Ok(true) => {
            let work_dir = job_work_dir(&state.config.analog_ingest_dir, job_id);
            if let Err(err) = remove_job_workdir(&work_dir).await {
                tracing::warn!(job_id, ?err, "failed to remove ingest workdir after delete");
            }
            state.preview_rotations.clear_job(job_id);
            tracing::info!(job_id, user_id = user.0.id, "analog ingest job deleted");
            list_ingest_jobs(user, State(state)).await
        }
        Ok(false) => html_error(
            StatusCode::CONFLICT,
            "Import läuft noch — bitte warten oder später löschen.",
        ),
        Err(err) => {
            tracing::error!(?err, job_id, "failed to delete ingest job");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Import konnte nicht gelöscht werden.",
            )
        }
    }
}

pub async fn preview_gallery(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> Response {
    match load_preview_job(&state, user.0.id, job_id).await {
        Ok(job) => render_preview(&state, job).await,
        Err(resp) => resp,
    }
}

pub async fn preview_image(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
    Query(query): Query<PreviewFileQuery>,
) -> Response {
    let job = match load_preview_job(&state, user.0.id, job_id).await {
        Ok(job) => job,
        Err(resp) => return resp,
    };

    let work_dir = job_work_dir(&state.config.analog_ingest_dir, job.id);
    let file_path = query.path.trim();
    let resolved = match resolve_workdir_file(&work_dir, file_path) {
        Ok(path) => path,
        Err(WorkdirPathError::Traversal) => {
            return html_error(StatusCode::BAD_REQUEST, "Ungültiger Dateipfad.");
        }
        Err(WorkdirPathError::NotFound) => {
            return html_error(StatusCode::NOT_FOUND, "Datei nicht gefunden.");
        }
    };

    if !resolved.is_file() {
        return html_error(StatusCode::NOT_FOUND, "Datei nicht gefunden.");
    }

    let bytes = match tokio::fs::read(&resolved).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!(?err, job_id, path = %resolved.display(), "failed to read preview image");
            return html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Vorschau konnte nicht geladen werden.",
            );
        }
    };

    let content_type = content_type_for_path(&resolved);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type), (header::CACHE_CONTROL, "no-store")],
        Body::from(bytes),
    )
        .into_response()
}

pub async fn preview_rotate(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
    Form(form): Form<RotateForm>,
) -> Response {
    let job = match load_preview_job(&state, user.0.id, job_id).await {
        Ok(job) => job,
        Err(resp) => return resp,
    };

    let work_dir = job_work_dir(&state.config.analog_ingest_dir, job.id);
    let file_path = form.file.trim();
    let resolved = match resolve_workdir_file(&work_dir, file_path) {
        Ok(path) => path,
        Err(WorkdirPathError::Traversal) => {
            return html_error(StatusCode::BAD_REQUEST, "Ungültiger Dateipfad.");
        }
        Err(WorkdirPathError::NotFound) => {
            return html_error(StatusCode::NOT_FOUND, "Datei nicht gefunden.");
        }
    };

    if !resolved.is_file() {
        return html_error(StatusCode::NOT_FOUND, "Datei nicht gefunden.");
    }

    let delta = match form.direction.trim().to_ascii_lowercase().as_str() {
        "cw" => 1i8,
        "ccw" => -1i8,
        _ => return html_error(StatusCode::BAD_REQUEST, "Ungültige Drehrichtung."),
    };

    // RAM only — no JPEG rewrite until confirm.
    state
        .preview_rotations
        .add_quarter(job.id, file_path, delta);

    render_preview(&state, job).await
}

pub async fn preview_confirm(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> Response {
    let job = match load_preview_job(&state, user.0.id, job_id).await {
        Ok(job) => job,
        Err(resp) => return resp,
    };

    let work_dir = job_work_dir(&state.config.analog_ingest_dir, job.id);
    if let Err(err) = flush_preview_rotations_to_disk(&state, job.id, &work_dir) {
        tracing::error!(?err, job_id, "failed to flush preview rotations");
        return html_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Drehung konnte nicht gespeichert werden.",
        );
    }

    match db::confirm_analog_ingest_preview_for_user(&state.db, job_id, user.0.id).await {
        Ok(true) => {
            state.preview_rotations.clear_job(job_id);
            tracing::info!(job_id, user_id = user.0.id, "analog ingest preview confirmed");
            list_ingest_jobs_clearing_preview(user, state).await
        }
        Ok(false) => html_error(
            StatusCode::CONFLICT,
            "Vorschau kann derzeit nicht bestätigt werden.",
        ),
        Err(err) => {
            tracing::error!(?err, job_id, "failed to confirm preview");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Import konnte nicht bestätigt werden.",
            )
        }
    }
}

pub async fn preview_cancel(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> Response {
    match db::cancel_analog_ingest_preview_for_user(&state.db, job_id, user.0.id).await {
        Ok(true) => {
            let work_dir = job_work_dir(&state.config.analog_ingest_dir, job_id);
            if let Err(err) = remove_job_workdir(&work_dir).await {
                tracing::warn!(job_id, ?err, "failed to remove preview workdir after cancel");
            }
            state.preview_rotations.clear_job(job_id);
            tracing::info!(job_id, user_id = user.0.id, "analog ingest preview cancelled");
            list_ingest_jobs_clearing_preview(user, state).await
        }
        Ok(false) => html_error(
            StatusCode::CONFLICT,
            "Vorschau kann derzeit nicht abgebrochen werden.",
        ),
        Err(err) => {
            tracing::error!(?err, job_id, "failed to cancel preview");
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Abbruch fehlgeschlagen.",
            )
        }
    }
}

async fn load_preview_job(
    state: &AppState,
    user_id: i64,
    job_id: i64,
) -> Result<AnalogIngestJob, Response> {
    let job = match db::get_analog_ingest_job(&state.db, job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            return Err(html_error(StatusCode::NOT_FOUND, "Import-Auftrag nicht gefunden."));
        }
        Err(err) => {
            tracing::error!(?err, job_id, "failed to load ingest job");
            return Err(html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Import-Auftrag konnte nicht geladen werden.",
            ));
        }
    };

    if job.user_id != user_id {
        return Err(html_error(StatusCode::NOT_FOUND, "Import-Auftrag nicht gefunden."));
    }

    if job.status != ANALOG_INGEST_STATUS_PREVIEW {
        return Err(html_error(
            StatusCode::CONFLICT,
            "Dieser Auftrag befindet sich nicht in der Vorschau.",
        ));
    }

    Ok(job)
}

async fn render_preview(state: &AppState, job: AnalogIngestJob) -> Response {
    let work_dir = job_work_dir(&state.config.analog_ingest_dir, job.id);
    let relative_paths = match list_preview_images(&work_dir) {
        Ok(paths) => paths,
        Err(err) => {
            tracing::error!(job_id = job.id, ?err, "failed to list preview images");
            return html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Vorschau konnte nicht geladen werden.",
            );
        }
    };

    let images = relative_paths
        .into_iter()
        .map(|relative_path| {
            let file_name = relative_path
                .rsplit('/')
                .next()
                .unwrap_or(&relative_path)
                .to_string();
            let quarters = state
                .preview_rotations
                .get_quarter(job.id, &relative_path);
            PreviewImage {
                relative_path,
                file_name,
                rotate_deg: i32::from(quarters) * 90,
            }
        })
        .collect();

    // Stable cache_bust: originals on disk don't change until confirm.
    let cache_bust = job.id;

    PreviewTemplate {
        job,
        images,
        cache_bust,
    }
    .into_response()
}

fn flush_preview_rotations_to_disk(
    state: &AppState,
    job_id: i64,
    work_dir: &std::path::Path,
) -> Result<(), String> {
    for (relative_path, quarters) in state.preview_rotations.snapshot_job(job_id) {
        if quarters == 0 {
            continue;
        }
        let path = resolve_workdir_file(work_dir, &relative_path).map_err(|e| e.to_string())?;
        for _ in 0..quarters {
            image_rotate::rotate_cw(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn html_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Html(format!(r#"<p class="error">{message}</p>"#)),
    )
        .into_response()
}
