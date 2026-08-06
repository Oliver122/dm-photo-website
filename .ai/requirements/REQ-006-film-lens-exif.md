# REQ-006 — Film ISO, lens EXIF & saved gear presets

- **ID:** REQ-006
- **Status:** planned

## Goal

When ingesting dm analog scans (REQ-005), let the user set **camera**, **film ISO**, and **lens / Objektiv** (e.g. `50mm` + `f/2.4`) for EXIF — and **save that combination once under a display name** so it can be **reselected** on later imports without retyping.

Example preset name: `Canon AE-1 · Portra 400 · 50mm f/1.8`

Preset fields:

- Display name (required, unique per user)
- Camera label (→ Make/Model, REQ-005)
- ISO (e.g. `400`)
- Focal length mm (e.g. `50`)
- Aperture f-number (e.g. `2.4`)

## Acceptance criteria

### Capture (per ingest job)

- [ ] Ingest create form accepts **camera label** plus optional **ISO**, **focal length (mm)**, **aperture (f-number)**.
- [ ] Persist values on `analog_ingest_jobs` (nullable ISO/focal/aperture; empty = do not stamp that tag).
- [ ] Validate: ISO positive integer (e.g. 1–102400); focal length > 0; aperture > 0; camera label non-empty when stamping camera.
- [ ] German labels: Kamera, Film-ISO, Brennweite (mm), Blende (z.B. 2.4).

### Named presets (save once, reselect)

- [ ] User can **save** the current form values as a named preset (`analog_gear_presets` or equivalent), scoped to `user_id`.
- [ ] Display name required; unique per user (case-insensitive trim).
- [ ] Dropdown / select lists the user’s presets; choosing one **fills** camera, ISO, focal length, aperture (and does not submit the ingest by itself).
- [ ] User can **update** an existing preset (same name overwrite or explicit “Speichern”) and **delete** a preset.
- [ ] Presets survive across sessions; not shared between users.
- [ ] Optional: “Als Preset speichern” next to the ingest form after filling fields once.

### EXIF stamp (before PhotoPrism upload)

- [ ] Stamp camera Make/Model from camera label (existing REQ-005 path).
- [ ] Stamp ISO → EXIF photographic sensitivity / ISO speed ratings.
- [ ] Stamp focal length → EXIF `FocalLength` (mm).
- [ ] Stamp aperture → EXIF `FNumber` (related tags if crate supports cleanly).
- [ ] Apply same values to every image in the batch (roll-level metadata).
- [ ] Do not clear unrelated existing tags.

### UX / jobs

- [ ] Job status list shows compact gear line when set: `Canon AE-1 · ISO 400 · 50mm f/2.4` (or preset name if linked).
- [ ] Worker uses job columns during `labeling` (after REQ-007 confirm if that ships).

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| T-006-a | +/− | ISO / focal / aperture accept and reject | parser |
| T-006-b | + | Stamp ISO + FocalLength + FNumber on JPEG | `camera_exif` |
| T-006-c | + | Job persists ISO/lens/camera columns | `db` |
| T-006-d | + | Preset create / list / delete | `db` |
| T-006-e | − | Duplicate preset name per user rejected | `db` |
| ST-006-a | + | Authenticated create ingest with preset fields (when implemented) | `system_tests` |
| ST-006-b | − | Invalid ISO on create → 400 (when implemented) | `system_tests` |

- [ ] T-006-a … T-006-e, ST-006-*

## Out of scope

- Per-frame different ISO/lens inside one ZIP.
- Full EXIF editor (GPS, date overrides, etc.).
- Shared / global presets across all users.
- Auto-detect lens from CEWE ZIP metadata.

## Touches

- Migrations: job columns + `analog_gear_presets` table
- `src/camera_exif.rs`, `src/db.rs`, `src/models.rs`
- `handlers/analog_ingest.rs` (+ preset routes), templates
- `jobs/analog_ingest.rs`
- `.ai/ROUTES-AND-DATA.md` when implemented

## Depends on

- REQ-005 (analog ingest pipeline)
