# UI improvement backlog

- **Date:** 2026-08-07
- **Status:** proposals only — pick into REQ-014 or later REQs
- **Contract:** [`.ai/FRONTEND-SURFACE.md`](../FRONTEND-SURFACE.md)
- **Parent REQ:** [REQ-014](../requirements/REQ-014-frontend-redesign.md)

Improvements are grouped by effort and whether they need **new backend** behavior. Prefer P0–P1 inside REQ-014; P2+ can ship later without blocking the redesign.

---

## P0 — Must land with REQ-014 (presentation only)

| ID | Improvement | Why | How (sketch) |
|----|-------------|-----|----------------|
| UI-01 | **One visual system** across home, gear, login, admin | Pages look like different products | Shared tokens: type scale, spacing scale, accent, buttons, fields, empty states |
| UI-02 | **Mobile-first tickets** | Rename + gear + convert crush on narrow screens | Stack: identity → status → actions; gear/import in progressive disclosure |
| UI-03 | **Ingest jobs as cards** (not a 5-col table) | Table unreadable on phone | Card: order, status pill, gear line, updated, primary action |
| UI-04 | **Clear home hierarchy** | First viewport feels like a dashboard dump | Brand + one lead + Aufträge primary; Analog-Import second; gear via nav only |
| UI-05 | **Ticket progressive disclosure** | Every ticket shows gear grid + import form noise | Default: name, order, status, 1–2 actions; “Ausrüstung” / “Importieren” expand |
| UI-06 | **Preview dock polish** | Film-strip idea OK, chrome inconsistent | Sticky/confirm bar; larger touch rotate; filename secondary; keep panel outside poll |
| UI-07 | **Empty states with next step** | Blank lists feel broken | CTA copy: “Auftragsnummer prüfen”, “Kamera anlegen”, “Import starten” |
| UI-08 | **German admin copy** | Admin mixes EN/DE | Align labels with user-facing German |

---

## P1 — Strong wins inside REQ-014 (small template logic OK)

| ID | Improvement | Why | How (sketch) |
|----|-------------|-----|----------------|
| UI-09 | **Show album used after queue/done** | Album find-or-create is invisible | Echo album title on job row (`job.album` or “Standard”) |
| UI-10 | **Pre-fill convert from ticket gear** | Convert form re-asks camera/lens/ISO | Already partially selected; hide free-text unless “manuell”; default album placeholder from env not needed in UI |
| UI-11 | **Status as timeline, not only pill** | dm codes opaque (`PROCESSING`) | Keep code muted; emphasize DE summary + simple steps (as order-check panel does) |
| UI-12 | **Ingest status progress** | `Download` / `Metadaten` / `Upload` feel random | Compact stepper: Warteschlange → Download → Vorschau → Metadaten → Upload → Fertig |
| UI-13 | **Preview entry affordance** | Easy to miss “Vorschau bearbeiten” | When any job is `preview`, auto-open panel or banner “N Bilder warten auf Freigabe” |
| UI-14 | **Destructive actions secondary** | Delete competes with Save/Import | Ghost/danger only in overflow or confirm; primary = check / import / confirm |
| UI-15 | **Nav for small screens** | Topbar wraps awkwardly | Compact nav: brand \| Ausrüstung \| user menu (Abmelden, Admin) |
| UI-16 | **Focus / loading feedback** | HTMX waits feel dead | Disable + spinner already partial; standardize on all mutating buttons |

---

## P2 — Nice UX (may need tiny backend / copy)

| ID | Improvement | Backend? | Notes |
|----|-------------|----------|-------|
| UI-17 | Link or plain-text “Album in PhotoPrism: …” on `done` | no if `album` stored | Deep-link only if we add PhotoPrism public URL pattern later |
| UI-18 | Relative time (“vor 3 Min”) on job `updated_at` | no | Keep absolute in `title` tooltip |
| UI-19 | Ticket count / open-count in section title | no | `Aufträge (3 offen)` |
| UI-20 | Collapse archive by default with clearer summary | no | Already `<details>`; improve summary line |
| UI-21 | Gear page: show “used by N tickets” | yes | Needs count query — defer |
| UI-22 | Confirm import summary (camera, ISO, lens, album, N files) | soft | Modal/copy before confirm; data mostly in preview head today |
| UI-23 | Keyboard: Enter submits active form predictably | no | Fix multiple buttons in `#order-form` (check vs save) |
| UI-24 | Prefetch / skeleton on 5s poll | no | Avoid full-list flash; `hx-swap` settle / morph if we accept small HTMX extension later |

---

## P3 — Product follow-ups (separate REQs — not REQ-014)

| ID | Improvement | Why separate |
|----|-------------|--------------|
| UI-25 | Discord DM when ingest `done` / `failed` | New notification behavior |
| UI-26 | Retry failed import without re-entering Secure-ID | Secure-ID currently cleared; storage/policy decision |
| UI-27 | Batch rotate (all CW) in preview | New endpoint or multi-file action |
| UI-28 | Mark ticket ↔ ingest job relationship in UI | Needs clearer join in list queries |
| UI-29 | PhotoPrism album picker (search existing) | New PP list call + UI |
| UI-30 | Light/dark or theme toggle | Product decision; current app is dark-only |
| UI-31 | PWA / install / offline shell | Out of current stack goals |
| UI-32 | JSON API + separate frontend | Explicitly out of REQ-014 |

---

## Visual direction — locked in REQ-014

| Choice | Status |
|--------|--------|
| **B — Lab bench** | **Selected** (neutral dark, one amber/gold accent, scanability over film costume) |
| A — Darkroom v2 | Rejected for this pass |
| C — Paper negative | Rejected for this pass |

## CSS tooling — locked in REQ-014

| Choice | Status |
|--------|--------|
| **Hand-authored `static/styles.css` + CSS variables** | **Selected** |
| Tailwind / Bootstrap / npm pipeline | Rejected (no Node in Docker builder; see REQ-014) |

---

## REQ-014 cut line (locked)

**In:** UI-01 … UI-16 + Lab bench + hand CSS.  
**Out (for later):** UI-17+.  
**Never in REQ-014:** UI-25 … UI-32; Tailwind.

---

## Acceptance mapping

| Backlog | REQ-014 criterion |
|---------|-------------------|
| UI-01, tokens | Shared design tokens |
| UI-02–UI-05, UI-15 | Mobile + hierarchy + disclosure |
| UI-03, UI-12 | Ingest readability |
| UI-06, UI-13 | Preview UX + poll invariant |
| UI-07 | Empty states |
| UI-08 | Admin German |
| UI-09 | Album visibility (preferred) |
