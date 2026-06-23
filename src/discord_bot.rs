use serde::{Deserialize, Serialize};
use thiserror::Error;

const API_BASE: &str = "https://discord.com/api/v10";

#[derive(Debug, Error)]
pub enum BotError {
    #[error("DISCORD_BOT_TOKEN is not configured")]
    NotConfigured,
    #[error("network error talking to discord: {0}")]
    Network(#[from] reqwest::Error),
    #[error("discord refused to open DM channel (status {status}): {body}")]
    OpenChannel { status: u16, body: String },
    #[error("discord refused to send message (status {status}): {body}")]
    SendMessage { status: u16, body: String },
}

#[derive(Debug, Serialize)]
struct CreateDmRequest<'a> {
    recipient_id: &'a str,
}

#[derive(Debug, Deserialize)]
struct DmChannel {
    id: String,
}

#[derive(Debug, Serialize)]
struct CreateMessageRequest<'a> {
    content: &'a str,
}

/// Open (or fetch) the DM channel with `recipient_id` and post `content` to it
/// using the supplied bot token. Returns the message id on success.
///
/// Discord will reject this with HTTP 403 / "Cannot send messages to this user"
/// when the user does not share a guild with the bot, or when their privacy
/// settings disallow DMs from server members.
pub async fn send_dm(
    http: &reqwest::Client,
    bot_token: Option<&str>,
    recipient_id: &str,
    content: &str,
) -> Result<String, BotError> {
    let token = bot_token.ok_or(BotError::NotConfigured)?;
    let auth = format!("Bot {token}");

    let create_channel = http
        .post(format!("{API_BASE}/users/@me/channels"))
        .header("Authorization", &auth)
        .json(&CreateDmRequest { recipient_id })
        .send()
        .await?;
    if !create_channel.status().is_success() {
        let status = create_channel.status().as_u16();
        let body = create_channel.text().await.unwrap_or_default();
        return Err(BotError::OpenChannel { status, body });
    }
    let channel: DmChannel = create_channel.json().await?;

    let send = http
        .post(format!("{API_BASE}/channels/{}/messages", channel.id))
        .header("Authorization", &auth)
        .json(&CreateMessageRequest { content })
        .send()
        .await?;
    if !send.status().is_success() {
        let status = send.status().as_u16();
        let body = send.text().await.unwrap_or_default();
        return Err(BotError::SendMessage { status, body });
    }

    #[derive(Deserialize)]
    struct Message {
        id: String,
    }
    let msg: Message = send.json().await?;
    Ok(msg.id)
}
