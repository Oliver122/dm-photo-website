# REQ-006 — Film ISO & lens (Objektiv) EXIF

- **ID:** REQ-006
- **Status:** planned

## Goal

When ingesting dm analog scans (REQ-005), let the user set **film ISO** and **lens / Objektiv** data (e.g. `50mm` + `f/2.4`) so PhotoPrism indexes sensible EXIF beyond camera Make/Model.

Example form inputs:

- ISO: `400` (or `800`, `1600`, …)
- Focal length: `50` (mm)
- Aperture: `2.4` (written as F-number / f-stop)

## Acceptance criteria

### Capture

- [ ] Ingest create form accepts optional **ISO**, **focal length (mm)**, **aperture (f-number)**.
- [ ] Persist values on `analog_ingest_jobs` (nullable columns; empty = do not stamp that tag).
- [ ] Validate: ISO positive integer in a sensible range (e.g. 1–102400); focal length > 0; aperture > 0.
- [ ] German field labels: Film-ISO, Brennweite (mm), Blende (z.B. 2.4).

### EXIF stamp (before PhotoPrism upload)

- [ ] Stamp ISO → EXIF photographic sensitivity / ISO speed ratings.
- [ ] Stamp focal length → EXIF `FocalLength` (mm).
- [ ] Stamp aperture → EXIF `FNumber` (and related aperture tag if the crate supports it cleanly).
- [ ] Apply same values to every image in the batch (roll-level metadata).
- [ ] Combine with existing camera Make/Model stamp (REQ-005); do not clear unrelated tags.

### UX / jobs

- [ ] Values shown on job status list (compact: `ISO 400 · 50mm f/2.4` when set).
- [ ] Worker uses stored columns during `labeling` (or equivalent) step.

## Tests

| ID | Case | Where |
|----|------|--------|
| T-006-a | Parse/validate ISO, focal mm, aperture (accept/reject) | `camera_exif` or small parser module |
| T-006-b | Stamp writes ISO + FocalLength + FNumber on a fixture JPEG | `camera_exif` (+ tempfile JPEG) |
| T-006-c | Migration adds nullable columns; create job persists them | `db` tests |

- [ ] T-006-a … T-006-c

## Out of scope

- Per-frame different ISO/lens inside one ZIP.
- Full EXIF editor (GPS, date overrides, etc.).
- Auto-detect lens from CEWE ZIP metadata (unless later proven available).

## Touches

- Migration `0008_…` on `analog_ingest_jobs`
- `src/camera_exif.rs` (or sibling), `src/db.rs`, `src/models.rs`
- `handlers/analog_ingest.rs`, `jobs/analog_ingest.rs`, templates
- REQ-005 form / pipeline

## Depends on

- REQ-005 (analog ingest pipeline)
