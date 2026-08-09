# Frontend surface — technical contract

**Purpose:** Canonical inventory of what the UI **must** expose and call. Design work (REQ-014) derives layout/visuals from this sheet — do not invent new product capabilities here; do not drop fields that handlers already accept.

**Authority order when docs disagree:** handler form structs in `src/handlers/*` → this file → templates → `ROUTES-AND-DATA.md`.

**Stack constraint:** Askama HTML + HTMX (`static/htmx.min.js`). No SPA framework unless a future REQ explicitly changes it. Most mutating endpoints return **HTML fragments** (not JSON), often with **out-of-band (OOB)** swaps.

---

## 1. Pages (full HTML documents)

| Route | Auth | Template | Role |
|-------|------|----------|------|
| `GET /` | public | `templates/index.html` | Logged-out CTA **or** logged-in home (tickets + analog ingest + preview + optional gear link) |
| `GET /login` | public | `login.html` | Discord login |
| `GET /gear` | user | `gear.html` | Camera + lens library |
| `GET /admin/login` | public | `admin_login.html` | Admin password form |
| `GET /admin` | admin | `admin.html` | Ticket tools + user table |
| `GET /auth/discord` | public | redirect | Start OAuth |
| `GET /auth/discord/callback` | public | redirect | Finish OAuth (`code`, `state` query) |
| `POST /logout` | public | redirect | Clear user session |
| `POST /admin/login` | public | form → redirect | `password` |
| `POST /admin/logout` | public | redirect | Clear admin flag |
| `GET /static/*` | public | files | CSS, HTMX |

Shell chrome (`templates/base.html`): brand → home, nav links (`/gear`, username, logout, optional `/admin`), `<main>` content. CSS inlined from `static/styles.css`.

---

## 2. Auth model (UI implications)

| Actor | How | UI must |
|-------|-----|---------|
| Anonymous | no session | Show login CTA; hide tickets/ingest/gear |
| User | Discord OAuth session | Home + gear; all `/api/*` user routes |
| Admin | same session + `ADMIN_PASSWORD` flag | Extra `/admin` + `/api/users` |

JSON helper (not primary UI): `GET /api/me` → current user JSON (user auth).

---

## 3. HTMX DOM targets (must remain stable or migrate carefully)

Stable element IDs used as `hx-target` / OOB anchors. Redesign may restyle, but **must not orphan** these without updating every caller:

| ID | Page | Fed by | Notes |
|----|------|--------|-------|
| `#order-out` | home | order check / manual ticket | Inline status + notices |
| `#tickets-list` | home | SSR + OOB after ticket mutations | Full tickets partial |
| `#analog-ingest-list` | home | SSR + poll `every 5s` + ingest mutations | Job list only |
| `#analog-preview-panel` | home | preview GET / rotate | **Must stay outside** poll target |
| `#dm-out` | home (dev) | test DM | Collapsed details OK |
| `#gear-cameras-list` | gear | SSR + OOB | Camera list |
| `#gear-lenses-list` | gear | SSR + OOB | Lens list |
| `#gear-cameras-out` | gear | camera create | `.notice` / `.error` (`aria-live`) |
| `#gear-lenses-out` | gear | lens create | `.notice` / `.error` (`aria-live`) |
| `#refresh-out` | admin | admin ticket actions | Feedback |
| `#users-body` / `#user-{id}` | admin | delete user | Row swap |

Action feedback (pending / success / `.error`) and negative HTMX HTML: [reviews/2026-08-07-frontend-ui-definition.md](reviews/2026-08-07-frontend-ui-definition.md) §6b + REQ-014 `ST-014-n*`.

---

## 4. User API — forms, options, responses

Content-Type for mutations unless noted: `application/x-www-form-urlencoded` (HTML forms / HTMX).

### 4.1 Orders & tickets

#### `POST /api/order/check`

| Field | Required | Constraints | Notes |
|-------|----------|-------------|-------|
| `order_number` | yes | `\d{6}-\d{6}` | Trimmed; invalid → 400 HTML error |
| `label` | no | max ~80 in UI | Empty → ignored |

**Behavior:** Calls dm spot API; creates/updates ticket; returns HTML status panel + ticket notice; OOB refreshes `#tickets-list`.

#### `POST /api/tickets`

Same form as order check (`order_number`, optional `label`). **No** dm API call — manual save only. HTML + OOB tickets list.

#### `POST /api/tickets/:id/label`

