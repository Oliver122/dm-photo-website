# REQ-010 — Container image

- **ID:** REQ-010
- **Status:** accepted

## Goal

Ship a production Docker image that runs the Axum binary with Askama templates and static assets, without baking secrets or relying on `CARGO_MANIFEST_DIR` at runtime.

## Acceptance criteria

- [x] Multi-stage Rust build → slim runtime image.
- [x] Ships binary + `templates/` + `static/` (Askama + HTMX assets).
- [x] Listens on `8080`; runs as non-root user (entrypoint drops to uid 10001).
- [x] No secrets baked in; `.dockerignore` excludes `.env`, `data/`, `target/`, `.git`.
- [x] Runtime static path is cwd/env-relative (`STATIC_DIR` / `static`).
- [x] `Dockerfile` + `.dockerignore` present at repo root.
- [x] Builder stage runs `cargo test` (REQ **Tests** must pass before release binary).

## Tests

| ID | Case | Where |
|----|------|--------|
| T-010-a | `cargo test` in Docker builder (enforced by Dockerfile `RUN cargo test`) | `Dockerfile` |

- [x] T-010-a

## Out of scope

- Pushing the image (REQ-011).
- App Compose / runtime orchestration (REQ-012).
- Installing Artifactory (REQ-013).

## Touches

- `Dockerfile`, `.dockerignore`
- `src/main.rs` (static serve path fix)

## Depends on

- REQ-013 (registry exists before push; image itself does not require it)
