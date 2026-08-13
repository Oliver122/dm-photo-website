use std::time::Duration;

use crate::{db, discord_bot, dm_order, state::AppState};

const REFRESH_INTERVAL: Duration = Duration::from_secs(3 * 60 * 60); // 3 hours

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
        let detail = state_text.map(|d| format!(" ({d})")).unwrap_or_default();
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
