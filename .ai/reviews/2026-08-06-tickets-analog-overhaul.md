# Plan — Tickets + Analog Integration Overhaul

- **Date:** 2026-08-06
- **Status:** in progress
- **Parent branch:** `feat/dm-analog-ingest`

## Problem

Tickets (migrations 0002–0005) and analog ingest (0006) were merged hastily. Backend domains are fine separately, but naming, schema idempotency, jobs module layout, home UX, and `.ai` docs are inconsistent.

## Approach

Cleanup foundation, then unify home UX. Keep two tables / two workers. Herdr composer-2.5 workers on slice branches; parent merges in order.

| Slice branch | Goal |
|--------------|------|
| `feat/dm-analog-overhaul-slice-schema` | `0007` partial unique on done; handler retry semantics |
| `feat/dm-analog-overhaul-slice-db-api` | Rename analog DB helpers; `album` field end-to-end |
| `feat/dm-analog-overhaul-slice-jobs` | Split `src/jobs.rs` → `src/jobs/` |
| `feat/dm-analog-overhaul-slice-docs` | Sync `.ai` + REQ-005 status |
| `feat/dm-analog-overhaul-slice-ui` | Fold analog into home `.stack` panels |

**Merge order:** schema → db-api → jobs → docs → ui.

## Non-goals

No PhotoPrism WebDAV, no single merged table, no Discord DM on analog completion, no deploy redesign.
