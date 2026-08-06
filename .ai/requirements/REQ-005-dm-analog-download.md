# REQ-005 — DM analog download → PhotoPrism

- **ID:** REQ-005
- **Status:** accepted

## Goal

Let a logged-in user submit dm analog **download credentials** (12-digit order number + Secure-ID), automatically download the photo ZIP from dm/CEWE, stamp **camera** metadata onto the images, and ingest them into a configured **PhotoPrism** instance.

Example credentials (from the slip / [download page](https://foto.dm.de/fotos/analog/download.html?ofqrcode=true)):

- Order: `544850-103396` (`^\d{6}-\d{6}$`)
- Secure-ID: `H5GGX3T5` (exactly 8 ASCII alphanumeric characters)

## Acceptance criteria

### Ingest job

- [x] Accept order number + Secure-ID (+ user-chosen camera label) via HTMX UI and API.
- [x] Validate order format (`^\d{6}-\d{6}$`); validate Secure-ID (8 alphanumeric chars, `dm_analog::validate_secure_id`).
- [x] Background job downloads the analog pack (ZIP) using the CEWE `api.cewe-myphotos.com` endpoint.
- [x] Persist job state in SQLite: queued / downloading / labeling / uploading / done / failed (+ error text).
- [x] Do not store Secure-ID longer than needed for the active job (`clear_analog_ingest_secure_id` after success; not encrypted at rest while queued).
- [x] Idempotent per `(user, order_number)`: re-submit of a completed order is rejected (`find_done_analog_ingest_job` + partial unique index on `status = 'done'`).

### Camera metadata

- [x] Before PhotoPrism import, write camera identity into image metadata (EXIF Make/Model) from the user-supplied camera label (`camera_exif::stamp_camera_label`).
- [ ] PhotoPrism indexes show the camera after import/index (implemented in pipeline; verify manually on target instance).

### PhotoPrism

- [x] Configurable base URL + auth (app password + user UID) via env — see `.env.example`.
- [x] Upload/import via PhotoPrism REST (`photoprism.rs`: stage upload POST then import commit).
- [x] Optional album name for the ingest batch (`album` column + `PHOTOPRISM_DEFAULT_ALBUM` fallback).

### UX

- [x] Form on site: order number, Secure-ID, camera label, optional album (`templates/index.html`).
- [x] Status visible in UI (HTMX poll on `#analog-ingest-list`).
- [x] Clear German error copy for bad credentials / expired download / already imported.

## Out of scope

- Reverse-engineering exploits or bypassing dm paywalls.
- Full PhotoPrism admin UI mirror.
- Automatic discovery of camera from dm (unless the download API exposes it later).
- CEWE myPhotos account linking.

## Touches (implemented)

- `dm_analog.rs` — CEWE download client
- `camera_exif.rs` — metadata stamp
- `photoprism.rs` — PhotoPrism client
- `handlers/analog_ingest.rs` + templates/partials
- `jobs/analog_ingest.rs` — worker loop
- Migrations `0006_analog_ingest.sql`, `0007_analog_ingest_partial_unique.sql`
- `.env`: `PHOTOPRISM_*`, `ANALOG_INGEST_DIR`

## Depends on

- REQ-001 (logged-in user)
- REQ-004 (migrations / SQLite rules)

## Research / branch slices

Parent branch: `feat/dm-analog-ingest` (integration).

| Slice branch | Topic |
|--------------|--------|
| `feat/dm-analog-ingest-slice-download` | dm/CEWE download API research |
| `feat/dm-analog-ingest-slice-metadata` | EXIF/XMP camera labeling plan |
| `feat/dm-analog-ingest-slice-photoprism` | PhotoPrism API research |

Research slices informed the implementation; product code lives on the parent / follow-on branches.
