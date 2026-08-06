# REQ-012 — App Compose

- **ID:** REQ-012
- **Status:** planned

## Goal

Run the published image via Compose with env file, SQLite volume, and pull from Artifactory — not a local registry stack.

## Acceptance criteria

- [x] `deploy/app/docker-compose.yml` pulls `image: ${ARTIFACTORY_DOCKER_REGISTRY}/dm-photo-website:${TAG}`.
- [x] `env_file: .env` (or documented path).
- [x] Volume for SQLite at `/data`; `DATABASE_URL=sqlite:///data/app.db`.
- [x] Port mapping `8080:8080`.
- [x] Document Discord redirect URI note when host ≠ localhost.

## Out of scope

- Shipping a registry Compose stack in this repo.
- TLS termination for Artifactory.
- Installing Artifactory (REQ-013).

## Touches

- `deploy/app/docker-compose.yml`
- Deploy docs / README notes

## Depends on

- REQ-010, REQ-011 (image available in Artifactory)
- REQ-013 (registry reachable for pull)
