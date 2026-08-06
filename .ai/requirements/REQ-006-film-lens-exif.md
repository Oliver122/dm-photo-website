# REQ-006 — Cameras, lenses, film ISO & ticket → import

- **ID:** REQ-006
- **Status:** planned

## Goal

Let users maintain **separate** libraries of **cameras** and **lenses** (not one combined preset), attach **camera + lens + film ISO** to a dm **ticket/order**, convert a ticket into an **analog import** with one action (Secure-ID prompted), and stamp camera / ISO / lens EXIF before PhotoPrism.

## Data model

### `user_cameras` (per user)

| Column | Notes |
|--------|--------|
| id, user_id | PK / FK |
| label | Display + EXIF Make/Model split (REQ-005), unique per user (trim, case-insensitive) |
| created_at | |

### `user_lenses` (per user)

| Column | Notes |
|--------|--------|
| id, user_id | PK / FK |
| name | Display name, unique per user |
| focal_mm | REAL > 0 |
| aperture | REAL > 0 (f-number, e.g. 2.4) |
| created_at | |

### `tickets` additions

- `camera_id` NULL FK → user_cameras
- `lens_id` NULL FK → user_lenses
- `film_iso` NULL INTEGER

### `analog_ingest_jobs` additions

- `camera_id` NULL, `lens_id` NULL, `film_iso` NULL INTEGER
- Keep `camera_label` TEXT (denormalized at job create from selected camera for worker/EXIF)

**ISO** lives on the **order/ticket and job** (film stock), not on camera/lens rows.

## Acceptance criteria

### Gear library page (`/gear`)

- [ ] Logged-in page: two lists — **Kameras** and **Objektive** — with add + delete.
- [ ] Camera: label only (e.g. `Canon AE-1`).
- [ ] Lens: name + Brennweite (mm) + Blende.
- [ ] Validate; German labels; scoped to `user_id`.

### Ticket attach

- [ ] On each active ticket: select camera, lens, film ISO (optional fields) + Speichern.
- [ ] Persist on ticket row.

### Convert ticket → Import

- [ ] Button **Importieren** on ticket (PhotoPrism configured).
- [ ] Opens compact form: **Secure-ID** (required) + optional album; pre-filled camera/lens/ISO from ticket (editable).
- [ ] Creates `analog_ingest_job` (`queued`) with denormalized `camera_label` + FKs/ISO; does not require retyping order number.
- [ ] Blocked if a `done` job exists for that order (same as today) unless deleted first.

### Ingest form (home)

- [ ] Prefer selects for camera + lens from library + Film-ISO; free-text camera label allowed as fallback if no cameras yet.
- [ ] Job list shows compact gear: `Canon AE-1 · ISO 400 · 50mm f/2.4`.

### EXIF (labeling stage)

- [ ] Stamp Make/Model from camera label.
- [ ] Stamp ISO when set; FocalLength + FNumber from lens when set.
- [ ] Same values for every frame in the batch.

## Defaults / decisions

- Secure-ID is **not** stored on the ticket long-term; entered at convert time.
- Combined named “presets” **out of scope** (replaced by separate camera + lens entities).

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| T-006-a | +/− | ISO / focal / aperture accept and reject | parser |
| T-006-b | + | Stamp ISO + FocalLength + FNumber on JPEG | `camera_exif` |
| T-006-c | + | Camera/lens CRUD + unique per user | `db` |
| T-006-d | + | Ticket gear save + convert creates job | `db` / handler |
| T-006-e | − | Duplicate camera/lens name rejected | `db` |
| ST-006-a | − | Convert without Secure-ID → 400 | `system_tests` |

- [ ] T-006-* / ST-006-*

## Out of scope

- Per-frame ISO/lens inside one ZIP.
- Shared gear across users.
- Storing Secure-ID on tickets.

## Touches

- `migrations/0008_*.sql` (and maybe 0009)
- `src/camera_exif.rs`, `db.rs`, `models.rs`, `jobs/analog_ingest.rs`
- handlers: gear, tickets, analog_ingest; templates; `ROUTES-AND-DATA.md`

## Depends on

- REQ-002, REQ-005, REQ-007 (preview before upload still applies)
