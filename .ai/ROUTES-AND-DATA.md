# Routes and data

Canonical route table for the **current** codebase (`src/main.rs`). Root `README.md` is incomplete for tickets/admin ticket actions — update README when convenient, but trust this file + `main.rs` first.

## HTTP routes

| Method | Path | Auth | Handler area | Notes |
|--------|------|------|--------------|-------|
| GET | `/` | public | pages | Landing; may show user + tickets + analog ingest |
| GET | `/login` | public | pages | Discord login CTA |
| POST | `/logout` | public | pages | Clears user session |
| GET | `/auth/discord` | public | pages | Start OAuth |
| GET | `/auth/discord/callback` | public | pages | Finish OAuth |
| GET | `/admin/login` | public | pages | Admin password form |
| POST | `/admin/login` | public | pages | Verify password |
| POST | `/admin/logout` | public | pages | Clear admin flag |
| GET | `/admin` | admin | pages | Dashboard |
| POST | `/admin/tickets/refresh` | admin | api | Run refresh cycle now |
| DELETE | `/admin/tickets` | admin | api | Delete all tickets |
| POST | `/admin/tickets/simulate` | admin | api | Create simulated ticket |
| GET | `/api/me` | user | api | Current user JSON |
| POST | `/api/dm/me` | user | api | Send test Discord DM |
| GET | `/api/analog/ingest` | user | analog_ingest | HTMX partial: analog ingest job list |
| POST | `/api/analog/ingest` | user | analog_ingest | Queue analog ingest job |
| DELETE | `/api/analog/ingest/:id` | user | analog_ingest | Delete job (queued/preview/done/failed) + workdir; allows re-import |
| GET | `/api/analog/ingest/:id/preview` | user | analog_ingest | HTMX partial: preview gallery (owner; job status `preview`) |
| GET | `/api/analog/ingest/:id/preview/file?path=…` | user | analog_ingest | Preview image bytes (owner; path under workdir) |
| POST | `/api/analog/ingest/:id/preview/rotate` | user | analog_ingest | Rotate one image 90° CW/CCW (`file`, `direction`) |
| POST | `/api/analog/ingest/:id/preview/confirm` | user | analog_ingest | Confirm import → status `labeling` |
| POST | `/api/analog/ingest/:id/preview/cancel` | user | analog_ingest | Cancel preview → `failed`, clear `secure_id`, delete workdir |
| POST | `/api/order/check` | user | api | Lookup order; may create ticket |
| POST | `/api/tickets` | user | api | Manual ticket create |
| DELETE | `/api/tickets/:id` | user | api | Delete own ticket |
| POST | `/api/tickets/:id/label` | user | api | Rename own ticket |
| POST | `/api/tickets/:id/gear` | user | tickets | Save camera, lens, film ISO on ticket |
| POST | `/api/tickets/:id/convert` | user | tickets | Convert ticket → analog ingest job (Secure-ID at submit) |
| GET | `/api/users` | admin | api | List users |
| DELETE | `/api/users/:id` | admin | api | Delete user |
| * | `/static/*` | public | ServeDir | CSS/JS |

## Schema

Migrations (embedded via SQLx in `db::init_pool`):

| File | Purpose |
|------|---------|
| `0001_init.sql` | `users` |
| `0002_tickets.sql` | `tickets` |
| `0003_ticket_completed.sql` | `tickets.completed` |
| `0004_ticket_timestamps.sql` | `last_updated`, `completed_at` |
| `0005_ticket_label.sql` | `tickets.label` |
| `0006_analog_ingest.sql` | `analog_ingest_jobs` |
| `0007_analog_ingest_partial_unique.sql` | partial unique on done jobs |
| `0008_user_gear.sql` | `user_cameras`, `user_lenses`; gear columns on tickets + jobs |

Sessions table owned by `tower-sessions-sqlx-store` (separate migrate).

### `users`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK | |
| `discord_id` | TEXT UNIQUE | Snowflake |
| `username` | TEXT | |
| `created_at` | TEXT | ISO / SQLite datetime |
| `last_login` | TEXT | Updated on OAuth upsert |

