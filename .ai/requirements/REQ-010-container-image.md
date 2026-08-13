# REQ-010 — Container image

- **ID:** REQ-010
- **Status:** accepted

## Goal

Ship a production Docker image that runs the Axum binary with Askama templates and static assets, without baking secrets or relying on `CARGO_MANIFEST_DIR` at runtime. The image must **self-initialize** persist paths so SQLite and ingest never fail with “unable to open database file” due to missing dirs or legacy URL forms.

## Acceptance criteria

- [x] Multi-stage Rust build → slim runtime image.
- [x] Ships binary + `templates/` + `static/` (Askama + HTMX assets).
- [x] Listens on `8080`; runs as non-root user (entrypoint drops to uid 10001).
- [x] No secrets baked in; `.dockerignore` excludes `.env`, `data/`, `target/`, `.git`.
- [x] Runtime static path is cwd/env-relative (`STATIC_DIR` / `static`).
- [x] `Dockerfile` + `.dockerignore` present at repo root.
- [x] Builder stage runs `cargo test` (REQ **Tests** must pass before release binary).
- [x] **Boot init (entrypoint):** create `/app/data` + ingest dir; `chown` to app; verify writable; rewrite legacy `DATABASE_URL` values (`sqlite://data/...`) to `sqlite:/app/data/app.db`.
- [x] **Default ENV:** `DATABASE_URL=sqlite:/app/data/app.db`, `ANALOG_INGEST_DIR=/app/data/ingest` (absolute).
- [x] **App init:** `main` creates ingest + DB parent dirs before `init_pool`; `init_pool` normalizes SQLite URLs.

## Tests

| ID | Case | Where |
|----|------|--------|
| T-010-a | `cargo test` in Docker builder (enforced by Dockerfile `RUN cargo test`) | `Dockerfile` |
| T-010-b | SQLite URL normalize (`sqlite://data/app.db` → path form) | `src/db.rs` |

- [x] T-010-a
- [x] T-010-b (`normalize_sqlite_url_absolute_and_relative`)

## Out of scope

- Pushing the image (REQ-011).
- App Compose / runtime orchestration (REQ-012).
- Installing Artifactory (REQ-013).

## Touches

- `Dockerfile`, `docker-entrypoint.sh`, `.dockerignore`
- `src/main.rs`, `src/db.rs` (dir create + URL normalize)

## Depends on

- REQ-013 (registry exists before push; image itself does not require it)
