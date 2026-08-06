# REQ-006 — Herdr slices (separate camera / lens)

Parent: `feat/dm-analog-ingest`. Merge order: **schema → exif → gear-ui → ticket-convert**.

| Slice | Branch | Scope |
|-------|--------|--------|
| schema | `feat/dm-analog-gear-slice-schema` | migrations, models, db CRUD cameras/lenses, ticket+job columns |
| exif | `feat/dm-analog-gear-slice-exif` | ISO/focal/aperture stamp + validation helpers |
| gear-ui | `feat/dm-analog-gear-slice-gear-ui` | `/gear` page CRUD HTMX |
| ticket-convert | `feat/dm-analog-gear-slice-ticket-convert` | ticket gear form, Importieren+Secure-ID, ingest selects, worker stamp |

Secure-ID: prompt at convert only (not stored on ticket).