| Field | Required | Constraints |
|-------|----------|-------------|
| `label` | no* | Trim; empty clears / placeholder UX |

OOB / full tickets list refresh.

#### `DELETE /api/tickets/:id`

No body. Confirm in UI recommended. Owner only. Refreshes tickets list.

#### `POST /api/tickets/:id/gear`

| Field | Required | Constraints | Notes |
|-------|----------|-------------|-------|
| `camera_id` | no | existing `user_cameras.id` or empty | Empty clears |
| `lens_id` | no | existing `user_lenses.id` or empty | Empty clears |
| `film_iso` | no | integer `1..=102400` or empty | Empty clears |

Persists gear on ticket for later convert. Returns tickets list (OOB pattern).

#### `POST /api/tickets/:id/convert`

Queues analog ingest from a ticket (Secure-ID at submit). Requires PhotoPrism configured.

| Field | Required | Constraints | Notes |
|-------|----------|-------------|-------|
| `secure_id` | yes | length 8 | From dm Beleg |
| `album` | no | max 128 | PhotoPrism album title; server find-or-create |
| `camera_id` | no | gear id | Prefer over free-text when set |
| `camera_label` | no | max 64 | Used if no camera_id / no cameras |
| `lens_id` | no | gear id | |
| `film_iso` | no | `1..=102400` | May inherit ticket ISO |

**Errors (HTML):** already-done import, missing Secure-ID, validation, PhotoPrism off. Success targets `#analog-ingest-list`.

**Ticket list UX data (SSR partial):** for each ticket — `id`, `label`, `order_number`, `completed`, `summary_state_code`, `summary_state_text`, `created_at`, `last_updated`, selected `camera_id` / `lens_id` / `film_iso`, archive partition (`completed_before(7)`).

---

### 4.2 Gear library (`/gear`)

#### `POST /api/gear/cameras`

| Field | Required | Constraints |
|-------|----------|-------------|
| `label` | yes | trim; unique per user (case-insensitive) |

→ HTML + OOB `#gear-cameras-list`.

#### `DELETE /api/gear/cameras/:id`

Owner only. OOB camera list.

#### `POST /api/gear/lenses`

| Field | Required | Constraints |
|-------|----------|-------------|
| `name` | yes | unique per user |
| `focal_mm` | yes | parse float `> 0` |
| `aperture` | yes | parse float `> 0` (f-number) |

→ HTML + OOB `#gear-lenses-list`.

#### `DELETE /api/gear/lenses/:id`

Owner only. OOB lens list.

---

### 4.3 Analog ingest

Job statuses (wire value → German label for UI):

| Status | DE label | User actions |
|--------|----------|--------------|
| `queued` | Warteschlange | Delete |
| `downloading` | Download | (wait; no delete) |
| `preview` | Vorschau | Open preview, delete |
| `labeling` | Metadaten | (wait) |
| `uploading` | Upload | (wait) |
| `done` | Fertig | Delete (allows re-import) |
| `failed` | Fehler | Delete; show `error_text` |

`can_delete`: `queued` \| `preview` \| `done` \| `failed` only.

#### `GET /api/analog/ingest`

HTML partial job list. Home polls: `hx-trigger="load, every 5s"`.

Row fields: `order_number`, gear line or `camera_label`, status (+ DE label), `updated_at`, optional `error_text`, preview CTA when `preview`, delete when `can_delete`.

#### `POST /api/analog/ingest`

| Field | Required | Constraints | Notes |
|-------|----------|-------------|-------|
| `order_number` | yes | `\d{6}-\d{6}` | |
| `secure_id` | yes | length 8 | Cleared after successful import |
| `camera_id` | conditional | gear id | Required path when user has cameras (UI) |
| `camera_label` | conditional | max 64 | Required when no cameras in library |
| `lens_id` | no | gear id | |
| `film_iso` | no | `1..=102400` | |
| `album` | no | max 128 | Falls back to `PHOTOPRISM_DEFAULT_ALBUM` |

Disabled entirely when PhotoPrism not configured (`photoprism_configured` flag on home).

#### `DELETE /api/analog/ingest/:id`

Deletes job + workdir when `can_delete`. Allows re-import of same order. Clears preview panel via OOB when appropriate.

#### Preview (status must be `preview`, owner only)

