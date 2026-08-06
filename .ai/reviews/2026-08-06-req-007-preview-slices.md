# REQ-007 preview/rotate — Herdr slice plan

Parent: `feat/dm-analog-ingest` (PR #3). Implement on slice branches, parent merges.

## Flow

`queued` → `downloading` → extract → **`preview`** (worker stops; files under `ANALOG_INGEST_DIR/{job_id}/`) → user rotate/confirm → **`labeling`** → `uploading` → `done` / `failed`.

Cancel from preview: `failed` or `cancelled`, clear secure_id, delete workdir.

## Slices

| Branch | Scope |
|--------|--------|
| `feat/dm-analog-preview-slice-status` | Status `preview`; worker pause; claim post-confirm; workdir helpers |
| `feat/dm-analog-preview-slice-rotate` | `src/image_rotate.rs` — 90° CW/CCW rewrite JPEG on disk + tests |
| `feat/dm-analog-preview-slice-ui` | Preview gallery HTMX, serve bytes, rotate/confirm/cancel routes |

Merge order: status → rotate → ui (ui may land after status if routes assume `preview`).
