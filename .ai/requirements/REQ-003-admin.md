# REQ-003 — Admin

- **ID:** REQ-003
- **Status:** accepted

## Goal

Provide admin-only dashboard and APIs for user/ticket tooling without requiring Discord for the admin session.

## Acceptance criteria

- [x] Dashboard of users / tickets tooling.
- [x] Refresh all open tickets on demand.
- [x] Simulate ticket (dev/admin helper).
- [x] Delete all tickets / delete user by id (API).
- [x] `GET|DELETE /api/users` — admin user list/delete.

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| ST-008-c | + | Admin login page 200 | `src/system_tests.rs` |
| ST-008-f | − | `/admin` without session → redirect | `src/system_tests.rs` |
| ST-008-g | − | `/api/users` HTMX without admin → 403 | `src/system_tests.rs` |
| ST-008-h | + | Correct admin password → dashboard 200 | `src/system_tests.rs` |
| ST-008-i | − | Wrong admin password → no dashboard | `src/system_tests.rs` |

- [x] Covered via REQ-008 system tests

## Out of scope

- Fine-grained RBAC beyond `is_admin`.
- Multi-admin accounts (single shared `ADMIN_PASSWORD`).

## Touches

- Admin routes/templates in `src/`
- Session `is_admin` gate

## Depends on

- REQ-001 (admin password session)
- REQ-002 (tickets refreshed/simulated)
