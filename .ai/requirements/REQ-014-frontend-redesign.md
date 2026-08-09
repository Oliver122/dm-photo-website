# REQ-014 — Frontend redesign (capability-preserving)

- **ID:** REQ-014
- **Status:** planned (implemented on `feat/req-014-lab-bench-ui` — accept after review)
- **Depends on (contract):** [`.ai/FRONTEND-SURFACE.md`](../FRONTEND-SURFACE.md) — **read first**
- **UI definition (frozen):** [`.ai/reviews/2026-08-07-frontend-ui-definition.md`](../reviews/2026-08-07-frontend-ui-definition.md) — screens, tokens, L1–L8 + I1–I6
- **Senior FE review:** [`.ai/reviews/2026-08-07-frontend-ui-definition-senior-review.md`](../reviews/2026-08-07-frontend-ui-definition-senior-review.md)
- **Idea backlog:** [`.ai/reviews/2026-08-07-ui-improvement-backlog.md`](../reviews/2026-08-07-ui-improvement-backlog.md)
- **Supersedes visually:** REQ-009 variant A look (behavior stays)

## Goal

Replace the current UI presentation (home, gear, auth, admin). Keep every API option and HTMX flow from `FRONTEND-SURFACE.md`. Ship a coherent, mobile-first design that is easy to scan.

---

## Stack decision (locked)

| Layer | Choice | Why |
|-------|--------|-----|
| Templates | **Askama** HTML | Already canonical; fragments + OOB |
| Interactivity | **HTMX** (`static/htmx.min.js` only) | Matches handlers; no SPA |
| CSS | **Hand-authored design system** in `static/styles.css` | No Node in Docker/Cargo build today; CSS is inlined via `base.html` |
| Tokens | **CSS custom properties** (`:root`) | One accent/type/spacing system across pages |
| Components | **Semantic class names** (`.ticket`, `.ingest-job`, `.btn`, …) | Stable for HTMX partials; readable in Rust templates |
| Icons | **Inline SVG or CSS-only** (no icon font CDN required) | Offline-friendly self-host |
| Fonts | Keep **Google Fonts** link in `base.html` *or* self-host later | Source Serif / Source Sans / IBM Plex Mono OK; no Inter/Roboto/Arial as hero |

### Explicitly not using

| Rejected | Reason |
|----------|--------|
| **Tailwind CSS** | Needs npm or standalone CLI + purge pipeline; Docker builder is Rust-only (`cargo test` / `cargo build`). Utility classes also fight Askama partial reuse and current include-of-one-CSS-file model. |
| Bootstrap / DaisyUI / similar | Heavy defaults, fights custom darkroom look, extra deps |
| React / Vue / Svelte SPA | Out of project conventions |
| CSS-in-JS | No JS app runtime |
| Tailwind CDN / Play CDN | Not for production; no purge, FOUC, policy risk |

### When to reconsider Tailwind

Only with a **new REQ** that adds a documented frontend build step (e.g. `tailwindcss` CLI in Docker builder, checked-in `static/app.css` output, CI cache). Not in REQ-014.

### CSS file shape (implementation target)

```
static/styles.css
  :root { /* color, type, space, radius, line */ }
  /* reset / base */
  /* layout: topbar, main, page-head */
  /* components: btn, field, pill, empty-state */
  /* tickets / ingest / preview / gear / admin */
  /* @media mobile */
```

Optional later split (`tokens.css`, `components.css`) only if the single file becomes unmaintainable — still no bundler required if Askama can include one entry file.

---

## Visual direction (locked)

**Lab bench** (backlog option B): neutral dark grey surfaces, **one** sharp accent (warm amber/gold — keep brand continuity, not purple), dense readable type, less “film theme” decoration than REQ-009 A. Preview may keep a horizontal strip, but chrome stays plain lab/workbench — not a costume.

Anti-patterns (same as before): purple gradients, cream+terracotta cliché, broadsheet density, multi-layer neon glow, pill-stat dashboards.

---

## Scope cut (from backlog)

**In (must):** UI-01 … UI-08 (P0), UI-09 … UI-16 (P1).  
**Opportunistic:** UI-12 ingest stepper, UI-09 album on job row.  
**Out of this REQ:** UI-17+ product follow-ups (DM on ingest, retry Secure-ID, batch rotate, album picker, theme toggle, PWA, SPA).

---

## Design constraints (must)

1. **No capability loss** vs `FRONTEND-SURFACE.md` §§3–5.
2. Askama + HTMX + hand CSS only (stack table above).
3. Invariants: `#analog-preview-panel` outside `#analog-ingest-list` poll; rotate cache-bust `t=`; ~44px touch targets; German user copy.
4. Mobile-first ≤400px: no full-page horizontal scroll (preview strip may scroll-x).
5. One primary action per section; progressive disclosure for ticket gear/import.
6. Shared tokens used by home, gear, login, admin.

