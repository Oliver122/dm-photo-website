use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;

use crate::{
    camera_exif, db, dm_analog,
    dm_analog::DmAnalogError,
    models::{
        AnalogIngestJob, ANALOG_INGEST_STATUS_DONE, ANALOG_INGEST_STATUS_FAILED,
        ANALOG_INGEST_STATUS_LABELING, ANALOG_INGEST_STATUS_UPLOADING,
    },
    photoprism::PhotoPrismClient,
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
    while let Some(job) = db::claim_next_queued_analog_ingest_job(&state.db).await? {
        if let Err(err) = process_ingest_job(state, job).await {
            tracing::error!(?err, "ingest job processing failed unexpectedly");
        }
    }
    Ok(())
}

async fn process_ingest_job(state: &AppState, job: AnalogIngestJob) -> anyhow::Result<()> {
    let job_id = job.id;
    let work_dir = state.config.analog_ingest_dir.join(job_id.to_string());

    let result = process_ingest_job_inner(state, &job, &work_dir).await;

    if let Err(err) = &result {
        let message = format!("{err:#}");
        tracing::error!(job_id, %message, "analog ingest job failed");
        if let Err(db_err) = db::update_analog_ingest_job_status(
            &state.db,
            job_id,
            ANALOG_INGEST_STATUS_FAILED,
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
    job: &AnalogIngestJob,
    work_dir: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let secure_id = job
        .secure_id
        .as_deref()
        .context("ingest job missing secure_id")?;

    let pp = &state.config.photoprism;
    if !pp.is_configured() {
        anyhow::bail!("PhotoPrism ist nicht konfiguriert");
    }

    tokio::fs::create_dir_all(work_dir)
        .await
        .with_context(|| format!("failed to create work dir {}", work_dir.display()))?;

    let zip_path = work_dir.join("pack.zip");
    let images_dir = work_dir.join("images");

    // Refresh expiry via metadata, then download ZIP.
    dm_analog::fetch_metadata(&state.http, &job.order_number, secure_id)
        .await
        .map_err(map_dm_error)?;
    dm_analog::download_zip(&state.http, &job.order_number, secure_id, &zip_path)
        .await
        .map_err(map_dm_error)?;

    db::update_analog_ingest_job_status(&state.db, job.id, ANALOG_INGEST_STATUS_LABELING, None).await?;

    tokio::fs::create_dir_all(&images_dir).await?;
    let image_paths = dm_analog::extract_zip(&zip_path, &images_dir).map_err(map_dm_error)?;

    for path in &image_paths {
        camera_exif::stamp_camera_label(path, &job.camera_label)
            .map_err(|err| anyhow::anyhow!("EXIF stamp failed for {}: {err}", path.display()))?;
    }

    db::update_analog_ingest_job_status(&state.db, job.id, ANALOG_INGEST_STATUS_UPLOADING, None).await?;

    let client = PhotoPrismClient::new(
        pp.base_url.clone().unwrap(),
        pp.username.clone().unwrap(),
        pp.app_password.clone().unwrap(),
        pp.user_uid.clone(),
        pp.verify_tls,
    )
    .context("failed to build PhotoPrism client")?;

    let album = job
        .album
        .as_deref()
        .or(pp.default_album.as_deref());

    let paths: Vec<PathBuf> = image_paths;
    client
        .upload_files(&paths, album)
        .await
        .map_err(|err| anyhow::anyhow!("PhotoPrism upload failed: {err}"))?;

    db::clear_analog_ingest_secure_id(&state.db, job.id).await?;
    db::update_analog_ingest_job_status(&state.db, job.id, ANALOG_INGEST_STATUS_DONE, None).await?;

    tracing::info!(job_id = job.id, order = %job.order_number, "analog ingest job completed");
    Ok(())
}

fn map_dm_error(err: DmAnalogError) -> anyhow::Error {
    anyhow::anyhow!("{err}")
}
