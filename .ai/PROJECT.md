# Project overview

## Purpose

Personal web app for tracking **dm Foto** print orders. Users sign in with Discord, submit an order number (`NNNNNN-NNNNNN`), and get status from the public dm spot API. When an order is not ready yet (`ERROR` / not initialized), the site can open a **ticket** and watch until pickup-ready, then notify the user via Discord DM.

Admin area (password-gated) lists users, can refresh/simulate/delete tickets, and manage users via JSON API.

## Product goals

- Low-friction Discord login (no local passwords for users).
- Reliable order lookup against `spot.photoprintit.com`.
- Persistent tickets for orders that are not yet in the pipeline.
- Background refresh (~every 3 hours) + optional admin “refresh now”.
- Discord DM when a watched ticket becomes done (`DELIVERED` / `PICKED_UP`).
- HTMX-friendly UI: fragments + out-of-band ticket list swaps.

## Non-goals (current)

- Multi-tenant SaaS, billing, or public registration beyond Discord.
- Full Discord bot gateway (HTTP bot token only for DMs).
- Replacing dm’s official photo site; this is a tracker/helper.
- Production hardening beyond local/self-host defaults (e.g. `with_secure(false)` on cookies until HTTPS).

## Stack (canonical)

Matches README “Stack” section; keep README and this file aligned when deps change:

- Axum / Tokio
- SQLx + SQLite + embedded migrations
- tower-sessions (+ SQLite store), signed cookies
- oauth2 + Discord OAuth2
- Askama templates + HTMX (`static/htmx.min.js`)
- reqwest (rustls) for Discord + dm APIs

## Runtime config

Loaded in `src/config.rs` from process env / `.env`:

| Variable | Required | Role |
|----------|----------|------|
| `SERVER_ADDR` | no | Bind address (default `127.0.0.1:8080`) |
| `DATABASE_URL` | no | Default `sqlite://data/app.db` |
| `DISCORD_CLIENT_ID` | yes | OAuth app |
| `DISCORD_CLIENT_SECRET` | yes | OAuth app |
| `DISCORD_REDIRECT_URI` | no | Default localhost callback |
| `DISCORD_BOT_TOKEN` | no | Needed for DM features |
| `DM_MESSAGE` | no | Default greeting for “DM me” |
| `DM_KEY_ACCOUNT_ID` | no | Default `1320` (dm Germany) |
| `ADMIN_PASSWORD` | yes | Admin login |
| `SESSION_SECRET` | yes | ≥64 bytes, signs cookies |

## Entry point

`src/main.rs`: load config → SQLite pool + migrations → session store → AppState → spawn ticket refresher → Axum router → serve.
