# Requirements

## Functional

### Auth

- [x] Discord OAuth login creates/updates `users` and session `user_id`.
- [x] Logout clears session user.
- [x] Admin login with `ADMIN_PASSWORD` sets `is_admin`; admin logout clears it.
- [x] User and admin auth are separate (admin password does not require Discord).

### Order tracking

- [x] Accept order numbers matching `^\d{6}-\d{6}$` (exactly 13 chars).
- [x] Query dm Foto order API with configurable `DM_KEY_ACCOUNT_ID` (default 1320).
- [x] Show German timeline labels aligned with dm’s progress wording.
- [x] When status is `ERROR`, allow creating a watch ticket for the logged-in user.
- [x] Manual ticket create / rename (label) / delete for own tickets.
- [x] Background refresh of uncompleted tickets every 3 hours.
- [x] On transition to done (`DELIVERED` / `PICKED_UP`), mark completed and attempt Discord DM if bot token configured.
- [x] Archive completed tickets older than 7 days in the UI (hidden/archive tab).

### Discord DM

- [x] “Send me a Discord DM” uses bot token + shared guild requirement (Discord policy).
- [x] Ticket completion DMs reuse the same bot path.
- [ ] Graceful UX when bot cannot DM (403 / privacy) — keep messages clear; do not crash.

### Admin

- [x] Dashboard of users / tickets tooling.
- [x] Refresh all open tickets on demand.
- [x] Simulate ticket (dev/admin helper).
- [x] Delete all tickets / delete user by id (API).

### API

- [x] `GET /api/me` — current Discord user JSON.
- [x] `GET|DELETE /api/users` — admin user list/delete.
- [x] Order check + ticket CRUD endpoints used by HTMX UI (see ROUTES-AND-DATA).

## Non-functional

- Single-process self-host; SQLite WAL; pool busy timeout for session contention.
- No secrets in git (`.env` gitignored).
- Tracing via `RUST_LOG` / default `info,sqlx=warn`.
- Prefer small, focused modules; keep SQL in `db.rs`.
- German UI copy for order timeline; English ok for admin/errors where already used.

## Explicit constraints for AI agents

1. Never commit `.env`, DB files under `data/`, or secrets.
2. Do not add exploit PoCs against Discord/dm APIs.
3. Preserve order-number format and DONE/READY state constants unless product asks to change them.
4. Cookie `Secure` stays false until HTTPS deployment is intentional.
5. Migrations are additive numbered SQL files; do not rewrite applied migrations.
6. `docs/kurze-wege-*` is not part of the product; ignore for feature work unless asked.
