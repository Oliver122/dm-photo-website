# REQ-001 — Auth

- **ID:** REQ-001
- **Status:** accepted

## Goal

Users authenticate via Discord OAuth; admins via a separate password session flag. Both can coexist without coupling.

## Acceptance criteria

- [x] Discord OAuth login creates/updates `users` and session `user_id`.
- [x] Logout clears session user.
- [x] Admin login with `ADMIN_PASSWORD` sets `is_admin`; admin logout clears it.
- [x] User and admin auth are separate (admin password does not require Discord).
- [x] `GET /api/me` returns current Discord user JSON when logged in.

## Tests

| ID | Case | Where |
|----|------|--------|
| T-001-a | Admin password match / reject wrong / reject different length | `src/auth/admin.rs` |

- [x] T-001-a

## Out of scope

- HTTPS / cookie `Secure=true` until intentional HTTPS deployment.
- Third-party IdPs other than Discord.

## Touches

- `src/auth.rs`, `src/main.rs` (auth routes/middleware)
- Session cookie handling
- `.env`: Discord OAuth + `ADMIN_PASSWORD`

## Depends on

- None
