# REQ-010 — Container image

- **ID:** REQ-010
- **Status:** planned

## Goal

Ship a production Docker image that runs the Axum binary with Askama templates and static assets, without baking secrets or relying on `CARGO_MANIFEST_DIR` at runtime.

## Acceptance criteria

- [ ] Multi-stage Rust build → slim runtime image.
- [ ] Ships binary + `templates/` + `static/` (Askama + HTMX assets).
- [ ] Listens on `8080`; runs as non-root user.
- [ ] No secrets baked in; `.dockerignore` excludes `.env`, `data/`, `target/`, `.git`.
- [ ] Runtime static path is cwd/env-relative (not `CARGO_MANIFEST_DIR`) so the image works.
- [ ] `Dockerfile` + `.dockerignore` present at repo root (or documented path).

## Out of scope

- Pushing the image (REQ-011).
- App Compose / runtime orchestration (REQ-012).
- Installing Artifactory (REQ-013).

## Touches

- `Dockerfile`, `.dockerignore`
- `src/main.rs` (static serve path fix)

## Depends on

- REQ-013 (registry exists before push; image itself does not require it)
