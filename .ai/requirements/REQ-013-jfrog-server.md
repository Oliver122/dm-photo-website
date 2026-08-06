# REQ-013 — JFrog server (ops-owned)

- **ID:** REQ-013
- **Status:** planned

## Goal

Ops owns JFrog Artifactory on the server. This repo does **not** ship a registry Compose stack; apps push/pull via env-configured registry endpoints.

## Acceptance criteria (human checklist)

- [ ] Artifactory installed and reachable from build machine.
- [ ] Docker (local) repository created (e.g. `docker-local`).
- [ ] User/token with push/pull on that repo.
- [ ] TLS or insecure-registry documented for the Docker daemon on build + pull hosts.
- [ ] Image path convention: `<host>/<repo-key>/dm-photo-website:<tag>`.

## Out of scope

- Harbor / Distribution / Docker Hub as primary registry.
- Artifactory install scripts or Compose in this repository.
- TLS termination setup for Artifactory (ops).

## Touches

- None in-repo (ops/server only). Consumers: REQ-011, REQ-012 env vars.

## Depends on

- None (blocks push/pull until done)
