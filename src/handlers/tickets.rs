use askama::Template;
use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    auth::session::AuthUser,
    db,
    handlers::analog_ingest::{self, list_ingest_jobs_response},
    models::{Ticket, UserCamera, UserLens},
    state::AppState,
};

#[derive(Template)]
#[template(path = "partials/tickets_list.html")]
pub struct TicketsListTemplate {
    pub tickets: Vec<Ticket>,
    pub archived_tickets: Vec<Ticket>,
    pub cameras: Vec<UserCamera>,
    pub lenses: Vec<UserLens>,
    pub photoprism_configured: bool,
}

#[derive(Debug, Deserialize)]
pub struct TicketGearForm {
    pub camera_id: Option<String>,
    pub lens_id: Option<String>,
    pub film_iso: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TicketConvertForm {
    pub secure_id: String,
    pub album: Option<String>,
    pub camera_id: Option<String>,
    pub camera_label: Option<String>,
    pub lens_id: Option<String>,
    pub film_iso: Option<String>,
}

pub async fn load_user_ticket_lists(
    pool: &SqlitePool,
    user_id: i64,
) -> (Vec<Ticket>, Vec<Ticket>) {
    let all = db::list_tickets_for_user(pool, user_id)
        .await
        .unwrap_or_default();
    all.into_iter()
        .partition(|t| t.completed_before(7))
}

async fn load_ticket_list_context(
    state: &AppState,
    user_id: i64,
) -> Result<TicketsListTemplate, Response> {
    let (tickets, archived_tickets) = load_user_ticket_lists(&state.db, user_id).await;
    let cameras = db::list_user_cameras(&state.db, user_id)
        .await
        .unwrap_or_default();
    let lenses = db::list_user_lenses(&state.db, user_id)
        .await
        .unwrap_or_default();
    Ok(TicketsListTemplate {
        tickets,
        archived_tickets,
        cameras,
        lenses,
        photoprism_configured: state.config.photoprism.is_configured(),
    })
}

pub async fn render_tickets_list_html(state: &AppState, user_id: i64) -> Result<String, Response> {
    load_ticket_list_context(state, user_id)
        .await?
        .render()
        .map_err(|err| {
            tracing::error!(?err, "failed to render tickets list");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<p class="error">Could not load tickets.</p>"#.to_string()),
            )
                .into_response()
        })
}

/// Wrap primary HTMX fragment with an out-of-band swap that refreshes `#tickets-list`.
pub async fn with_tickets_oob(
    state: &AppState,
    user_id: i64,
    primary: String,
) -> Result<String, Response> {
    let list = render_tickets_list_html(state, user_id).await?;
    Ok(format!(
        r#"{primary}<div id="tickets-list" hx-swap-oob="innerHTML">{list}</div>"#
    ))
}

pub async fn save_ticket_gear(
    user: AuthUser,
    State(state): State<AppState>,
    Path(ticket_id): Path<i64>,
    Form(form): Form<TicketGearForm>,
) -> Response {
    let camera_id = analog_ingest::parse_optional_form_id(form.camera_id);
    let lens_id = analog_ingest::parse_optional_form_id(form.lens_id);
    let film_iso = match analog_ingest::parse_optional_film_iso(form.film_iso) {
        Ok(iso) => iso,
        Err(resp) => return resp,
    };

    match db::update_ticket_gear_for_user(
        &state.db,
        ticket_id,
        user.0.id,
        camera_id,
        lens_id,
        film_iso,
    )
    .await
    {
        Ok(true) => match render_tickets_list_html(&state, user.0.id).await {
            Ok(html) => Html(html).into_response(),
            Err(resp) => resp,
        },
        Ok(false) => html_error(StatusCode::NOT_FOUND, "Auftrag nicht gefunden."),
        Err(err) => {
            tracing::error!(?err, ticket_id, "failed to save ticket gear");
            let message = if err.to_string().contains("not found") {
                "Kamera oder Objektiv nicht gefunden."
            } else {
                "Ausrüstung konnte nicht gespeichert werden."
            };
            html_error(StatusCode::BAD_REQUEST, message)
        }
    }
}

pub async fn convert_ticket_to_ingest(
    user: AuthUser,
    State(state): State<AppState>,
    Path(ticket_id): Path<i64>,
    Form(form): Form<TicketConvertForm>,
) -> Response {
    let secure_id = form.secure_id.trim().to_string();
    if let Err(err) = crate::dm_analog::validate_secure_id(&secure_id) {
        return html_error(StatusCode::BAD_REQUEST, &err.to_string());
    }

    let ticket = match db::find_ticket_by_id(&state.db, ticket_id).await {
        Ok(Some(ticket)) if ticket.user_id == user.0.id => ticket,
        Ok(Some(_)) | Ok(None) => {
            return html_error(StatusCode::NOT_FOUND, "Auftrag nicht gefunden.");
        }
        Err(err) => {
            tracing::error!(?err, ticket_id, "failed to load ticket for convert");
            return html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Auftrag konnte nicht geladen werden.",
            );
        }
    };

    let album = form
        .album
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty());
    let camera_id = analog_ingest::parse_optional_form_id(form.camera_id);
    let lens_id = analog_ingest::parse_optional_form_id(form.lens_id);
    let film_iso = match analog_ingest::parse_optional_film_iso(form.film_iso) {
        Ok(iso) => iso,
        Err(resp) => return resp,
    };

    let gear = match analog_ingest::resolve_ingest_gear(
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

    match db::find_done_analog_ingest_job(&state.db, user.0.id, &ticket.order_number).await {
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
        &ticket.order_number,
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
                ticket_id,
                user_id = user.0.id,
                order = %ticket.order_number,
                "ticket converted to analog ingest job"
            );
            list_ingest_jobs_response(user, State(state)).await
        }
        Err(err) => {
            tracing::error!(?err, ticket_id, "failed to create ingest job from ticket");
            if err.to_string().contains("UNIQUE") {
                return html_error(
                    StatusCode::CONFLICT,
                    "Dieser Auftrag wurde bereits erfolgreich importiert.",
                );
            }
            html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Import-Auftrag konnte nicht angelegt werden.",
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
