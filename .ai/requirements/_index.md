# Requirements index

Open by **REQ id** — do not dump this folder into context. See [`README.md`](README.md).

| ID | Summary | Status | File |
|----|---------|--------|------|
| REQ-001 | Discord OAuth + admin password auth | accepted | [REQ-001-auth.md](REQ-001-auth.md) |
| REQ-002 | Order check, tickets, Discord DMs | accepted | [REQ-002-orders-tickets.md](REQ-002-orders-tickets.md) |
| REQ-003 | Admin dashboard and tooling | accepted | [REQ-003-admin.md](REQ-003-admin.md) |
| REQ-004 | SQLite data, migrations, agent constraints | accepted | [REQ-004-data-migrations.md](REQ-004-data-migrations.md) |
| REQ-005 | DM analog download → camera EXIF → PhotoPrism | accepted | [REQ-005-dm-analog-download.md](REQ-005-dm-analog-download.md) |
| REQ-006 | Separate cameras/lenses, ISO, ticket→import | accepted | [REQ-006-film-lens-exif.md](REQ-006-film-lens-exif.md) |
| REQ-007 | Ingest preview + rotate before PhotoPrism import | accepted | [REQ-007-ingest-preview-rotate.md](REQ-007-ingest-preview-rotate.md) |
| REQ-008 | System tests (HTTP +/- + DB flows) | accepted | [REQ-008-system-tests.md](REQ-008-system-tests.md) |
| REQ-009 | Home UI overhaul (simpler; pick variant A/B) | accepted | [REQ-009-home-ui-overhaul.md](REQ-009-home-ui-overhaul.md) |
| REQ-010 | Container image (Dockerfile) | accepted | [REQ-010-container-image.md](REQ-010-container-image.md) |
| REQ-011 | Artifactory build/push script + env | accepted | [REQ-011-artifactory-push.md](REQ-011-artifactory-push.md) |
| REQ-012 | App Compose (run from Artifactory) | accepted | [REQ-012-app-compose.md](REQ-012-app-compose.md) |
| REQ-013 | JFrog server checklist (ops-owned) | planned | [REQ-013-jfrog-server.md](REQ-013-jfrog-server.md) |
| REQ-014 | Frontend redesign (capability-preserving) | planned | [REQ-014-frontend-redesign.md](REQ-014-frontend-redesign.md) |

**UI contract (not a REQ):** [`.ai/FRONTEND-SURFACE.md`](../FRONTEND-SURFACE.md) — all form fields, HTMX targets, statuses. Read before REQ-014.

Accepted REQs list automated **Tests** (`T-*`). Keep them green with `cargo test` (also run in the Docker builder stage).
