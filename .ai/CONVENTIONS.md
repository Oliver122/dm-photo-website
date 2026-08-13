# Conventions for code changes

## Style

- Match existing Rust style in neighboring files (imports, error handling with `anyhow`/`thiserror`, `tracing` for logs).
- Prefer `anyhow::Context` on fallible paths in handlers/jobs; domain errors in `dm_order` / `discord_bot` use `thiserror`.
- Keep handlers thin: validation + call `db` / `dm_order` / `jobs` / `discord_bot`.
- New SQL → `db.rs` functions; new routes → register in `main.rs` and implement under `handlers/`.
- Askama templates under `templates/`; shared chrome in `base.html`. Static assets under `static/`.

## Auth

- Protect user routes with `AuthUser` extractor; admin with `AdminUser`.
- Do not invent a third auth scheme without updating `.ai` + session keys in `auth/session.rs`.

## Migrations

1. Add next file `migrations/000N_description.sql`.
2. Never edit already-applied migration files in shared/prod DBs.
3. SQLite `ALTER TABLE` limitations: nullable columns + backfill when defaults can’t be expressions (see `0004_ticket_timestamps.sql`).

## Env / config

- New knobs: add to `.env.example`, parse in `config.rs`, document in `.ai/PROJECT.md`.
- Values with spaces in `.env` must be quoted.

## Frontend

- HTMX attributes on templates; reuse OOB ticket list helper when mutating tickets.
- Styling: hand-authored CSS in `static/styles.css` with design tokens (see REQ-014). **No Tailwind / Bootstrap / npm frontend toolchain** unless a REQ explicitly adds one.
- This is not a React app — no SPA frameworks.

## Tests

- Unit tests live next to modules (e.g. `dm_order.rs` format/timeline tests).
- Every `REQ-*.md` **Tests** section lists required cases (`T-<req>-…`); keep them green.
- Run: `cargo test` and `cargo check` before finishing non-trivial changes.
- Docker image build runs `cargo test` in the builder stage before `cargo build --release`.
- GitHub Actions (`.github/workflows/ci.yml`) runs `cargo test` then `cargo llvm-cov` on pull requests and pushes to `main`. Frontend coverage is the HTTP system tests (`src/system_tests.rs`).

## Docs sync

When you change routes, schema, env vars, or auth behavior:

1. Update `.ai/ROUTES-AND-DATA.md` / `PROJECT.md` / `REQUIREMENTS.md` as needed.
2. Update root `README.md` if the change is user-facing setup or the public route table.
3. Drop a short note in `.ai/reviews/` only for design decisions or post-review findings — not for every typo fix.

## Security checklist (every change)

- No secrets in logs or HTML.
- Admin actions stay behind `AdminUser`.
- Users may only mutate their own tickets (verify `user_id`).
- Validate order numbers before calling external APIs.
