use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;

use crate::{
    camera_exif, db, dm_analog,
    dm_analog::DmAnalogError,
    models::{
        AnalogIngestJob, ANALOG_INGEST_STATUS_DONE, ANALOG_INGEST_STATUS_FAILED,
        ANALOG_INGEST_STATUS_PREVIEW,
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
        if let Err(err) = process_download_job(state, job).await {
            tracing::error!(?err, "analog ingest download job failed unexpectedly");
        }
    }
    while let Some(job) = db::claim_next_labeling_analog_ingest_job(&state.db).await? {
        if let Err(err) = process_labeling_job(state, job).await {
            tracing::error!(?err, "analog ingest labeling job failed unexpectedly");
        }
    }
    Ok(())
}

async fn process_download_job(state: &AppState, job: AnalogIngestJob) -> anyhow::Result<()> {
    let job_id = job.id;
    let work_dir = state.config.analog_ingest_work_dir(job_id);

    let result = process_download_job_inner(state, &job, &work_dir).await;

    if let Err(err) = &result {
        let message = format!("{err:#}");
        tracing::error!(job_id, %message, "analog ingest download job failed");
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
        cleanup_work_dir(&work_dir).await;
    }

    result
}

async fn process_labeling_job(state: &AppState, job: AnalogIngestJob) -> anyhow::Result<()> {
    let job_id = job.id;
    let work_dir = state.config.analog_ingest_work_dir(job_id);

    let result = process_labeling_job_inner(state, &job, &work_dir).await;

    if let Err(err) = &result {
        let message = format!("{err:#}");
        tracing::error!(job_id, %message, "analog ingest labeling job failed");
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

    cleanup_work_dir(&work_dir).await;
    result
}

async fn cleanup_work_dir(work_dir: &std::path::Path) {
    if let Err(err) = tokio::fs::remove_dir_all(work_dir).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(work_dir = %work_dir.display(), ?err, "failed to remove ingest work dir");
        }
    }
}

async fn process_download_job_inner(
    state: &AppState,
    job: &AnalogIngestJob,
    work_dir: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let secure_id = job
        .secure_id
        .as_deref()
        .context("ingest job missing secure_id")?;

    tokio::fs::create_dir_all(work_dir)
        .await
        .with_context(|| format!("failed to create work dir {}", work_dir.display()))?;

    let zip_path = work_dir.join("pack.zip");
    let images_dir = work_dir.join("images");

    dm_analog::fetch_metadata(&state.http, &job.order_number, secure_id)
        .await
        .map_err(map_dm_error)?;
    dm_analog::download_zip(&state.http, &job.order_number, secure_id, &zip_path)
        .await
        .map_err(map_dm_error)?;

    tokio::fs::create_dir_all(&images_dir).await?;
    dm_analog::extract_zip(&zip_path, &images_dir).map_err(map_dm_error)?;

    db::update_analog_ingest_job_status(&state.db, job.id, ANALOG_INGEST_STATUS_PREVIEW, None)
        .await?;

    tracing::info!(
        job_id = job.id,
        order = %job.order_number,
        "analog ingest job ready for preview"
    );
    Ok(())
}

async fn process_labeling_job_inner(
    state: &AppState,
    job: &AnalogIngestJob,
    work_dir: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let pp = &state.config.photoprism;
    if !pp.is_configured() {
        anyhow::bail!("PhotoPrism ist nicht konfiguriert");
    }

    let images_dir = work_dir.join("images");
    let image_paths = list_workdir_images(&images_dir)?;

    let lens = if let Some(lens_id) = job.lens_id {
        db::find_user_lens_by_id(&state.db, lens_id, job.user_id)
            .await?
            .map(|lens| (lens.focal_mm, lens.aperture))
    } else {
        None
    };
    let iso = job.film_iso.map(|value| value as u32);
    let (focal_mm, aperture) = lens
        .map(|(focal_mm, aperture)| (Some(focal_mm), Some(aperture)))
        .unwrap_or((None, None));

    for path in &image_paths {
        camera_exif::stamp_ingest_metadata(path, &job.camera_label, iso, focal_mm, aperture)
            .map_err(|err| anyhow::anyhow!("EXIF stamp failed for {}: {err}", path.display()))?;
    }

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

fn list_workdir_images(images_dir: &std::path::Path) -> Result<Vec<PathBuf>, anyhow::Error> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(images_dir)
        .with_context(|| format!("failed to read images dir {}", images_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(dm_analog::is_image_file_name)
        {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        anyhow::bail!("no preview images found in {}", images_dir.display());
    }
    Ok(paths)
}

fn map_dm_error(err: DmAnalogError) -> anyhow::Error {
    anyhow::anyhow!("{err}")
}
