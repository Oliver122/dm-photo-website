use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;

use crate::{
    camera_exif::CameraLabel,
    config::Config,
    db,
    dm_analog::{DmAnalogCredentials, DmAnalogError},
    models::{IngestJob, IngestJobStatus},
    photoprism,
    state::AppState,
};

const POLL_INTERVAL: Duration = Duration::from_secs(30);

pub fn spawn_analog_ingest_worker(state: AppState) {
    tokio::spawn(async move {
        tracing::info!(
            interval_secs = POLL_INTERVAL.as_secs(),
            "analog ingest worker started"
        );
        loop {
            if let Err(err) = run_ingest_cycle(&state).await {
                tracing::error!(?err, "analog ingest cycle failed");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

async fn run_ingest_cycle(state: &AppState) -> anyhow::Result<()> {
    while let Some(job) = db::claim_next_ingest_job(&state.db).await? {
        if let Err(err) = process_ingest_job(state, job).await {
            tracing::error!(?err, "ingest job processing failed unexpectedly");
        }
    }
    Ok(())
}

async fn process_ingest_job(state: &AppState, job: IngestJob) -> anyhow::Result<()> {
    let job_id = job.id;
    let work_dir = state.config.analog_ingest_dir.join(job_id.to_string());

    let result = process_ingest_job_inner(state, &job, &work_dir).await;

    if let Err(err) = &result {
        let message = format!("{err:#}");
        tracing::error!(job_id, %message, "analog ingest job failed");
        if let Err(db_err) = db::set_ingest_job_status(
            &state.db,
            job_id,
            IngestJobStatus::Failed,
            Some(&message),
        )
        .await
        {
            tracing::error!(job_id, ?db_err, "failed to mark ingest job failed");
        }
    }

    if let Err(err) = tokio::fs::remove_dir_all(&work_dir).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(job_id, ?err, "failed to remove ingest work dir");
        }
    }

    result
}

async fn process_ingest_job_inner(
    state: &AppState,
    job: &IngestJob,
    work_dir: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let secure_id = job
        .secure_id
        .as_deref()
        .context("ingest job missing secure_id")?;

    if !state.config.photoprism.is_configured() {
        anyhow::bail!("PhotoPrism ist nicht konfiguriert");
    }

    tokio::fs::create_dir_all(work_dir)
        .await
        .with_context(|| format!("failed to create work dir {}", work_dir.display()))?;

    let zip_path = work_dir.join("pack.zip");
    let images_dir = work_dir.join("images");

    let creds = DmAnalogCredentials {
        order_number: job.order_number.clone(),
        secure_id: secure_id.to_string(),
    };

    crate::dm_analog::download_zip(&state.http, &creds, &zip_path)
        .await
        .map_err(map_dm_error)?;

    db::set_ingest_job_status(&state.db, job.id, IngestJobStatus::Labeling, None).await?;

    tokio::fs::create_dir_all(&images_dir).await?;
    let image_paths = crate::dm_analog::extract_zip(&zip_path, &images_dir).map_err(map_dm_error)?;

    let camera = CameraLabel::from_user_label(&job.camera_label)
        .map_err(|err| anyhow::anyhow!("camera label invalid: {err}"))?;

    for path in &image_paths {
        crate::camera_exif::stamp_camera_metadata(path, &camera)
            .map_err(|err| anyhow::anyhow!("EXIF stamp failed for {}: {err}", path.display()))?;
    }

    db::set_ingest_job_status(&state.db, job.id, IngestJobStatus::Uploading, None).await?;

    let album = job
        .album_name
        .as_deref()
        .or(state.config.photoprism.default_album.as_deref());

    photoprism::upload_and_import(
        &state.http,
        &state.config.photoprism,
        &image_paths,
        album,
    )
    .await
    .map_err(|err| anyhow::anyhow!("PhotoPrism upload failed: {err}"))?;

    db::clear_ingest_secure_id(&state.db, job.id).await?;
    db::set_ingest_job_status(&state.db, job.id, IngestJobStatus::Done, None).await?;

    tracing::info!(job_id = job.id, order = %job.order_number, "analog ingest job completed");
    Ok(())
}

fn map_dm_error(err: DmAnalogError) -> anyhow::Error {
    match err {
        DmAnalogError::NotImplemented => {
            anyhow::anyhow!("dm analog download not implemented yet (todo: merge dm_analog client)")
        }
        DmAnalogError::InvalidOrderNumber => {
            anyhow::anyhow!("Ungültige Auftragsnummer")
        }
        DmAnalogError::InvalidSecureId => anyhow::anyhow!("Ungültige Secure-ID"),
        DmAnalogError::Other(msg) => anyhow::anyhow!(msg),
    }
}

#[allow(dead_code)]
pub fn ingest_work_dir(config: &Arc<Config>, job_id: i64) -> PathBuf {
    config.analog_ingest_dir.join(job_id.to_string())
}
