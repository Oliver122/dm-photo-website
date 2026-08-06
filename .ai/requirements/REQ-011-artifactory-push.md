# REQ-011 — Artifactory push

- **ID:** REQ-011
- **Status:** planned

## Goal

Provide a script that builds the app image and pushes it to JFrog Artifactory using env-based registry config (no hardcoded host URL).

## Acceptance criteria

- [ ] `scripts/build-and-push.sh` exists and fails loudly if required registry env is missing.
- [ ] Flow: `docker login` → `docker build` → tag → `docker push`.
- [ ] Env contract:

| Env | Required | Purpose |
|-----|----------|---------|
| `ARTIFACTORY_DOCKER_REGISTRY` | yes | e.g. `artifactory.example.com/docker-local` |
| `ARTIFACTORY_USER` | yes for push | login user |
| `ARTIFACTORY_TOKEN` / `ARTIFACTORY_PASSWORD` | yes for push | identity token preferred |
| `IMAGE_NAME` | no | default `dm-photo-website` |
| `TAG` | no | default git short SHA; also push `latest` if `PUSH_LATEST=1` |

- [ ] Image tagged as `$ARTIFACTORY_DOCKER_REGISTRY/$IMAGE_NAME:$TAG`.
- [ ] Docs use placeholders only (e.g. `$ARTIFACTORY_HOST/docker-local/dm-photo-website`).

## Out of scope

- CI wiring (may call this script later).
- Installing/configuring Artifactory (REQ-013).
- Local registry Compose in this repo.

## Touches

- `scripts/build-and-push.sh`
- README / deploy notes for env vars

## Depends on

- REQ-010 (image buildable)
- REQ-013 (Artifactory reachable with push credentials)
