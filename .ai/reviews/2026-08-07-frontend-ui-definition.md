# UI definition — Lab bench (REQ-014)

- **Date:** 2026-08-07
- **Status:** revised after senior review — **frozen for implement**
- **REQ:** [REQ-014](../requirements/REQ-014-frontend-redesign.md)
- **Contract:** [FRONTEND-SURFACE.md](../FRONTEND-SURFACE.md)
- **Backlog:** [2026-08-07-ui-improvement-backlog.md](2026-08-07-ui-improvement-backlog.md)
- **Senior review:** [2026-08-07-frontend-ui-definition-senior-review.md](2026-08-07-frontend-ui-definition-senior-review.md) (Approve with changes → Must-fix addressed below)
- **Stack:** Askama + HTMX + hand CSS (`static/styles.css`) — **no Tailwind**

This file is the **design source of truth** for implementation. Do not invent layouts without updating this doc.

---

## 1. Product voice

- Language: **German** (user + admin).
- Tone: calm lab / workbench — precise, short labels, no marketing fluff.
- Brand: **dm-photo** is the hero on logged-out; logged-in brand stays in topbar with **visible wordmark** at ≤640px (not icon-only). Page title carries the job (“Deine Aufträge”).

---

## 2. Design tokens

Implement as `:root` CSS variables.

| Token | Role | Target |
|-------|------|--------|
| `--bg` | Page ground | `#121411` |
| `--bg-raised` | Topbar / sticky | `#1a1c18` |
| `--surface` | Section wells (sparingly) | `#22251f` |
| `--fg` | Primary text | `#e6e4dc` |
| `--fg-secondary` | Supporting | `#b0ada3` |
| `--muted` | Meta / placeholders | `#858278` |
| `--line` | Dividers | `#34382f` |
| `--accent` | CTA / focus / order nums | `#d4a853` (locked — keep gold) |
| `--accent-hover` | CTA hover | `#e0b65e` |
| `--accent-soft` | Soft accent wells | `rgba(212, 168, 83, 0.14)` |
| `--ok` | Done / ready | `#6db87a` |
| `--ok-soft` | Soft OK wells | `rgba(109, 184, 122, 0.14)` |
| `--bad` | Errors / delete hover | `#e07070` |
| `--bad-soft` | Error wells | `rgba(224, 112, 112, 0.14)` |
| `--warn` | In-progress ingest | `#d4a853` (same as accent) |
| `--warn-soft` | Soft warn wells | `rgba(212, 168, 83, 0.12)` |
| `--radius` | Controls | `4px` |
| `--max` | Content width | `720px` |
| `--space-1…5` | 4 / 8 / 12 / 16 / 24 px | scale only |
| `--font` | UI | Source Sans 3 |
| `--font-display` | H1 / brand | Source Serif 4 |
| `--mono` | Orders, Secure-ID, paths | IBM Plex Mono |

**Motion (2–3 only):** (1) button disabled + opacity on HTMX request, (2) image replace after rotate via cache-bust `t=` (optional short opacity fade — no CSS `rotate()` fighting the server image), (3) 150ms focus ring on inputs. No scroll-jacking, no `scrollIntoView` on preview load, no page-load fireworks.

---

## 3. Shell + stable DOM targets

```
┌─────────────────────────────────────────┐
│ brand(mark + "dm-photo")     nav…       │  topbar --bg-raised
├─────────────────────────────────────────┤
│ main (max 720px, pad space-4)           │
│   page-head | sections | footer details │
└─────────────────────────────────────────┘
```

**Nav (desktop):** `Ausrüstung` · `username` · `Abmelden` · (`Admin` if admin).  
**Nav (≤640px):** brand (mark + wordmark) left; `Ausrüstung` + `<details class="nav-more">` for user / Abmelden / Admin. Native `<details>` only; summary ≥44px tap; no focus-trap library — close on navigate is enough.

### Stable IDs (must keep wired)

| ID | Page | Role |
|----|------|------|
| `#order-out` | home | Order check / manual ticket HTML |
| `#tickets-list` | home | Tickets partial (+ OOB) |
| `#analog-ingest-list` | home | Job list (poll `every 5s`) |
| `#analog-preview-panel` | home | Preview dock — **outside** poll target |
| `#dm-out` | home footer | Discord test result |
| `#gear-cameras-list` | gear | Camera list (+ OOB) |
| `#gear-lenses-list` | gear | Lens list (+ OOB) |
| `#gear-cameras-out` | gear | Camera create notice/error (`aria-live`) |
| `#gear-lenses-out` | gear | Lens create notice/error (`aria-live`) |
| `#refresh-out` | admin | Admin action feedback |
| `#users-body` / `#user-{id}` | admin | User rows |

