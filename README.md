# dm-photo-website

Rust + HTMX website with Discord OAuth login, SQLite-backed users, server-side
cookie sessions, a JSON REST API, and admin routes gated by an admin password
stored in `.env`.

**AI / agent context:** see [`.ai/INDEX.md`](.ai/INDEX.md) (architecture, requirements, routes, branch rules, reviews). Prefer that over this README when they disagree.

## Stack

- **Backend:** [Axum](https://github.com/tokio-rs/axum) on Tokio
- **Database:** [SQLx](https://github.com/launchbadge/sqlx) with SQLite + migrations
- **Sessions:** [`tower-sessions`](https://crates.io/crates/tower-sessions) persisted in SQLite
- **OAuth:** [`oauth2`](https://crates.io/crates/oauth2) crate against Discord
- **Templates:** [Askama](https://crates.io/crates/askama) rendering HTMX fragments

## Quick start

1. Install the Rust toolchain (1.94+).
2. Create a Discord application at <https://discord.com/developers/applications>,
   add a redirect URL matching `DISCORD_REDIRECT_URI` (default
   `http://localhost:8080/auth/discord/callback`) and copy the **Client ID** and
   **Client Secret**.
3. Copy the example env file and fill it in:

   ```bash
   cp .env.example .env
   # Edit DISCORD_CLIENT_ID, DISCORD_CLIENT_SECRET, ADMIN_PASSWORD, SESSION_SECRET
   ```

4. Run it:

   ```bash
   cargo run
   ```

   The server listens on `SERVER_ADDR` (default `127.0.0.1:8080`). The SQLite
   database lives at `data/app.db` and is created on first start via migrations
   embedded in the binary.

## Routes

| Method | Path                       | Auth     | Description                              |
| ------ | -------------------------- | -------- | ---------------------------------------- |
| GET    | `/`                        | public   | Landing page                             |
| GET    | `/login`                   | public   | Login page (Discord button)              |
| GET    | `/auth/discord`            | public   | Start the Discord OAuth flow             |
| GET    | `/auth/discord/callback`   | public   | OAuth redirect; creates session          |
| POST   | `/logout`                  | public   | Clear the current session                |
| GET    | `/admin/login`             | public   | Admin login form                         |
| POST   | `/admin/login`             | public   | Verify `ADMIN_PASSWORD`                  |
| GET    | `/admin`                   | admin    | Admin dashboard                          |
| GET    | `/api/me`                  | user     | Returns the current Discord user as JSON |
| GET    | `/api/users`               | admin    | Lists all known users                    |
| DELETE | `/api/users/:id`           | admin    | Deletes a user by id                     |

## Data

Single `users` table:

| Column      | Type    | Notes                       |
| ----------- | ------- | --------------------------- |
| `id`        | INTEGER | Primary key                 |
| `discord_id`| TEXT    | Unique Discord snowflake id |
| `username`  | TEXT    | Discord username            |
| `created_at`| TEXT    | ISO timestamp               |
| `last_login`| TEXT    | ISO timestamp               |

The `tower_sessions` session table is created automatically by the store.

## Container image (Artifactory)

JFrog Artifactory is operated on the server outside this repo. This project only
builds/pushes the app image and runs it via Compose.

**Build and push** (needs Docker + Artifactory credentials):

```bash
export ARTIFACTORY_DOCKER_REGISTRY=artifactory.example.com/docker-local
export ARTIFACTORY_USER=...
export ARTIFACTORY_TOKEN=...   # or ARTIFACTORY_PASSWORD
# optional: IMAGE_NAME=dm-photo-website  TAG=<sha>  PUSH_LATEST=1
./scripts/build-and-push.sh
```

**Run the app** (pull from Artifactory; put `.env` beside the compose file):

```bash
export ARTIFACTORY_DOCKER_REGISTRY=artifactory.example.com/docker-local
export TAG=latest   # or a short git SHA you pushed
cp .env.example deploy/app/.env   # then edit secrets / DISCORD_REDIRECT_URI
docker compose -f deploy/app/docker-compose.yml up -d
```

SQLite persists in the `app-data` volume at `/data/app.db`. If the public host
is not localhost, set `DISCORD_REDIRECT_URI` to match the Discord app redirect.