| Method | Path | Body / query | Response |
|--------|------|--------------|----------|
| `GET` | `/api/analog/ingest/:id/preview` | — | HTML gallery → `#analog-preview-panel` |
| `GET` | `/api/analog/ingest/:id/preview/file` | `path` (rel under workdir), optional `t` cache-bust | JPEG/image bytes |
| `POST` | `/api/analog/ingest/:id/preview/rotate` | `file`, `direction`=`cw`\|`ccw` | HTML gallery (RAM rotate; flush on confirm) |
| `POST` | `/api/analog/ingest/:id/preview/confirm` | — | → `labeling`; refresh job list (+ clear preview OOB) |
| `POST` | `/api/analog/ingest/:id/preview/cancel` | — | → `failed`; wipe workdir; refresh list |

**Invariant:** Preview panel **must not** live inside `#analog-ingest-list` (poll would destroy in-progress rotate UI).

---

### 4.4 Discord test

#### `POST /api/dm/me`

No body. Sends configured test DM to current user’s Discord id. HTML into `#dm-out`. Keep out of primary flow (collapsed).

---

## 5. Admin API

| Method | Path | Body | UI |
|--------|------|------|-----|
| `POST` | `/admin/tickets/refresh` | — | HTML → `#refresh-out` |
| `DELETE` | `/admin/tickets` | — | confirm; wipe all tickets |
| `POST` | `/admin/tickets/simulate` | — | creates done ticket `999999-999999` |
| `GET` | `/api/users` | — | JSON list (admin page uses SSR table today) |
| `DELETE` | `/api/users/:id` | — | remove row `#user-{id}` |

Admin login form: `password` → `POST /admin/login`.

---

## 6. Server-driven UI flags / context

Passed into templates (not form fields):

| Flag / data | Source | UI effect |
|-------------|--------|-----------|
| `current_user` | session | Logged-in vs CTA |
| `is_admin` | session | Show Admin nav |
| `photoprism_configured` | env PhotoPrism | Show/hide ingest + ticket convert |
| `cameras` / `lenses` | DB | Selects vs free-text camera; gear page lists |
| `tickets` / `archived_tickets` | DB | Active vs archive `<details>` |
| Job `gear_line` | derived | Compact camera/lens/ISO string in ingest table |

---

## 7. External systems (UI copy only — not called from browser)

Browser never talks to these directly; server proxies:

| System | User-facing inputs | Outcome visible in UI |
|--------|--------------------|----------------------|
| dm spot order API | `order_number` | Status stepper / codes / ticket updates |
| CEWE analog download | `order_number` + `secure_id` | Ingest job progress |
| PhotoPrism | `album` (+ stamped EXIF from gear) | Job `done` / `failed`; album find-or-create server-side |
| Discord | OAuth + DM | Login; pickup / test notifications |

---

## 8. Validation cheat-sheet (client + server)

| Value | Rule |
|-------|------|
| Order number | `NNNNNN-NNNNNN` (12 digits + hyphen) |
| Secure-ID | exactly 8 chars |
| Film ISO | integer `1..=102400` |
| Focal mm / aperture | finite `> 0` |
| Camera label | non-empty; EXIF Make/Model split on first whitespace |
| Album title | optional; empty → env default; server ensures album exists |
| Ticket label | optional display name |

Prefer matching `pattern` / `min` / `max` / `maxlength` in HTML to cut round-trips; server remains authoritative.

---

## 9. Known UX debt (inputs to REQ-014 — not new APIs)

Prioritized idea list: [`reviews/2026-08-07-ui-improvement-backlog.md`](reviews/2026-08-07-ui-improvement-backlog.md).

These are **presentation** problems; the options above already exist:

1. Home packs tickets + gear controls + convert + ingest + preview into one dense scroll — hard to scan on mobile.
2. Ingest job table is desktop-table shaped; status/actions compete.
3. Ticket foot: rename, delete, gear grid, convert `<details>` feel bolted on.
4. Visual language from REQ-009 variant A (film-strip) landed but still reads unfinished / inconsistent across home, gear, admin, login.
5. Admin remains English-mixed and panel-card heavy vs German user home.
6. Album success is invisible in UI (no link/confirmation that PhotoPrism album was created).
7. Polling every 5s refreshes whole job list — redesign must keep preview isolation and avoid layout jump.

---

## 10. Non-goals for the surface itself

- New REST JSON SPA API (unless a later REQ).
- Browser → PhotoPrism / CEWE credentials.
- Multi-user sharing of tickets/gear.
- Changing ingest state machine or EXIF rules (those stay REQ-005/006/007).

When adding a field or route, update **this file** and `ROUTES-AND-DATA.md` in the same change.
