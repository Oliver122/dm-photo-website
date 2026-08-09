# REQ-008 — System tests (HTTP + DB flows)

- **ID:** REQ-008
- **Status:** accepted

## Goal

Require **system-level** automated tests that exercise the running Axum app (router + sessions + DB) with explicit **positive** and **negative** cases — beyond unit tests next to modules. These must pass in `cargo test` and in the Docker builder (`RUN cargo test`).

## Acceptance criteria

- [x] Shared test app factory builds router against a temp SQLite DB (no real Discord/CEWE/PhotoPrism network).
- [x] Positive HTTP: public pages return 200 (`/`, `/login`).
- [x] Negative HTTP: protected APIs without session → redirect `/login` or HTMX `401`.
- [x] Negative HTTP: admin routes without Discord session → redirect `/login` or HTMX `401` (REQ-015); logged-in non-admin → `/admin/login` or HTMX `403`.
- [x] Positive HTTP: Discord session + correct admin password unlocks `/admin` (200).
- [x] Negative HTTP: admin login with wrong password does not grant admin.
- [x] Negative HTTP: anonymous `GET /admin/login` → redirect `/login` (Discord required).
- [x] Positive DB system flow: create user → create analog ingest job → mark done → clear secure_id → find_done (covered by `db::tests` + listed here as ST-db-*).
- [x] REQ **Tests** tables mark **+/-** (positive / negative) for unit and system cases.
- [x] Planned REQs (006/007) include +/- system cases to implement with the feature.

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| ST-008-a | + | `GET /` → 200 | `src/system_tests.rs` |
| ST-008-b | + | `GET /login` → 200 | `src/system_tests.rs` |
| ST-008-c | − | `GET /admin/login` anonymous → redirect `/login` | `src/system_tests.rs` |
| ST-008-d | − | `GET /api/me` → 303/302 to `/login` | `src/system_tests.rs` |
| ST-008-e | − | `GET /api/analog/ingest` + `HX-Request` → 401 | `src/system_tests.rs` |
| ST-008-f | − | `GET /admin` anonymous → redirect `/login` | `src/system_tests.rs` |
| ST-008-g | − | `GET /api/users` + HTMX anonymous → 401 | `src/system_tests.rs` |
| ST-008-h | + | Discord session + admin password → `GET /admin` 200 | `src/system_tests.rs` |
| ST-008-i | − | Discord session + wrong password → still not admin | `src/system_tests.rs` |
| ST-db-a | + | Migrations + analog job happy path | `src/db.rs` |
| ST-db-b | − | Done job blocks re-import discovery (find_done present) | `src/db.rs` |

- [x] ST-008-a … ST-008-i, ST-db-a/b

## Out of scope

- Live calls to Discord, CEWE, or PhotoPrism in CI.
- Browser/Playwright E2E (optional later).

## Touches

- `src/app.rs` (router factory), `src/system_tests.rs`
- `src/main.rs` (use factory)
- Dockerfile already runs `cargo test`

## Depends on

- REQ-001, REQ-003, REQ-004, REQ-005 (surfaces under test)