**Do not:** gear CRUD on home; Discord test in first viewport.

---

## 4. Screens

### 4.1 Logged-out `/` and `/login`

Same composition: brand (display), one sentence purpose, one CTA `Mit Discord anmelden`. Soft atmospheric ground OK — not purple. No tickets/stats.

### 4.2 Home `/` (logged-in)

**Page head:** omit costume eyebrow (or muted `Labor`). H1 `Deine Aufträge`. Lead: Discord pickup sentence + link `/gear`.

**Section A — Aufträge** (primary)

1. `#order-form.order-form`
   - `label` (optional, maxlength 80)
   - `order_number` (required, `pattern=\d{6}-\d{6}`, `inputmode=numeric`)
   - Primary: `Status prüfen` — **`type="submit"`** (Enter submits this)
   - Secondary: `Nur speichern` — `type="button"` + explicit `hx-post="/api/tickets"`
2. `#order-out`
3. `#tickets-list`

**Empty tickets:** “Noch keine Aufträge” + “Oben Auftragsnummer eingeben und Status prüfen.”

**Section B — Analog-Import** (secondary)

If `photoprism_configured`:

1. Note: Secure-ID ~6 weeks.
2. `.ingest-form` fields:
   | Field | Required | Notes |
   |-------|----------|-------|
   | `order_number` | yes | same pattern |
   | `secure_id` | yes | `minlength=8` `maxlength=8` |
   | `camera_id` | yes if user has cameras | `<select required>` |
   | `camera_label` | yes if **no** cameras | free text maxlength 64; **do not show** when cameras exist |
   | `lens_id` | no | select |
   | `film_iso` | no | `min=1` `max=102400` |
   | `album` | no | maxlength 128 |
3. `#analog-ingest-list` (`hx-trigger="load, every 5s"`)
4. `#analog-preview-panel` sibling below

If PP off: single note that import is disabled — section stays visually secondary (pickup-only users).

**Empty jobs:** “Noch keine Import-Aufträge” + “Formular ausfüllen oder aus einem Auftrag importieren.”

**Poll note:** full list replace OK in REQ-014 (no morph extension). Keep card min-heights stable; do not auto-open ticket `<details>` on poll. Ticket list OOB refresh **resets details closed** — acceptable.

**Footer:** `<details class="dev-panel">` Discord test → `#dm-out`.

### 4.3 Ticket anatomy

**Collapsed (default):**

```
┌ ticket ──────────────────────────────────┐
│ Name                          [pill]     │
│ 544850-103396  (mono, accent)            │
│ Status text (DE); wire code muted        │
│ meta: erstellt / aktualisiert            │
│ [Name ………][Speichern]   <details More>   │
│ ▸ Ausrüstung     ▸ Importieren           │
└──────────────────────────────────────────┘
```

**Pills (active list only):** `In Bearbeitung` (not completed) · `Abholbereit` (completed).  
**Archive block:** separate `<details class="archive">` summary `Archiv · N` — tickets inside use muted treatment; pill may say `Abholbereit` (no third live state named Archiv on active rows).

**More menu:** native `<details class="ticket-more">` summary “Mehr”/⋯ — contains **Löschen** with DE `hx-confirm`. No JS menu library.

**Rename:** always-visible compact row (input `label` + Speichern → `POST …/label`).

**▸ Ausrüstung** (`<details class="ticket-disclose">`, closed default): `camera_id`, `lens_id`, `film_iso` → `POST …/gear`.

**▸ Importieren** (only if `photoprism_configured`, closed default):

| Field | Required | Notes |
|-------|----------|-------|
| `secure_id` | yes | 8 chars |
| `album` | no | |
| `camera_id` / `lens_id` / `film_iso` | no | prefilled from ticket |
| `camera_label` | only if user has **no** cameras | no separate “Manuell” toggle when library non-empty |

Submit → `#analog-ingest-list`.

### 4.4 Ingest job card (`.ingest-job`)

```
┌ job ─────────────────────────────────────┐
│ 123456-123456              [status chip] │
│ gear line (or camera_label)              │
│ Album: {title} | Standard (wenn leer)    │
│ stepper · aktualisiert …                 │
│ [Vorschau bearbeiten?]  [Löschen?]       │
└──────────────────────────────────────────┘
```

- **Stepper (DE):** Warteschlange → Download → Vorschau → Metadaten → Upload → Fertig | Fehler.
- Wire classes stay `.status-queued` etc.
- **Löschen** only if `can_delete`: `queued` \| `preview` \| `done` \| `failed`. Hide during `downloading` \| `labeling` \| `uploading`. DE `hx-confirm`.
- **Vorschau bearbeiten** only when `status=preview` (primary).
- Failed: `error_text` in `--bad-soft` block.
- **No `<table>`** for jobs at any breakpoint.
- Preview waiting: list banner/CTA “Bilder warten auf Freigabe” (not auto-focus).

