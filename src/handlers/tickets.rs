use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use sqlx::SqlitePool;

use crate::{db, models::Ticket, state::AppState};

#[derive(Template)]
#[template(path = "partials/tickets_list.html")]
pub struct TicketsListTemplate {
    pub tickets: Vec<Ticket>,
    pub archived_tickets: Vec<Ticket>,
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

pub async fn render_tickets_list_html(state: &AppState, user_id: i64) -> Result<String, Response> {
    let (tickets, archived_tickets) = load_user_ticket_lists(&state.db, user_id).await;
    TicketsListTemplate {
        tickets,
        archived_tickets,
    }
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
