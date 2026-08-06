# Bootstrap: `.ai` knowledge base

- **Date:** 2026-08-06
- **Author:** ai
- **PR / branch:** local / `feat/discord-auth-scaffold`
- **Status:** accepted

## Context

Future AI sessions needed a single project-local reference for architecture, requirements, routes (beyond outdated README), branch rules, and review storage.

## Findings

- Root `README.md` documents Discord OAuth scaffold but omits tickets, dm order API, jobs, bot DMs, and several admin/API routes present in `src/main.rs`.
- `docs/kurze-wege-*` is unrelated coursework — not product documentation.
- Auth is dual-track: Discord `user_id` session vs admin password `is_admin` flag.

## Decisions

- Treat `.ai/` as the canonical AI onboarding path; keep README as human quick-start and sync when practical.
- Store review/decision markdown under `.ai/reviews/`.
- Point Cursor always-apply rule at `.ai/INDEX.md`.

## Follow-ups

- [ ] Refresh root `README.md` route/data sections to match `.ai/ROUTES-AND-DATA.md`
- [ ] Confirm cookie `Secure` policy when deploying behind HTTPS