### 4.5 Preview dock

- Head: order + file count; `#preview-feedback` with `aria-live="polite"`.
- Strip: plain **dark rail** (no film sprocket holes). Scroll-x + snap; 44px rotate buttons.
- Rotate: POST → replace panel HTML; images use `t=` cache-bust (prefer image replace over CSS rotate desync).
- Footer: sticky on mobile OK; primary `Import bestätigen` (`hx-confirm` DE), secondary `Abbrechen` (`hx-confirm` DE) → list refresh + **panel cleared OOB**.
- **UI-13 locked:** persistent list CTA; optional one-shot silent `hx-get` into empty panel **without** `scrollIntoView` or focus steal. User must tap confirm/rotate.

### 4.6 Gear `/gear`

- Kameras: form `label` → `POST /api/gear/cameras` → OOB `#gear-cameras-list`.
- Objektive: `name`, `focal_mm`, `aperture` → OOB `#gear-lenses-list`.
- Delete → OOB respective list.
- Empty cameras: “Noch keine Kameras” + “Anlegen für EXIF beim Import.”
- Empty lenses: “Noch keine Objektive” + “Optional für Brennweite/Blende.”

### 4.7 Admin

Same tokens. German labels + DE confirms:

| Action | Label | Confirm (if any) |
|--------|-------|------------------|
| refresh | Alle offenen Aufträge aktualisieren | — |
| simulate | Test-Ticket erzeugen (999999-999999) | — |
| delete all tickets | Alle Tickets löschen | DE destructive confirm |
| users table | Nutzer | |
| delete user | Nutzer löschen | DE confirm |
| logout | Admin abmelden | — |

Feedback → `#refresh-out`. Users: stacked rows ≤640px; row id `#user-{id}`.

---

## 5. Component inventory

| Class | Role |
|-------|------|
| `.topbar` `.brand` `.nav-more` | Shell |
| `.page-head` `.section-title` `.home-section` | Structure |
| `.order-form` `.ingest-form` | Home forms |
| `button` + `.secondary` `.ghost` `.danger` `.small` | Actions |
| `.field` | Label + control |
| `.pill` `.pill-open` `.pill-done` | Ticket state (active) |
| `.ticket` `.ticket-disclose` `.ticket-more` | Ticket + details |
| `.archive` | Archived tickets block |
| `.ingest-job` `.ingest-stepper` | Job cards |
| `.status` `.status-*` | Status chips (wire values) |
| `.film-strip` `.film-frame` | Preview strip (flattened rail) |
| `.empty-state` `.error` `.success` `.notice` | Feedback |
| `.dev-panel` | Discord test |
| `.panel` | Legacy admin wells OK |

---

## 6. Client constraints (all mutating UIs)

From surface validation + confirms:

- Order: `pattern=\d{6}-\d{6}`
- Secure-ID: length 8
- Film ISO: `1..=102400`
- Focal / aperture: `> 0` on gear form
- DE `hx-confirm` on: ticket delete, ingest delete, preview confirm/cancel, admin wipe tickets, admin delete user
- Mutating buttons: `hx-disabled-elt` + visible indicator (spinner / “Bitte warten …”)

---

## 6b. Action feedback contract (required)

Every user-triggered mutation must give **visible feedback**. Silent success or silent failure is a bug.

### Rules

1. **Pending:** disable triggering control(s) + show indicator for that action.
2. **Success:** HTML update in a known target — list refresh, `.notice` / `.success`, status chip change, or preview panel content. Prefer `aria-live="polite"` on feedback slots.
3. **Failure:** `.error` (or `#…-out` with `.error`) with **German** message; HTTP 4xx/5xx still returns HTML the user can read in the HTMX target (not an empty swap).
4. **Confirm:** destructive / irreversible actions use DE `hx-confirm` before request.
5. **Stable feedback slots** (keep or add in templates):

| Slot | Used by |
|------|---------|
| `#order-out` | L1 check / save — errors + status |
| `#tickets-list` | L1/L2 success via OOB or full replace |
| `#analog-ingest-list` | L3/L4/L6 + poll status changes |
| `#analog-preview-panel` | L5 gallery / clear on cancel |
| `#preview-feedback` | L5 rotate/live messages (`aria-live`) |
| `#gear-cameras-out` | camera create notice/error (`aria-live`) |
| `#gear-lenses-out` | lens create notice/error (`aria-live`) |
| `#dm-out` | Discord test |
| `#refresh-out` | admin actions |

### Per-loop feedback matrix

