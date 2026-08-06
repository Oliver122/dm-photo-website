# Requirements index

Open by **REQ id** — do not dump this folder into context. See [`README.md`](README.md).

| ID | Summary | Status | File |
|----|---------|--------|------|
| REQ-001 | Discord OAuth + admin password auth | accepted | [REQ-001-auth.md](REQ-001-auth.md) |
| REQ-002 | Order check, tickets, Discord DMs | accepted | [REQ-002-orders-tickets.md](REQ-002-orders-tickets.md) |
| REQ-003 | Admin dashboard and tooling | accepted | [REQ-003-admin.md](REQ-003-admin.md) |
| REQ-004 | SQLite data, migrations, agent constraints | accepted | [REQ-004-data-migrations.md](REQ-004-data-migrations.md) |
| REQ-005 | DM analog download → camera EXIF → PhotoPrism | accepted | [REQ-005-dm-analog-download.md](REQ-005-dm-analog-download.md) |
| REQ-006 | Film ISO + lens/Objektiv EXIF on analog ingest | planned | [REQ-006-film-lens-exif.md](REQ-006-film-lens-exif.md) |
| REQ-007 | Ingest preview + rotate before PhotoPrism import | planned | [REQ-007-ingest-preview-rotate.md](REQ-007-ingest-preview-rotate.md) |
| REQ-010 | Container image (Dockerfile) | planned | [REQ-010-container-image.md](REQ-010-container-image.md) |
| REQ-011 | Artifactory build/push script + env | planned | [REQ-011-artifactory-push.md](REQ-011-artifactory-push.md) |
| REQ-012 | App Compose (run from Artifactory) | planned | [REQ-012-app-compose.md](REQ-012-app-compose.md) |
| REQ-013 | JFrog server checklist (ops-owned) | planned | [REQ-013-jfrog-server.md](REQ-013-jfrog-server.md) |

Accepted REQs list automated **Tests** (`T-*`). Keep them green with `cargo test` (also run in the Docker builder stage).
