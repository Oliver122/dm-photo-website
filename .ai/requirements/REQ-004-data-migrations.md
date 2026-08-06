# REQ-004 — Data & migrations

- **ID:** REQ-004
- **Status:** accepted

## Goal

Persist app state in SQLite with additive migrations; keep secrets and DB files out of git; document constraints for agents.

## Acceptance criteria

### Non-functional

- [x] Single-process self-host; SQLite WAL; pool busy timeout for session contention.
- [x] No secrets in git (`.env` gitignored).
- [x] Tracing via `RUST_LOG` / default `info,sqlx=warn`.
- [x] Prefer small, focused modules; keep SQL in `db.rs`.
- [x] German UI copy for order timeline; English ok for admin/errors where already used.
- [x] DB file at `data/app.db` (gitignored); migrations under `migrations/`.

### Explicit constraints for AI agents

1. Never commit `.env`, DB files under `data/`, or secrets.
2. Do not add exploit PoCs against Discord/dm APIs.
3. Preserve order-number format and DONE/READY state constants unless product asks to change them.
4. Cookie `Secure` stays false until HTTPS deployment is intentional.
5. Migrations are additive numbered SQL files; do not rewrite applied migrations.
6. `docs/kurze-wege-*` is not part of the product; ignore for feature work unless asked.

## Tests

| ID | Case | Where |
|----|------|--------|
| T-004-a | Migrations apply on empty SQLite (`:memory:` or temp file) | `src/db.rs` |
| T-004-b | Analog ingest job create + find_done + clear secure_id | `src/db.rs` |

- [x] T-004-a … T-004-b

## Out of scope

- Multi-process / multi-host SQLite sharing.
- Rewriting applied migration history.

## Touches

- `migrations/`, `src/db.rs`, `data/` (runtime only)
- `.env` / `.env.example`

## Depends on

- None
