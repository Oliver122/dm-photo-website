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

## Out of scope

- Fine-grained RBAC beyond `is_admin`.
- Multi-admin accounts (single shared `ADMIN_PASSWORD`).

## Touches

- Admin routes/templates in `src/`
- Session `is_admin` gate

## Depends on

- REQ-001 (admin password session)
- REQ-002 (tickets refreshed/simulated)
