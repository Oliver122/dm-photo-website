# Routes and data

Canonical route table for the **current** codebase (`src/main.rs`). Root `README.md` is incomplete for tickets/admin ticket actions — update README when convenient, but trust this file + `main.rs` first.

## HTTP routes

| Method | Path | Auth | Handler area | Notes |
|--------|------|------|--------------|-------|
| GET | `/` | public | pages | Landing; may show user + tickets |
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
| POST | `/api/order/check` | user | api | Lookup order; may create ticket |
| POST | `/api/tickets` | user | api | Manual ticket create |
| DELETE | `/api/tickets/:id` | user | api | Delete own ticket |
| POST | `/api/tickets/:id/label` | user | api | Rename own ticket |
| GET | `/api/users` | admin | api | List users |
| DELETE | `/api/users/:id` | admin | api | Delete user |
| * | `/static/*` | public | ServeDir | CSS/JS |

## Schema

Migrations: `migrations/0001_init.sql` … `0005_ticket_label.sql`. Sessions table owned by `tower-sessions-sqlx-store`.

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

## Models

Rust structs in `src/models.rs`: `User`, `Ticket` (+ `completed_before`).
