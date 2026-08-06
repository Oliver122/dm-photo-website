# REQ-009 — Home UI overhaul (simpler; two style variants)

- **ID:** REQ-009
- **Status:** planned
- **Decision:** Parent merges one chosen variant after you pick A or B.

## Goal

The logged-in home (tickets + analog import + preview) feels **overbuilt**. Overhaul for **clarity and fewer UI layers**, keep all existing capabilities (order check, tickets, analog ingest, preview rotate/confirm/cancel, delete/redo).

Deliver **two parallel visual variants** (Herdr workers). You compare them and pick one for `feat/dm-analog-ingest`.

## Problems to fix (must land in both variants)

- [ ] Preview must **not** live inside the 5s HTMX poll target (already fixed on parent — do not re-nest).
- [ ] Rotate must rewrite the file **and** show the new orientation (cache-bust `t=` on img URLs; spinner while rotating).
- [ ] Visible feedback: disable buttons + short indicator while HTMX requests run.
- [ ] Delete / redo imports remain obvious for terminal + preview + queued jobs.
- [ ] Fewer nested cards/panels; one clear job per section.

## Simplicity rules (both variants)

1. **One composition** on first viewport: brand/title, short purpose, primary actions — not a dashboard of widgets.
2. **Two main sections only** on home when logged in: **Aufträge** (tickets) and **Analog-Import** (form + job list + preview).
3. Dev Discord test stays in a collapsed `<details>` or footer — not competing with main flow.
4. Preview: simple gallery (image + two rotate controls + confirm/cancel). No card chrome stacks, no floating badges.
5. Prefer CSS variables; **avoid** purple gradients, cream+terracotta cliché, dense broadsheet, heavy multi-shadow “AI UI”.
6. Expressive but purposeful fonts (not Inter/Roboto/Arial as hero). Mobile-first touch targets (~44px).
7. Keep Askama + HTMX; no new SPA framework.

## Variant briefs

| Variant | Branch | Direction |
|---------|--------|-----------|
| **A — Film strip** | `feat/dm-analog-ui-variant-a` | Darkroom / contact-sheet: charcoal or deep olive ground, warm highlight, mono for order numbers, preview as a horizontal strip of frames |
| **B — Clear desk** | `feat/dm-analog-ui-variant-b` | Light, airy “work desk”: lots of whitespace, single accent (ink blue or forest), typographic hierarchy, list-first tickets, preview as a clean vertical stack |

Both must keep German copy and the same routes/handlers (CSS + template structure only unless a tiny HTML tweak is required for the layout).

## Acceptance criteria

- [ ] Variant A and B each commit a coherent home + preview look on their branch.
- [ ] Rotate + preview panel survive list polling; images update after rotate.
- [ ] `cargo test` green on each branch.
- [ ] Short note in `.ai/reviews/` describing the variant (3–5 lines).
- [ ] Parent (or you) picks one; loser branch is abandoned; winner merges to `feat/dm-analog-ingest`.

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| ST-009-a | + | Preview panel outside poll target (manual / code review) | templates |
| ST-009-b | + | Rotate returns 200 and new cache_bust (manual) | preview UI |
| T-009-a | + | Existing system tests still pass | `cargo test` |

- [ ] T-009-a; ST-009-* manual on chosen variant

## Out of scope

- REQ-006 gear library / ticket→import convert (separate).
- Admin dashboard redesign.
- New JS framework, dark-mode toggle product, animation libraries.

## Touches

- `static/styles.css`, `templates/**`, maybe light `partials` structure
- Do not break `src/app.rs` routes or PhotoPrism/session auth

## Depends on

- REQ-002, REQ-005, REQ-007 (preview)