| Loop | Pending | Success signal | Negative / error signal |
|------|---------|----------------|-------------------------|
| L1 check | order spinner | status panel in `#order-out` + tickets OOB | `#order-out` `.error` (bad format, upstream) |
| L1 save | order spinner | notice + tickets OOB | `#order-out` `.error` |
| L2 gear | disable Speichern | tickets list refresh | `.error` in response / list |
| L3 convert | disable submit | job appears in ingest list | `.error` (Secure-ID, conflict, PP off) |
| L4 ingest create | disable submit | job card queued | `.error` (validation, PP off, conflict) |
| L5 rotate | rotate spinner | panel refresh + new `t=` | `#preview-feedback` or panel `.error` |
| L5 confirm/cancel | disable buttons | list status change; panel clear on cancel | `.error` + confirm already gated |
| L6 delete job | disable + confirm | job gone; panel clear if needed | `.error` / 404 |
| L7 gear create | disable submit | `.notice` in `#gear-*-out` + list OOB | `.error` in `#gear-*-out` |
| L7 gear delete | confirm | list refresh | `.error` / 404 |
| L8 admin | spinner on refresh | `#refresh-out` message | `#refresh-out` `.error` |

### Negative cases the UI must surface (not only HTTP status)

- Invalid order number
- Missing / short Secure-ID
- Empty camera label / empty lens name
- Invalid focal / aperture / ISO
- PhotoPrism not configured (ingest/convert)
- Duplicate done import / conflict
- Not found (wrong id / other user’s resource)
- Admin wrong password (login page error)

Automated coverage: REQ-014 **−** system tests (`ST-014-n*`) assert status **and** `.error` / German cue in body.

---

## 7. Interaction loops

### L1 — Track order

```
Fill order (+ optional name) → Enter/Status prüfen → POST /api/order/check
  → #order-out + #tickets-list OOB
Alt: Nur speichern → POST /api/tickets (no dm API)
Pickup-only: stop here; wait Discord; rename OK
```

### L2 — Gear on ticket

```
▸ Ausrüstung → POST …/gear → #tickets-list refresh (details reset closed)
```

### L3 — Ticket → import

```
▸ Importieren (PP on) → POST …/convert → #analog-ingest-list → L4/L5
```

### L4 — Direct ingest

```
.ingest-form → POST /api/analog/ingest → poll 5s → preview → L5
```

### L5 — Preview → PhotoPrism

```
CTA/banner → GET …/preview → #analog-preview-panel
  rotate loop → panel refresh + t=
  confirm → labeling → uploading → done (album on card)
  OR cancel → failed + panel clear OOB
```

**Invariant:** panel never inside `#analog-ingest-list`.

### L6 — Redo import

```
DELETE …/ingest/:id (can_delete only) → list refresh + preview clear if that job
  → L3 or L4 again
```

### L7 — Gear library

```
/gear add/delete → OOB lists → home selects on next tickets render
```

### L8 — Admin

```
refresh | simulate | delete-all → #refresh-out
delete user → #user-{id} swap
```

---

## 8. Process loops

### P-Loop (design) — done this pass

Author → senior review → revise Must-fix → **freeze** (this revision).

### I-Loop (implement)

| Slice | Delivers | Exit check |
|-------|----------|------------|
| I1 | Tokens, shell, auth, nav-more | 360px topbar wordmark; tab order smoke |
| I2 | Tickets + L1/L2 disclosure | ST-014-b; **grep surface `name=` on tickets** |
| I3 | Ingest cards, stepper, album, can_delete | no table; **grep ingest form `name=`** |
| I4 | Preview dock + UI-13 CTA | poll invariant; no focus steal |
| I5 | Gear + admin DE | empty states; DE confirms |
| I6 | Loading polish, delete weight, feedback audit | full R-Loop + `cargo test st_014` |

Deferred (not I3 scope): HTMX morph / poll flicker (backlog UI-24).

### R-Loop (every slice / before merge)

```
cargo test
+ L1 → L2 → L3/L5 or L4 → L5 → L6
+ pickup-only: L1 only
+ PP-off home glance
```

---

## 9. Locked decisions (was §8 open questions)

| # | Decision |
|---|----------|
| 1 | UI-13: list CTA + optional silent fill; **no** auto scroll/focus |
| 2 | Rename always visible compact |
| 3 | Preview rail **flat** (no sprocket holes) |
| 4 | Accent **gold** `#d4a853` |
| 5 | Pickup-only = L1; Analog stays secondary |

---

## 10. Out of definition

- Tailwind / npm / SPA
- New API fields
- Light theme / PhotoPrism deep links
- Discord DM on ingest done
- HTMX morph extensions (UI-24)
