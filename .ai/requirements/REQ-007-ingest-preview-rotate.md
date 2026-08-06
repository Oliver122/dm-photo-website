# REQ-007 — Ingest preview & rotate before PhotoPrism

- **ID:** REQ-007
- **Status:** planned

## Goal

After CEWE download + ZIP extract (REQ-005), show a **preview** of the scanned frames so the user can **rotate** images (90° steps) before EXIF stamping and PhotoPrism upload. Import must not proceed until the user confirms.

## Acceptance criteria

### Job flow

- [ ] New status (or equivalent): after download/extract → `preview` (waiting for user); then `labeling` → `uploading` → `done` / `failed`.
- [ ] Background worker **stops** at preview (does not auto-upload).
- [ ] User action **Confirm import** resumes the job (stamp metadata including REQ-006 fields → PhotoPrism).
- [ ] User may **cancel** a preview job (mark failed/cancelled; scrub secure_id; delete workdir).
- [ ] Only the owning user can preview / rotate / confirm / cancel.

### Preview UI

- [ ] HTMX (or fragment) gallery of extracted images for the job (thumbnails or scaled previews).
- [ ] Per-image rotate controls: 90° CW / 90° CCW (optional 180°).
- [ ] Rotation persisted on disk (rewrite JPEG with correct orientation / pixels) before confirm so PhotoPrism sees upright images.
- [ ] German copy: Vorschau, Drehen, Import bestätigen, Abbrechen.
- [ ] Works on mobile (touch-friendly buttons; no desktop-only gestures required).

### Serving previews

- [ ] Authenticated route(s) to serve preview bytes for a job image (no public unauthenticated file leak).
- [ ] Paths stay under the job workdir; reject `..` / escape.

## Tests

| ID | Case | Where |
|----|------|--------|
| T-007-a | Rotate helper applies 90° and is invertible (4× → identity) | image util module |
| T-007-b | Preview path resolution rejects traversal | handler or util |
| T-007-c | Job status transition: queued→…→preview; confirm → labeling path (db/unit) | `db` / jobs |

- [ ] T-007-a … T-007-c

## Out of scope

- Crop, filters, color correction.
- Drag-and-drop reorder into PhotoPrism albums beyond existing album field.
- Client-side-only rotation without rewriting files (must survive upload).

## Touches

- Migration if new status values / per-file rotation table needed (prefer filesystem + job status first)
- `jobs/analog_ingest.rs` (pause/resume)
- New handlers + templates/partials for preview gallery
- Image crate for rotate (e.g. `image`)
- Routes in `main.rs` + `.ai/ROUTES-AND-DATA.md`

## Depends on

- REQ-005 (download + extract + upload pipeline)
- REQ-006 (stamp ISO/lens during post-confirm labeling — same step as camera)
