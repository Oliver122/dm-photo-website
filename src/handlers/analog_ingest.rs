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

#[derive(Debug, Clone)]
pub(crate) struct ResolvedIngestGear {
    pub camera_label: String,
    pub camera_id: Option<i64>,
    pub lens_id: Option<i64>,
    pub film_iso: Option<i32>,
}

pub(crate) fn parse_optional_form_id(raw: Option<String>) -> Option<i64> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.parse().ok()
        }
    })
}

pub(crate) fn parse_optional_film_iso(raw: Option<String>) -> Result<Option<i32>, Response> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let iso = trimmed.parse::<i32>().map_err(|_| {
        html_error(
            StatusCode::BAD_REQUEST,
            "Film-ISO muss eine ganze Zahl sein.",
        )
    })?;
    camera_exif::validate_film_iso(iso as u32).map_err(|err| {
        html_error(
            StatusCode::BAD_REQUEST,
            &format!("Ungültige Film-ISO: {err}"),
        )
    })?;
    Ok(Some(iso))
}

pub(crate) async fn resolve_ingest_gear(
    pool: &sqlx::SqlitePool,
    user_id: i64,
    camera_id: Option<i64>,
    camera_label_input: Option<&str>,
    lens_id: Option<i64>,
    film_iso: Option<i32>,
) -> Result<ResolvedIngestGear, Response> {
    let camera_label = if let Some(camera_id) = camera_id {
        let camera = db::find_user_camera_by_id(pool, camera_id, user_id)
            .await
            .map_err(|err| {
                tracing::error!(?err, camera_id, "failed to load camera for ingest");
                html_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Kamera konnte nicht geladen werden.",
                )
            })?
            .ok_or_else(|| html_error(StatusCode::BAD_REQUEST, "Kamera nicht gefunden."))?;
        camera.label
    } else {
        let label = camera_label_input
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                html_error(StatusCode::BAD_REQUEST, "Kamera-Bezeichnung fehlt.")
            })?;
        camera_exif::label_to_make_model(label).map_err(|err| {
            html_error(
                StatusCode::BAD_REQUEST,
                &format!("Ungültige Kamera-Bezeichnung: {err}"),
            )
        })?;
        label.to_string()
    };

    if let Some(lens_id) = lens_id {
        let lens = db::find_user_lens_by_id(pool, lens_id, user_id)
            .await
            .map_err(|err| {
                tracing::error!(?err, lens_id, "failed to load lens for ingest");
                html_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Objektiv konnte nicht geladen werden.",
                )
            })?;
        if lens.is_none() {
            return Err(html_error(StatusCode::BAD_REQUEST, "Objektiv nicht gefunden."));
        }
    }

    Ok(ResolvedIngestGear {
        camera_label,
        camera_id,
        lens_id,
        film_iso,
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateIngestForm {
    pub order_number: String,
    pub secure_id: String,
    #[serde(default)]
    pub camera_id: Option<String>,
    #[serde(default)]
    pub camera_label: Option<String>,
    #[serde(default)]
    pub lens_id: Option<String>,
    #[serde(default)]
    pub film_iso: Option<String>,
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
    jobs: Vec<IngestJobRow>,
    preview_waiting: usize,
}

pub fn preview_waiting_count(jobs: &[IngestJobRow]) -> usize {
    jobs.iter()
        .filter(|row| row.job.status == ANALOG_INGEST_STATUS_PREVIEW)
        .count()
}

pub struct IngestJobRow {
    pub job: AnalogIngestJob,
    pub gear_line: Option<String>,
}

pub async fn ingest_job_rows(state: &AppState, user_id: i64) -> Vec<IngestJobRow> {
    load_ingest_job_rows(state, user_id)
        .await
        .unwrap_or_default()
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
    let album = form
        .album
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());
    let camera_id = parse_optional_form_id(form.camera_id);
    let lens_id = parse_optional_form_id(form.lens_id);
    let film_iso = match parse_optional_film_iso(form.film_iso) {
        Ok(iso) => iso,
        Err(resp) => return resp,
    };

    if let Err(err) = dm_analog::validate_order_id(&order_number) {
        return html_error(StatusCode::BAD_REQUEST, &err.to_string());
    }

    if let Err(err) = dm_analog::validate_secure_id(&secure_id) {
        return html_error(StatusCode::BAD_REQUEST, &err.to_string());
    }

    let gear = match resolve_ingest_gear(
        &state.db,
        user.0.id,
        camera_id,
        form.camera_label.as_deref(),
        lens_id,
        film_iso,
    )
    .await
    {
        Ok(gear) => gear,
        Err(resp) => return resp,
    };

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
        &gear.camera_label,
        album.as_deref(),
        gear.camera_id,
        gear.lens_id,
        gear.film_iso,
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
            list_ingest_jobs_response(user, State(state)).await
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

pub async fn list_ingest_jobs_response(user: AuthUser, State(state): State<AppState>) -> Response {
    match load_ingest_job_rows(&state, user.0.id).await {
        Ok(jobs) => {
            let preview_waiting = preview_waiting_count(&jobs);
            IngestListTemplate {
                jobs,
                preview_waiting,
            }
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

pub async fn list_ingest_jobs(user: AuthUser, State(state): State<AppState>) -> Response {
    list_ingest_jobs_response(user, State(state)).await
}

async fn load_ingest_job_rows(state: &AppState, user_id: i64) -> anyhow::Result<Vec<IngestJobRow>> {
    let jobs = db::list_analog_ingest_jobs_for_user(&state.db, user_id).await?;
    let lenses = db::list_user_lenses(&state.db, user_id).await?;
    Ok(jobs
        .into_iter()
        .map(|job| {
            let lens = job
                .lens_id
                .and_then(|lens_id| lenses.iter().find(|lens| lens.id == lens_id));
            let gear_line = job.gear_line(lens);
            IngestJobRow { job, gear_line }
        })
        .collect())
}

async fn list_ingest_jobs_clearing_preview(user: AuthUser, state: AppState) -> Response {
    match load_ingest_job_rows(&state, user.0.id).await {
        Ok(jobs) => {
            let preview_waiting = preview_waiting_count(&jobs);
            let list = IngestListTemplate {
                jobs,
                preview_waiting,
            }
            .render()
            .unwrap_or_else(|_| {
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
