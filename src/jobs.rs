use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;

use crate::{
    camera_exif, db, discord_bot, dm_analog,
    dm_analog::DmAnalogError,
    dm_order,
    models::{
        AnalogIngestJob, ANALOG_INGEST_STATUS_DONE, ANALOG_INGEST_STATUS_FAILED,
        ANALOG_INGEST_STATUS_LABELING, ANALOG_INGEST_STATUS_UPLOADING,
    },
    photoprism::PhotoPrismClient,
    state::AppState,
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(3 * 60 * 60); // 3 hours
const POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub struct RefreshSummary {
    pub checked: usize,
    pub completed: usize,
    pub failed: usize,
}

pub fn spawn_ticket_refresher(state: AppState) {
    tokio::spawn(async move {
        // tokio's interval fires immediately on the first tick, so the first
        // refresh runs shortly after startup, then every REFRESH_INTERVAL.
        let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            ticker.tick().await;
            if let Err(err) = refresh_open_tickets(&state).await {
                tracing::error!(?err, "ticket refresh cycle failed");
            }
        }
    });
}

/// Re-check every uncompleted ticket against the dm order API, persist the new
/// state, complete tickets whose order is now available, and DM their owners.
/// Shared by the 3-hour background job and the admin "refresh now" action.
pub async fn refresh_open_tickets(state: &AppState) -> anyhow::Result<RefreshSummary> {
    let tickets = db::list_uncompleted_tickets(&state.db).await?;
    let mut summary = RefreshSummary::default();
    if tickets.is_empty() {
        return Ok(summary);
    }
    tracing::info!(count = tickets.len(), "refreshing open tickets");

    let key = state.config.dm_key_account_id.as_str();
    for ticket in tickets {
        let old_code = ticket.summary_state_code.clone();
        let info = match dm_order::query_order(&state.http, key, &ticket.order_number).await {
            Ok(info) => info,
            Err(err) => {
                tracing::warn!(ticket_id = ticket.id, ?err, "failed to refresh ticket");
                summary.failed += 1;
                continue;
            }
        };

        // The ticket exists because the order was not initialized (ERROR). It
        // is "done" once the order is ready for pickup (DELIVERED) or later.
        let completed = info.is_done();
        if let Err(err) = db::refresh_ticket(
            &state.db,
            ticket.id,
            &info.summary_state_code,
            info.summary_state_text.as_deref(),
            completed,
        )
        .await
        {
            tracing::error!(ticket_id = ticket.id, ?err, "failed to persist refresh");
            summary.failed += 1;
            continue;
        }

        summary.checked += 1;
        if completed {
            summary.completed += 1;
        }

        // Notify the owner about any status change (including completion).
        if completed || info.summary_state_code != old_code {
            notify_status_change(
                state,
                &ticket,
                &old_code,
                &info.summary_state_code,
                info.summary_state_text.as_deref(),
                completed,
            )
            .await;
        }
    }

    Ok(summary)
}

/// DM a ticket's owner about a status change. When `completed` is true the
/// "order ready" wording is used; otherwise it reports the state transition.
pub async fn notify_status_change(
    state: &AppState,
    ticket: &crate::models::Ticket,
    old_code: &str,
    new_code: &str,
    state_text: Option<&str>,
    completed: bool,
) {
    let user = match db::find_by_id(&state.db, ticket.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            tracing::warn!(ticket_id = ticket.id, "ticket user not found, skip DM");
            return;
        }
        Err(err) => {
            tracing::error!(ticket_id = ticket.id, ?err, "failed to load ticket user");
            return;
        }
    };

    let content = if completed {
        let detail = state_text.unwrap_or("now available");
        format!(
            "Your dm Foto order {} is ready: {}",
            ticket.order_number, detail
        )
    } else {
        let detail = state_text
            .map(|d| format!(" ({d})"))
            .unwrap_or_default();
        format!(
            "Status update for your dm Foto order {}: {} -> {}{}",
            ticket.order_number, old_code, new_code, detail
        )
    };

    match discord_bot::send_dm(
        &state.http,
        state.config.discord_bot_token.as_deref(),
        &user.discord_id,
        &content,
    )
    .await
    {
        Ok(message_id) => tracing::info!(
            ticket_id = ticket.id,
            %message_id,
            "sent status-change DM"
        ),
        Err(err) => tracing::warn!(ticket_id = ticket.id, ?err, "failed to send status DM"),
    }
}

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
        pp.app_password.clone().unwrap(),
        pp.user_uid.clone().unwrap(),
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
