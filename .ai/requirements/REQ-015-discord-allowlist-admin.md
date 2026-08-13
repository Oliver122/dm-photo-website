# REQ-015 — Discord allowlist + gated admin

- **ID:** REQ-015
- **Status:** accepted

## Goal

Restrict Discord login to an allowlist (DB + env seed); deny OAuth before any `users` row is created. Require a Discord session before admin access; admin capability via password flag and/or Discord admin IDs. Ship deploy-friendly Compose defaults (`ADMIN_PASSWORD=changeme`, documented allowlist env).

## Locked decisions

- Match Discord **snowflake ID** and/or **username handle** (case-insensitive; not display name). Username-only rows bind to snowflake on first OAuth.
- **Empty allowlist** → deny all Discord logins (fail closed).
- **Denied OAuth** → redirect `/login?denied=1` (German); **no `users` row** created.
- **Admin gate:** Discord session (`AuthUser`) **and** (`ADMIN_PASSWORD` session flag **or** allowlist `is_admin`).
- **`/admin/login` password step** requires Discord session first (anonymous → `/login`).
- Deploy default **`ADMIN_PASSWORD=changeme`**; boot **WARN** + admin dashboard banner while unchanged.

## Acceptance criteria

- [x] `discord_allowlist` table + `discord_pending_handles` queue; boot seeds IDs/handles from `DISCORD_ALLOWLIST` / `DISCORD_ADMIN_IDS`.
- [x] OAuth callback checks allowlist **before** `upsert_discord_user`; denied → `/login?denied=1` (German), no `users` row.
- [x] Empty allowlist → deny all Discord logins (fail closed).
- [x] `AdminUser` requires Discord session (`AuthUser`) **and** admin capability (`ADMIN_KEY` or admin Discord ID / DB `is_admin`).
- [x] `/admin/login` password step requires Discord session first.
- [x] Admin dashboard: list/add/remove allowlist entries; toggle admin flag; last-admin guard.
- [x] Boot WARN + admin banner while `ADMIN_PASSWORD` is still `changeme`.
- [x] `deploy/app/docker-compose.yml` mirrors app env with `${VAR:-default}`; `SESSION_SECRET` required at compose time.

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| T-015-a | + | Seed allowlist ID → allowlisted true | `src/db.rs` |
| T-015-b | − | Unknown ID → not allowlisted | `src/db.rs` |
| T-015-c | + | Config parses comma-separated IDs | `src/config.rs` |
| T-015-d | + | Upsert does not demote last admin; demote/delete guarded | `src/db.rs` |
| T-015-e | + | Seed skips garbage; handle not allowlisted until claim copies `is_admin` and drops pending | `src/db.rs` |
| T-015-pending | − | Pending-only handle fails `is_discord_allowlisted` | `src/db.rs` |
| T-015-empty | − | Empty claimed + empty pending denies | `src/db.rs` |
| ST-015-a | − | Denied OAuth path: no user row created | `src/system_tests.rs` |
| ST-015-b | − | `GET /admin` without Discord session → redirect `/login` | `src/system_tests.rs` |
| ST-015-c | − | Discord session, not admin → no dashboard | `src/system_tests.rs` |
| ST-015-d | + | Allowlisted admin Discord session → `/admin` 200 | `src/system_tests.rs` |
| ST-015-e | + | Allowlisted user + correct password → admin | `src/system_tests.rs` |
| ST-015-f | − | Empty allowlist → login denied | `src/system_tests.rs` |
| ST-015-g | − | Last admin toggle → 400 | `src/system_tests.rs` |
| ST-015-h | + | Allowlist add HTMX; upsert cannot demote admin | `src/system_tests.rs` |
| ST-015-i | − | Revoked allowlist clears session | `src/system_tests.rs` |

## Out of scope

- Matching allowlist by Discord **display name**.
- Invite links / email verification.
- Multiple password admin accounts.
- Changing PhotoPrism / ingest flows.

## Touches

- `migrations/0009_discord_allowlist.sql`, `migrations/0010_discord_pending_handles.sql`, `src/config.rs`, `src/db.rs`
- `src/handlers/pages.rs` (OAuth gate, login denied), `src/auth/session.rs` (admin gate)
- Admin allowlist routes/templates
- `deploy/app/docker-compose.yml`, `.env.example`, `deploy/app/.env.example`
- `.ai/PROJECT.md`, `ROUTES-AND-DATA.md`, `FRONTEND-SURFACE.md`

## Depends on

- REQ-001 (Discord OAuth + admin password session)
- REQ-003 (admin dashboard and APIs)
