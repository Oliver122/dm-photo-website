# Decision: Container + JFrog requirements (specs only)

- **Date:** 2026-08-06
- **Author:** ai (worker `reqs`)
- **PR / branch:** local / `feat/container-reqs`
- **Status:** accepted (requirements split); implementation planned

## Context

Plan: containerize the app and push to ops-owned JFrog Artifactory. Agents should read small REQ files by ID instead of a monolith `.ai/REQUIREMENTS.md`.

## Decisions

- Split product requirements into `.ai/requirements/REQ-001…004`; keep `.ai/REQUIREMENTS.md` as a thin pointer to `_index.md`.
- Add planned specs REQ-010…013 for Dockerfile, Artifactory push script, app Compose, and ops JFrog checklist.
- **JFrog is ops-owned** — no local registry Compose in this repo; registry host/credentials via env only.
- Implementation of Dockerfile / push / `deploy/app` follows in separate work after these specs.

## Follow-ups

- [ ] Implement REQ-010 (Dockerfile + static path fix)
- [ ] Implement REQ-011 (`scripts/build-and-push.sh`)
- [ ] Implement REQ-012 (`deploy/app/docker-compose.yml`)
- [ ] Ops complete REQ-013 checklist on server