### `tickets`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK | |
| `user_id` | INTEGER FK → users ON DELETE CASCADE | |
| `order_number` | TEXT | `NNNNNN-NNNNNN` |
| `label` | TEXT NULL | User-facing name |
| `customer_no` / `shop_no` / `order_no` | TEXT NULL | From API |
| `summary_state_code` | TEXT | e.g. ERROR, PROCESSING, DELIVERED |
| `summary_state_text` | TEXT NULL | |
| `status` | TEXT | Default `open` |
| `completed` | INTEGER bool | 0/1 |
| `created_at` | TEXT | |
| `last_updated` | TEXT NULL | App always writes explicitly |
| `completed_at` | TEXT NULL | |
| `camera_id` | INTEGER NULL FK → user_cameras | Optional gear for import |
| `lens_id` | INTEGER NULL FK → user_lenses | |
| `film_iso` | INTEGER NULL | Film stock ISO |

### `user_cameras`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK | |
| `user_id` | INTEGER FK → users ON DELETE CASCADE | |
| `label` | TEXT | Unique per user (trim, case-insensitive) |
| `created_at` | TEXT | |

### `user_lenses`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK | |
| `user_id` | INTEGER FK → users ON DELETE CASCADE | |
| `name` | TEXT | Unique per user |
| `focal_mm` | REAL | > 0 |
| `aperture` | REAL | f-number, > 0 |
| `created_at` | TEXT | |

### `analog_ingest_jobs`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK | |
| `user_id` | INTEGER FK → users ON DELETE CASCADE | |
| `order_number` | TEXT | `NNNNNN-NNNNNN` |
| `secure_id` | TEXT NULL | Cleared after successful import |
| `camera_label` | TEXT | User-supplied camera name |
| `album` | TEXT NULL | Optional PhotoPrism album for this batch |
| `status` | TEXT | `queued` / `downloading` / `preview` / `labeling` / `uploading` / `done` / `failed` |
| `error_text` | TEXT NULL | German/technical message on failure |
| `created_at` / `updated_at` | TEXT | |
| `camera_id` | INTEGER NULL FK → user_cameras | |
| `lens_id` | INTEGER NULL FK → user_lenses | |
| `film_iso` | INTEGER NULL | Film stock ISO |

Partial unique index `analog_ingest_jobs_user_order_done_idx` on `(user_id, order_number)` **WHERE `status = 'done'`** — idempotent re-import guard (failed/queued jobs for the same order may coexist).

## Models

Rust structs in `src/models.rs`: `User`, `UserCamera`, `UserLens`, `Ticket` (+ `completed_before`), `AnalogIngestJob` (+ `gear_line`).

Status strings and helpers: `ANALOG_INGEST_STATUS_*` constants, `is_valid_analog_ingest_status`, `is_terminal_analog_ingest_status`, `AnalogIngestJob::status_label_de`.

## DB helpers (analog ingest)

Analog ingest SQL uses the `analog_ingest_*` naming prefix (not generic `ingest_*`):

| Function | Role |
|----------|------|
| `create_analog_ingest_job` | Insert queued job |
| `get_analog_ingest_job` | Fetch by id |
| `list_analog_ingest_jobs_for_user` | User’s jobs, newest first |
| `claim_next_queued_analog_ingest_job` | Atomic claim → `downloading` |
| `update_analog_ingest_job_status` | Status + optional `error_text` |
| `clear_analog_ingest_secure_id` | NULL `secure_id` after success |
| `confirm_analog_ingest_preview_for_user` | Owner confirm: `preview` → `labeling` |
| `cancel_analog_ingest_preview_for_user` | Owner cancel: `preview` → `failed`, clear `secure_id` |
| `find_done_analog_ingest_job` | Idempotency check for completed order |
| `list_user_cameras` / `list_user_lenses` | Gear library for selects |
| `update_ticket_gear_for_user` | Persist camera/lens/ISO on ticket |
