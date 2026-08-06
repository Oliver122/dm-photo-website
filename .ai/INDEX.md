# AI context — dm-photo-website

**Start here** for every AI session on this repo. Human README: [`../README.md`](../README.md). Prefer this folder when README and code disagree — README lags the ticket/DM-order work.

## Read order

1. [`PROJECT.md`](PROJECT.md) — what this is, goals, non-goals
2. [`ARCHITECTURE.md`](ARCHITECTURE.md) — modules, request flow, jobs
3. [`requirements/_index.md`](requirements/_index.md) — product + engineering requirements (open by REQ id; do not load the whole `requirements/` folder into context). Legacy pointer: [`REQUIREMENTS.md`](REQUIREMENTS.md)
4. [`ROUTES-AND-DATA.md`](ROUTES-AND-DATA.md) — HTTP surface + SQLite schema
5. [`CONVENTIONS.md`](CONVENTIONS.md) — how to change code safely
6. [`GIT-AND-BRANCHES.md`](GIT-AND-BRANCHES.md) — branch naming, commits, PRs
7. [`reviews/`](reviews/) — design/PR review notes (append, don’t overwrite)

## Quick facts

| Item | Value |
|------|--------|
| Language | Rust (edition 2024), toolchain 1.94+ |
| Web | Axum + Askama + HTMX |
| DB | SQLite via SQLx, migrations in `migrations/` |
| Auth | Discord OAuth (users) + `ADMIN_PASSWORD` session flag (admin) |
| External APIs | Discord OAuth/Bot API, dm Foto `spot.photoprintit.com`, dm analog download (CEWE `api.cewe-myphotos.com`), PhotoPrism `/api/v1` (REQ-005) |
| Planned ingest | REQ-006 film ISO + lens EXIF; REQ-007 preview + rotate before import |
| Default listen | `127.0.0.1:8080` |
| DB file | `data/app.db` (gitignored) |
| Secrets | `.env` (never commit) — see `.env.example` |

## Out of scope in this repo

- `docs/kurze-wege-*` and `docs/literatur.bib` are unrelated university notes, not product docs.
- Do not treat them as feature requirements for the website.

## Updating this folder

When behavior, routes, schema, or branch policy change, update the matching `.ai` file in the same PR/commit when practical. New review write-ups go under `reviews/YYYY-MM-DD-short-slug.md`.
