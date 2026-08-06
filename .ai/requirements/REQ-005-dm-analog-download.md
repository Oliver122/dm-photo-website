# REQ-005 — DM analog download → PhotoPrism

- **ID:** REQ-005
- **Status:** planned

## Goal

Let a logged-in user submit dm analog **download credentials** (12-digit order number + Secure-ID), automatically download the photo ZIP from dm/CEWE, stamp **camera** metadata onto the images, and ingest them into a configured **PhotoPrism** instance.

Example credentials (from the slip / [download page](https://foto.dm.de/fotos/analog/download.html?ofqrcode=true)):

- Order: `544850-103396` (`^\d{6}-\d{6}$`)
- Secure-ID: `H5GGX3T5` (alphanumeric; exact length/charset TBD from live API)

## Acceptance criteria

### Ingest job

- [ ] Accept order number + Secure-ID (+ user-chosen camera label) via HTMX UI and API.
- [ ] Validate order format (`^\d{6}-\d{6}$`); validate Secure-ID against observed dm rules once researched.
- [ ] Background job downloads the analog pack (ZIP or file list) using the real dm/CEWE download endpoint (not browser automation unless unavoidable).
- [ ] Persist job state in SQLite: queued / downloading / labeling / uploading / done / failed (+ error text).
- [ ] Do not store Secure-ID longer than needed for the active job (prefer encrypt-at-rest or delete after success).
- [ ] Idempotent per `(user, order_number)`: re-submit of a completed order is rejected or explicitly “re-run”.

### Camera metadata

- [ ] Before PhotoPrism import, write camera identity into image metadata (EXIF Make/Model and/or XMP) from the user-supplied camera label (and optional make/model split).
- [ ] PhotoPrism indexes show the camera after import/index.

### PhotoPrism

- [ ] Configurable base URL + auth (app password or session token) via env.
- [ ] Upload/import via PhotoPrism REST (`/api/v1`, upload token POST then PUT) **or** documented fallback (WebDAV / import folder + index trigger).
- [ ] Optional album name / UID for the ingest batch.

### UX

- [ ] Form on site: order number, Secure-ID, camera label, optional album.
- [ ] Status visible in UI (HTMX poll or refresh partial).
- [ ] Clear German error copy for bad credentials / expired download (dm links ~6 weeks).

## Out of scope

- Reverse-engineering exploits or bypassing dm paywalls.
- Full PhotoPrism admin UI mirror.
- Automatic discovery of camera from dm (unless the download API exposes it later).
- CEWE myPhotos account linking.

## Touches

- New module(s): dm analog download client, metadata stamp, PhotoPrism client
- Handlers + templates for ingest form/status
- `jobs` worker loop extension
- Migrations for ingest jobs table
- `.env`: PhotoPrism URL/credentials, optional download temp dir

## Depends on

- REQ-001 (logged-in user)
- REQ-004 (migrations / SQLite rules)

## Research / branch slices

Parent branch: `feat/dm-analog-ingest` (integration; human/parent agent merges).

| Slice branch | Topic |
|--------------|--------|
| `feat/dm-analog-ingest-slice-download` | dm/CEWE download API research |
| `feat/dm-analog-ingest-slice-metadata` | EXIF/XMP camera labeling plan |
| `feat/dm-analog-ingest-slice-photoprism` | PhotoPrism API research |

Implementation PRs stack onto the parent **after** research merges — do not start product code until research notes are reviewed.
