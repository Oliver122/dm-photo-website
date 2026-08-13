use axum::{
    Form, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    auth::session::{AdminUser, AuthUser},
    db,
    discord_bot::{self, BotError},
    dm_order::{self, OrderError},
    handlers::tickets::{render_tickets_list_html, with_tickets_oob},
    jobs,
    state::AppState,
};

/// Minimal HTML escaping for values interpolated into response fragments.
fn esc(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Render the dm order status (a step timeline) as an HTML fragment, mirroring
/// what the public dm site shows.
fn render_order_status(order_number: &str, info: &dm_order::OrderInfo) -> String {
    let mut out = String::new();
    out.push_str(r#"<div class="status-panel">"#);
    out.push_str(r#"<div class="status-panel-head">"#);
    out.push_str(r#"<span class="label">Auftrag</span>"#);
    out.push_str(&format!(r#"<code>{}</code>"#, esc(order_number)));
    out.push_str("</div>");

    if let Some(date) = &info.order_date {
        out.push_str(&format!(
            r#"<p class="muted">Bestellt am {}</p>"#,
            esc(date)
        ));
    }

    let steps = info.timeline();
    if steps.is_empty() {
        let text = info
            .summary_state_text
            .as_deref()
            .unwrap_or(&info.summary_state_code);
        out.push_str(&format!(r#"<p class="ticket-message">{}</p>"#, esc(text)));
    } else {
        out.push_str(r#"<ol class="stepper">"#);
        for step in steps {
            out.push_str(&format!(
                r#"<li class="step {}" data-code="{}">{}</li>"#,
                step.status.css(),
                step.code,
                esc(step.label)
            ));
        }
        out.push_str("</ol>");
    }

    if let Some(updated) = &info.summary_date {
        out.push_str(&format!(
            r#"<p class="ticket-meta-line">Stand: {}</p>"#,
            esc(updated)
        ));
    }

    if info.is_done() {
        out.push_str(
            r#"<p class="status-banner status-banner--ready">Abholbereit — Tracking abgeschlossen.</p>"#,
        );
    } else if info.is_ready() {
        out.push_str(
            r#"<p class="status-banner status-banner--transit">Unterwegs in die Filiale.</p>"#,
        );
    }
    out.push_str("</div>");
    out
}

fn ticket_notice(ticket: &crate::models::Ticket, created: bool) -> String {
    if created {
        format!(
            r#"<p class="notice">Auftrag gespeichert (#{}).</p>"#,
            ticket.id
        )
    } else {
        format!(
            r#"<p class="notice">Auftrag aktualisiert (#{}).</p>"#,
            ticket.id
        )
    }
}

async fn persist_ticket_for_order(
    state: &AppState,
    user_id: i64,
    order_number: &str,
    label: Option<&str>,
    info: &dm_order::OrderInfo,
) -> Result<(crate::models::Ticket, bool), Response> {
    db::ensure_ticket_for_user(
        &state.db,
        user_id,
        order_number,
        label,
        info.customer_no.as_deref(),
        info.shop_no.as_deref(),
        info.order_no.as_deref(),
        &info.summary_state_code,
        info.summary_state_text.as_deref(),
        info.is_done(),
    )
    .await
    .map_err(|err| {
        tracing::error!(?err, "failed to ensure ticket");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(r#"<p class="error">Could not save ticket.</p>"#.to_string()),
        )
            .into_response()
    })
}

pub async fn me(user: AuthUser) -> Json<crate::models::User> {
    Json(user.0)
}

pub async fn list_users(_admin: AdminUser, State(state): State<AppState>) -> Response {
    match db::list_users(&state.db).await {
        Ok(users) => Json(users).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to list users");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "db_error" })),
            )
                .into_response()
        }
    }
}

/// HTMX endpoint that DMs the currently logged-in Discord user. Returns a
/// short HTML fragment intended to be swapped into a result element.
pub async fn dm_me(user: AuthUser, State(state): State<AppState>) -> Response {
    let bot_token = state.config.discord_bot_token.as_deref();
    let message = state.config.dm_message.as_str();

    match discord_bot::send_dm(&state.http, bot_token, &user.0.discord_id, message).await {
        Ok(message_id) => {
            tracing::info!(user_id = user.0.id, %message_id, "discord DM sent");
            Html(format!(
                r#"<p class="success">Sent! Check your Discord DMs (message id <code>{message_id}</code>).</p>"#
            ))
            .into_response()
        }
        Err(err @ BotError::NotConfigured) => {
            tracing::warn!(?err, "DM requested but bot token missing");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(
                    r#"<p class="error">DMs are not configured. The site admin needs to set <code>DISCORD_BOT_TOKEN</code>.</p>"#
                        .to_string(),
                ),
            )
                .into_response()
        }
        Err(err @ BotError::OpenChannel { status: 403, .. }) => {
            tracing::warn!(?err, "discord refused to open DM");
            (
                StatusCode::BAD_GATEWAY,
                Html(
                    r#"<p class="error">Discord refused to open a DM. Make sure you share a server with the bot and that your privacy settings allow DMs from server members.</p>"#
                        .to_string(),
                ),
            )
                .into_response()
        }
        Err(err) => {
            tracing::error!(?err, "DM send failed");
            (
                StatusCode::BAD_GATEWAY,
                Html(format!(r#"<p class="error">Could not send DM: {err}</p>"#)),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OrderCheckForm {
    pub order_number: String,
    pub label: Option<String>,
}

fn normalize_label(label: Option<String>) -> Option<String> {
    label
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// HTMX endpoint: query the dm Foto order-status API for the submitted order
/// number and always create or update a tracking ticket for the current user.
pub async fn check_order(
    user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<OrderCheckForm>,
) -> Response {
    let order_number = form.order_number.trim().to_string();
    let label = normalize_label(form.label);
    let key = state.config.dm_key_account_id.as_str();

    match dm_order::query_order(&state.http, key, &order_number).await {
        Ok(info) => {
            let (ticket, created) = match persist_ticket_for_order(
                &state,
                user.0.id,
                &order_number,
                label.as_deref(),
                &info,
            )
            .await
            {
                Ok(t) => t,
                Err(resp) => return resp,
            };

            tracing::info!(
                user_id = user.0.id,
                ticket_id = ticket.id,
                order = %order_number,
                state = %info.summary_state_code,
                created,
                "ticket ensured from order check"
            );

            let mut html = if info.is_error() {
                let detail = info
                    .summary_state_text
                    .as_deref()
                    .map(esc)
                    .unwrap_or_default();
                format!(
                    r#"<p class="error">Order <code>{}</code> is not initialized yet ({}).</p>"#,
                    esc(&order_number),
                    detail
                )
            } else {
                render_order_status(&order_number, &info)
            };
            html.push_str(&ticket_notice(&ticket, created));
            match with_tickets_oob(&state, user.0.id, html).await {
                Ok(full) => Html(full).into_response(),
                Err(resp) => resp,
            }
        }
        Err(OrderError::InvalidFormat) => (
            StatusCode::BAD_REQUEST,
            Html(
                r#"<p class="error">Invalid order number. Use the 12-digit format like <code>544850-103554</code>.</p>"#
                    .to_string(),
            ),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(?err, "dm order query failed");
            (
                StatusCode::BAD_GATEWAY,
                Html(format!(
                    r#"<p class="error">Could not reach dm order status: {}</p>"#,
                    esc(&err.to_string())
                )),
            )
                .into_response()
        }
    }
}

/// HTMX endpoint: create a tracking ticket from the order number alone, without
/// calling the dm API (manual / offline tracking).
pub async fn create_ticket_manual(
    user: AuthUser,
    State(state): State<AppState>,
    Form(form): Form<OrderCheckForm>,
) -> Response {
    let order_number = form.order_number.trim().to_string();
    let label = normalize_label(form.label);
    if !dm_order::is_valid_order_number(&order_number) {
        return (
            StatusCode::BAD_REQUEST,
            Html(
                r#"<p class="error">Invalid order number. Use the 12-digit format like <code>544850-103554</code>.</p>"#
                    .to_string(),
            ),
        )
            .into_response();
    }

    let pending = dm_order::OrderInfo {
        summary_state_code: "PENDING".to_string(),
        summary_state_text: Some("Ticket created manually; status not checked yet.".to_string()),
        summary_date: None,
        customer_no: None,
        shop_no: None,
        order_no: None,
        order_date: None,
        delivery_type: -1,
    };

    match persist_ticket_for_order(&state, user.0.id, &order_number, label.as_deref(), &pending)
        .await
    {
        Ok((ticket, created)) => {
            let primary = format!(
                r#"<p class="success">Ticket #{} {} for order <code>{}</code>. Use "Check status" to fetch the latest state from dm.</p>"#,
                ticket.id,
                if created { "created" } else { "updated" },
                esc(&order_number)
            );
            match with_tickets_oob(&state, user.0.id, primary).await {
                Ok(full) => Html(full).into_response(),
                Err(resp) => resp,
            }
        }
        Err(resp) => resp,
    }
}

/// Rename one of the current user's tickets (optional label for the order).
pub async fn rename_my_ticket(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<RenameTicketForm>,
) -> Response {
    let label = normalize_label(form.label);
    match db::rename_ticket_for_user(&state.db, id, user.0.id, label.as_deref()).await {
        Ok(true) => match render_tickets_list_html(&state, user.0.id).await {
            Ok(html) => Html(html).into_response(),
            Err(resp) => resp,
        },
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Html(r#"<p class="error">Ticket not found.</p>"#.to_string()),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to rename ticket");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<p class="error">Rename failed.</p>"#.to_string()),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RenameTicketForm {
    pub label: Option<String>,
}

/// Delete one of the current user's own tickets. Ownership is enforced in the
/// query, so a user cannot delete someone else's ticket.
pub async fn delete_my_ticket(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match db::delete_ticket_for_user(&state.db, id, user.0.id).await {
        Ok(true) => match render_tickets_list_html(&state, user.0.id).await {
            Ok(html) => Html(html).into_response(),
            Err(resp) => resp,
        },
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Html(r#"<p class="error">Ticket not found.</p>"#.to_string()),
        )
            .into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to delete ticket");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<p class="error">Delete failed.</p>"#.to_string()),
            )
                .into_response()
        }
    }
}

// Fixed values for the admin "simulate" demo, per request.
const SIM_DISCORD_ID: &str = "299610882034499584";
const SIM_ORDER_NUMBER: &str = "999999-999999";

/// Admin-only: create a simulated ticket for a fixed user, set it to "done",
/// and fire the status-change Discord DM. Used to demo the notification flow.
pub async fn simulate_ticket(_admin: AdminUser, State(state): State<AppState>) -> Response {
    let user = match db::ensure_user(&state.db, SIM_DISCORD_ID, "simulated-user").await {
        Ok(user) => user,
        Err(err) => {
            tracing::error!(?err, "simulate: failed to ensure user");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<p class="error">Could not create simulated user.</p>"#.to_string()),
            )
                .into_response();
        }
    };

    let ticket = match db::create_ticket(
        &state.db,
        user.id,
        SIM_ORDER_NUMBER,
        Some("Simulated order"),
        Some("999999"),
        None,
        Some("999999"),
        "ERROR",
        Some("Auftragsnummer nicht gefunden. (simulated)"),
    )
    .await
    {
        Ok(ticket) => ticket,
        Err(err) => {
            tracing::error!(?err, "simulate: failed to create ticket");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<p class="error">Could not create simulated ticket.</p>"#.to_string()),
            )
                .into_response();
        }
    };

    // Transition the ticket to done (status change: ERROR -> DELIVERED, ready for pickup).
    if let Err(err) = db::refresh_ticket(
        &state.db,
        ticket.id,
        "DELIVERED",
        Some("Dein Auftrag liegt zur Abholung bereit. (simulated)"),
        true,
    )
    .await
    {
        tracing::error!(?err, "simulate: failed to complete ticket");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(r#"<p class="error">Could not complete simulated ticket.</p>"#.to_string()),
        )
            .into_response();
    }

    // Notify the user about the status change (ERROR -> DONE / completed).
    jobs::notify_status_change(
        &state,
        &ticket,
        "ERROR",
        "DELIVERED",
        Some("Dein Auftrag liegt zur Abholung bereit. (simulated)"),
        true,
    )
    .await;

    tracing::info!(
        ticket_id = ticket.id,
        user_id = user.id,
        "simulated ticket completed"
    );
    Html(format!(
        r#"<p class="success">Simulated ticket #{} ({}) for user <code>{}</code> set to done. Status-change DM attempted (needs DISCORD_BOT_TOKEN + shared guild).</p>"#,
        ticket.id, SIM_ORDER_NUMBER, SIM_DISCORD_ID
    ))
    .into_response()
}

/// Admin-only: delete every ticket. Returns an HTMX feedback fragment.
pub async fn delete_all_tickets(_admin: AdminUser, State(state): State<AppState>) -> Response {
    match db::delete_all_tickets(&state.db).await {
        Ok(count) => {
            tracing::info!(count, "admin deleted all tickets");
            Html(format!(
                r#"<p class="success">Deleted {count} ticket(s).</p>"#
            ))
            .into_response()
        }
        Err(err) => {
            tracing::error!(?err, "failed to delete all tickets");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<p class="error">Delete failed.</p>"#.to_string()),
            )
                .into_response()
        }
    }
}

/// Admin-only HTMX endpoint: re-check all uncompleted tickets against the dm
/// order API right now (same logic as the 3-hour background job).
pub async fn refresh_tickets(_admin: AdminUser, State(state): State<AppState>) -> Response {
    match jobs::refresh_open_tickets(&state).await {
        Ok(summary) => {
            tracing::info!(?summary, "admin triggered ticket refresh");
            Html(format!(
                r#"<p class="success">Refreshed {} ticket(s): {} completed, {} failed.</p>"#,
                summary.checked, summary.completed, summary.failed
            ))
            .into_response()
        }
        Err(err) => {
            tracing::error!(?err, "admin ticket refresh failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<p class="error">Refresh failed.</p>"#.to_string()),
            )
                .into_response()
        }
    }
}

pub async fn delete_user(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    match db::delete_user(&state.db, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not_found" }))).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to delete user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "db_error" })),
            )
                .into_response()
        }
    }
}
