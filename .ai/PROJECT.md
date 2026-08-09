# Project overview

## Purpose

Personal web app for tracking **dm Foto** print orders. Users sign in with Discord, submit an order number (`NNNNNN-NNNNNN`), and get status from the public dm spot API. When an order is not ready yet (`ERROR` / not initialized), the site can open a **ticket** and watch until pickup-ready, then notify the user via Discord DM.

Logged-in users can also queue **dm analog** ingest jobs: download a CEWE photo pack with order + Secure-ID, stamp a camera label into EXIF, and upload into a configured **PhotoPrism** instance (REQ-005).

Admin area requires a Discord session plus admin capability (`ADMIN_PASSWORD` login or allowlist `is_admin`). It lists users, manages the Discord allowlist, can refresh/simulate/delete tickets, and exposes user JSON API. Boot warns while `ADMIN_PASSWORD` remains the deploy default `changeme`.

## Product goals

- Low-friction Discord login (no local passwords for users).
- Reliable order lookup against `spot.photoprintit.com`.
- Persistent tickets for orders that are not yet in the pipeline.
- Background refresh (~every 3 hours) + optional admin “refresh now”.
- Discord DM when a watched ticket becomes done (`DELIVERED` / `PICKED_UP`).
- Analog ingest: CEWE download → EXIF camera label → PhotoPrism import, with job status in the UI.
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
| `DISCORD_ALLOWLIST` | no* | Comma/whitespace-separated Discord snowflake IDs; seeds `discord_allowlist` on boot (*empty = deny all logins) |
| `DISCORD_ADMIN_IDS` | no | IDs seeded with `is_admin=1`; also grant admin without password when logged in |
| `ADMIN_PASSWORD` | yes | Admin login; deploy Compose default `changeme` — change before production |
| `SESSION_SECRET` | yes | ≥64 bytes, signs cookies |
| `PHOTOPRISM_BASE_URL` | no* | PhotoPrism base URL (*required for analog ingest) |
| `PHOTOPRISM_USERNAME` | no* | PhotoPrism login username for `POST /api/v1/session` |
| `PHOTOPRISM_APP_PASSWORD` | no* | Password / app password for session login |
| `PHOTOPRISM_USER_UID` | no | Optional; upload uses session `user.UID` (warn if mismatch) |
| `PHOTOPRISM_DEFAULT_ALBUM` | no | Fallback album when job has none |
| `PHOTOPRISM_VERIFY_TLS` | no | Default `true` |
| `ANALOG_INGEST_DIR` | no | Temp work dir (default `data/ingest`) |

## Entry point

`src/main.rs`: load config → SQLite pool + migrations → session store → AppState → spawn ticket refresher + analog ingest worker → Axum router → serve.