---

## Information architecture

| Surface | Role |
|---------|------|
| `/` | Aufträge + Analog-Import; preview under import |
| `/gear` | Cameras / lenses only (link from nav + home lead) |
| Auth | Brand-first, single Discord CTA |
| `/admin` | Same tokens; German labels |

Dev Discord test: collapsed footer/`<details>`.

---

## Acceptance criteria

### Contract

- [ ] All `FRONTEND-SURFACE` form fields and HTMX targets still wired.
- [ ] `photoprism_configured` still gates ingest + convert.
- [ ] Preview survives 5s poll; rotate updates orientation.
- [ ] Ingest statuses still show DE labels; `can_delete` actions unchanged.

### Visual / UX

- [ ] Tokenized `styles.css`; no Tailwind/Bootstrap artifacts.
- [ ] Home first viewport: brand/title, short purpose, primary order action.
- [ ] Tickets: identity + status first; gear/import disclosed.
- [ ] Ingest: card/stack layout on small screens (not squeezed table).
- [ ] Album title visible on job when set (UI-09).
- [ ] Empty states with next-step CTA.
- [ ] Admin UI German.
- [ ] Design note: `.ai/reviews/YYYY-MM-DD-frontend-redesign.md` (confirms Lab bench + stack).

### Tooling

- [ ] `cargo test` / Docker builder unchanged (no Node stage).
- [ ] No `package.json` / Tailwind config added by this REQ.

### Action feedback

- [ ] Every L1–L8 mutation shows pending (disable + indicator) and a success or `.error` outcome in a documented slot (see UI definition §6b).
- [ ] Negative paths return readable German HTML in the HTMX target (not blank).
- [ ] Gear create success uses `.notice` in `#gear-cameras-out` / `#gear-lenses-out` (already patterned).
- [ ] Feedback slots use `aria-live` where they are dedicated outs (`gear-*-out`, preview feedback).

### Quality

- [ ] Manual smoke (+): login → order check → gear → convert/ingest → preview rotate → confirm; admin refresh.
- [ ] Manual smoke (−): bad order format, empty Secure-ID, empty camera, PP-off ingest message, wrong admin password.

## Tests

Positive **and** negative. Negatives must assert **HTTP status + user-visible error cue** (`.error` and/or known German fragment).

| ID | +/- | Case | Where |
|----|-----|------|--------|
| T-014-a | + | Full suite green after UI work | `cargo test` |
| ST-014-a | + | Preview panel outside poll target | templates review |
| ST-014-b | + | Mobile ticket/ingest usable ≤400px | browser |
| ST-014-c | + | Surface form `name=` attrs still present | grep / review |
| ST-014-d | + | No Tailwind/npm toolchain | repo review |
| ST-014-p1 | + | Create camera → 200 + notice + list OOB cue | `src/system_tests.rs` |
| ST-014-n1 | − | Order check invalid format → 400 + `.error` | `src/system_tests.rs` |
| ST-014-n2 | − | Convert empty Secure-ID → 400 + error cue | `src/system_tests.rs` (extends ST-006-a) |
| ST-014-n3 | − | Create camera empty label → `.error` in `#gear-cameras-out` | `src/system_tests.rs` |
| ST-014-n4 | − | Create lens invalid aperture → `.error` in `#gear-lenses-out` | `src/system_tests.rs` |
| ST-014-n5 | − | Delete other user’s ticket → 404 + `.error` | `src/system_tests.rs` |
| ST-014-n6 | − | Analog ingest HTMX anonymous → 401 | `src/system_tests.rs` (alias ST-008-e) |
| ST-014-n7 | − | Ingest/convert when PP unset → error HTML (503/unavailable cue) | `src/system_tests.rs` |
| ST-014-n8 | − | Admin login wrong password → no admin + error cue | `src/system_tests.rs` (ST-008-i) |

Manual (−) still required for preview rotate failure paths and confirm cancels (hard to fully fake without fixtures).

## Out of scope

- Tailwind or any npm-based CSS pipeline.
- New backend features (ingest DM, retry without Secure-ID, etc.).
- EXIF / PhotoPrism upload semantics changes.
- Parallel A/B variant branches.

## Touches (expected)

- `static/styles.css`, `templates/**`
- Light handler HTML wrappers only if OOB/target IDs change
- `.ai/FRONTEND-SURFACE.md` if IDs/fields change
- `.ai/reviews/` design note
- `.ai/CONVENTIONS.md` (stack note — already aligned)

## Depends on

- REQ-001–REQ-007, REQ-009 (accepted behavior)
- `FRONTEND-SURFACE.md`, UI improvement backlog
