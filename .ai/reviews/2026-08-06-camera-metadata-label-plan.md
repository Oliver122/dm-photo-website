# Plan — Camera EXIF labeling for dm analog ingest

- **Date:** 2026-08-06
- **Author:** ai (worker `slice-metadata`)
- **PR / branch:** `feat/dm-analog-ingest-slice-metadata` → merge into `feat/dm-analog-ingest` (parent review)
- **Status:** proposed
- **Related:** [REQ-005](../requirements/REQ-005-dm-analog-download.md), [2026-08-06-dm-analog-photoprism-plan.md](2026-08-06-dm-analog-photoprism-plan.md)

## Context

REQ-005 pipeline: download dm analog JPEGs → stamp **camera identity** from user input → import into PhotoPrism. dm does not reliably expose which film camera was used; the user supplies a label at ingest time (e.g. `Canon AE-1`, `Leica M6`, `Nikon FM2`).

PhotoPrism indexes camera from embedded metadata, not from upload API fields alone. [EXIF extraction docs](https://docs.photoprism.app/developer-guide/metadata/exif/) show a two-stage read path:

1. **Native Go parser (Stage 1)** — reads EXIF `Make` and `Model` (IFD0) into `CameraMake` / `CameraModel`.
2. **ExifTool overlay (Stage 2, optional)** — fills only fields Stage 1 left empty; also reads XMP `tiff:Make` / `tiff:Model` from embedded XMP via ExifTool JSON.

Therefore the ingest job should **write EXIF `Make` and `Model` into each JPEG file on disk before PhotoPrism upload/import**, and touch the file mtime so a re-index picks up changes ([user guide](https://docs.photoprism.app/user-guide/library/metadata/)).

## Placement in pipeline

```mermaid
flowchart LR
  DL[Download ZIP] --> UNZ[Extract JPEGs to temp dir]
  UNZ --> STAMP[metadata_stamp step]
  STAMP --> PP[PhotoPrism upload/import]
```

Suggested module: `src/metadata_stamp.rs` (or `src/analog_metadata.rs`), called from the ingest job between extract and PhotoPrism client. Job state: `labeling` (already in REQ-005).

## User input → metadata mapping

### MVP (single field)

REQ-005 UI: one **camera label** string. Map to EXIF as follows:

| User input | EXIF `Make` | EXIF `Model` | Notes |
|------------|-------------|--------------|-------|
| `Canon AE-1` | `Canon` | `AE-1` | Split on **first** whitespace |
| `Leica M6` | `Leica` | `M6` | Same rule |
| `Holga` (one token) | `Analog` | `Holga` | Default make when no split |
| `My custom rig` | `Analog` | `My custom rig` | Full string preserved in Model |

Rules:

- Trim whitespace; reject empty label (validation error before download).
- Max length **64 chars** per EXIF string field (TIFF convention; truncate with `…` only if user exceeds — prefer hard validation error).
- ASCII-safe only for MVP (German umlauts in camera names are rare; document UTF-8 as follow-up if needed).

### Optional enhancement (same ingest form)

Add optional **Make** field; if set, use it as `Make` and put the label entirely in `Model`. Avoids bad splits (`Minolta` vs `Minolta X-700`).

### Do not rely on for PhotoPrism camera index

- PhotoPrism **labels** or **albums** via API — useful for batch grouping, but do not replace EXIF for the Cameras view.
- IPTC `By-line` / keywords — secondary; EXIF Make/Model is the primary path.
- XMP-only without EXIF — works via ExifTool Stage 2, but EXIF is simpler and hits Stage 1 directly.

## Existing file metadata policy

dm/CEWE scans often embed **scanner/lab** Make/Model (e.g. Noritsu, Fujifilm, CEWE), not the user's film camera.

**Recommended policy:** **overwrite** IFD0 `Make` and `Model` when the user supplied a camera label. Preserve other EXIF (DateTimeOriginal, Orientation, dimensions) by merge-read-then-write, not full strip.

| Tag | Action |
|-----|--------|
| `Make`, `Model` | Set from user mapping |
| `DateTimeOriginal`, `Orientation`, exposure tags | Keep if present |
| `Software` | Optional append note: `dm-photo-website ingest` (or leave untouched) |
| `ImageDescription` / `UserComment` | Optional: store raw user label if Make/Model split loses info |

Research slice `feat/dm-analog-ingest-slice-download` should sample one real ZIP and confirm baseline EXIF; adjust overwrite policy if files are EXIF-empty.

## Rust implementation options

### 1. `little_exif` (recommended primary)

- **Crate:** [little_exif](https://crates.io/crates/little_exif) (pure Rust read/write).
- **Pros:** In-process; supports JPEG; explicit `ExifTag::Make` / `ExifTag::Model`; `Metadata::new_from_path` merges with existing EXIF; fits Axum/Tokio job worker without subprocess; active releases (2026).
- **Cons:** Subset of EXIF tags; known JPEG edge cases (APP12/APP13 segments from Photoshop can shadow tags — crate provides `clear_app12_segment` / `clear_app13_segment` helpers); no embedded XMP writer.
- **Usage sketch:**

```rust
use little_exif::{exif_tag::ExifTag, metadata::Metadata};

let mut meta = Metadata::new_from_path(path)?;
meta.set_tag(ExifTag::Make(make.into()));
meta.set_tag(ExifTag::Model(model.into()));
meta.write_to_file(path)?;
// touch mtime after write for PhotoPrism re-index
```

### 2. `kamadak-exif` + `img-parts` (pure Rust, lower level)

- **Crates:** [kamadak-exif](https://docs.rs/kamadak-exif) (parse/encode EXIF payload) + [img-parts](https://crates.io/crates/img-parts) (JPEG segment surgery).
- **Pros:** Pure Rust; fine-grained control; kamadak already common for **reading** EXIF in Rust ecosystems.
- **Cons:** **No high-level JPEG write API** in kamadak alone; must assemble APP1 segment and reinsert — more code and test surface than `little_exif`; easy to corrupt JPEG if segment sizes wrong.
- **When:** Consider only if `little_exif` fails on dm JPEG fixtures in tests.

### 3. ExifTool subprocess (recommended fallback)

- **Tool:** [ExifTool](https://exiftool.org/) (`exiftool -overwrite_original -Make=… -Model=… file.jpg`).
- **Pros:** Same family PhotoPrism uses in Stage 2; battle-tested; can set XMP `tiff:Make` / `tiff:Model` in one pass (`-XMP-tiff:Make=…`); handles odd JPEG variants.
- **Cons:** External binary dependency (Docker image must install `perl` + `libimage-exiftool-perl` or standalone ExifTool); subprocess overhead; harder unit tests in CI unless ExifTool installed; error parsing stderr.
- **When:** Fallback path on `little_exif` write error; optional env `METADATA_STAMP_BACKEND=exiftool|little_exif` for homelab debugging.

### 4. Other crates (not recommended for MVP)

| Option | Verdict |
|--------|---------|
| `rexiv2` (gexiv2 bindings) | Needs system libexiv2; deployment friction in slim Docker images |
| `xmp_toolkit` / manual XMP | Overkill; EXIF Make/Model sufficient for PhotoPrism camera index |
| Re-encode JPEG with `image` crate | Destructive; drops EXIF unless carefully merged — avoid |

## Recommended approach

**Primary:** `little_exif` in a new `metadata_stamp` module — stamp IFD0 `Make` + `Model` after ZIP extract, before PhotoPrism upload.

**Fallback:** shell out to **ExifTool** when `little_exif` returns an error (log + retry once); document in `.env.example` as optional.

**Verification:** integration test with a fixture JPEG (minimal EXIF + dm-like scanner metadata); after stamp, parse with `kamadak-exif` (read-only) asserting Make/Model; manual check in PhotoPrism Cameras view on homelab.

## Proposed API (implementation later)

```rust
pub struct CameraLabel {
    pub make: String,
    pub model: String,
}

impl CameraLabel {
    pub fn from_user_label(label: &str) -> Result<Self, LabelError>;
}

pub fn stamp_camera_metadata(path: &Path, camera: &CameraLabel) -> Result<(), StampError>;
```

Config (future `.env`):

- `METADATA_STAMP_BACKEND` — `little_exif` (default) | `exiftool`
- `EXIFTOOL_PATH` — default `exiftool`

## Error handling

- Per-file failure: mark job `failed` with path + reason, or continue with partial success + warning list (decide in implementation PR; prefer **fail job** if any image fails so PhotoPrism batch is consistent).
- Non-JPEG in ZIP: skip with warning or fail (depends on download research).
- Read-only temp dir: fail fast at job start.

## Decisions (proposed)

| # | Decision |
|---|----------|
| D1 | Write **EXIF IFD0 `Make` + `Model`** before PhotoPrism import; do not rely on API-only metadata. |
| D2 | Primary library: **`little_exif`**; fallback: **ExifTool** subprocess. |
| D3 | Single user **camera label** → split on first space; default Make `Analog` for one-token labels. |
| D4 | **Overwrite** existing Make/Model; preserve other EXIF tags where possible. |
| D5 | Update file **mtime** after write so PhotoPrism re-index sees changes. |
| D6 | XMP / IPTC stamping **out of scope for MVP** unless EXIF-only fails PhotoPrism indexing in homelab test. |

## Open questions

- [ ] What EXIF do real dm analog JPEGs ship with? (blocked on download research slice + one sample ZIP)
- [ ] Are downloads always JPEG, or also TIFF/HEIC? (affects format support in `little_exif`)
- [ ] Should UI offer separate Make + Model fields now, or only after MVP feedback?
- [ ] Truncate vs reject labels longer than 64 characters?
- [ ] Store chosen camera on the SQLite job row for audit/re-run without re-prompting?
- [ ] If PhotoPrism instance has ExifTool disabled (`PHOTOPRISM_DISABLE_EXIFTOOL`), is EXIF-only still enough? (Expected yes — Stage 1 native parser reads Make/Model)
- [ ] Container image: add ExifTool to runtime for fallback, or keep image slim and fail closed on `little_exif` errors?

## Follow-ups (implementation phase)

- [ ] Add fixture JPEG(s) under `tests/fixtures/` from anonymized dm sample
- [ ] Implement `CameraLabel::from_user_label` + unit tests for split edge cases
- [ ] Implement `stamp_camera_metadata` + round-trip test with kamadak-exif read
- [ ] Wire into ingest job between extract and PhotoPrism client
- [ ] Document homelab verification: import one stamped file → Cameras view shows expected Make/Model
