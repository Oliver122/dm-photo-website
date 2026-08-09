# Senior frontend review — UI definition (REQ-014)

- **Reviewer:** senior frontend (Herdr worker)
- **Date:** 2026-08-07
- **Verdict:** Approve with changes

## Summary

IA, Lab-bench tokens, and L1–L8 match a real Askama/HTMX workbench — hierarchy and preview isolation are right. Freeze is blocked by a corrupted ticket diagram, undefined tokens/classes, incomplete form-field inventories (especially Analog create + gear outs), and thin a11y / `can_delete` / confirm specs. Fix Must-fix, answer §8 into the definition, then freeze.

## Strengths

- Clear home split: Aufträge primary, Analog secondary, gear off-home — matches UI-04 / REQ-014 IA.
- Preview dock sibling + L5 invariant is correct and non-negotiable; called out repeatedly.
- Ticket progressive disclosure (Ausrüstung / Importieren) is the right mobile fix for UI-02/UI-05.
- Ingest cards + stepper (no table) solve the worst current breakpoint failure (UI-03/UI-12).
- Component inventory reuses existing class names — good for partial churn.
- Process slices I1–I6 are shippable; R-Loop path covers the critical happy path.
- German voice + calm workbench tone fits the product; logged-out hero rules are clean.

## Must-fix before implement

| ID | Location | Problem | Fix |
|----|----------|---------|-----|
| M1 | §4.3 ticket ASCII | Diagram is corrupted (mojibake / duplicated rename line). | Redraw clean collapsed/expanded anatomy; no broken glyphs. |
| M2 | §2 tokens vs §4.4 | `--bad-soft` used; not in token table. `--warn` has no hex. | Add `--bad-soft` (+ optional `--warn` hex) or drop the name. |
| M3 | §4.2 Section B | “Ingest create form” — no field list. Easy to drop `camera_label` / album / ISO. | Enumerate fields + conditionals (`cameras` empty → `camera_label`; else `camera_id` required). |
| M4 | §4.4 job card | Löschen shown without `can_delete` rules. | Spec: show delete only for `queued\|preview\|done\|failed`; hide during `downloading\|labeling\|uploading`. |
| M5 | §4.3 `⋮` menu | Overflow menu undefined for HTMX (no JS menu lib). | Spec native `<details class="ticket-more">` (or delete only inside Importieren/Ausrüstung details); keep `hx-confirm`. |
| M6 | §4.3 pills | Pill `Archiv` vs archive `<details>` partition unclear vs `completed` → Abholbereit. | Active: In Bearbeitung / Abholbereit; archive section tickets reuse Abholbereit or muted “Archiv”-context — don’t invent a third live state. |
| M7 | §5 / screens | `#gear-cameras-out` / `#gear-lenses-out`, `#dm-out`, admin `#refresh-out` / `#users-body` under-specified. | Add stable IDs + feedback slots to shell/screens inventory (surface §3). |
| M8 | §6 loops | No `hx-confirm` / validation (`pattern`, Secure-ID 8, ISO range) called out. | Add one “Client constraints” bullet: HTML attrs from surface §8 + DE confirms on ticket/ingest/user wipe. |
| M9 | §4.1 / L1 | Dual buttons both `type="button"` today — Enter behavior undefined (UI-23). | Spec: primary = Status prüfen on Enter (one `type="submit"` or documented default); secondary stays explicit click. |
| M10 | §4.5 / UI-13 | Auto `hx-get` left open; implementers will guess. | Lock decision from §8 Q1 into definition before I4 (see Answers). |

## Should-fix

| ID | Location | Problem | Fix |
|----|----------|---------|-----|
| S1 | §4.5 preview | Sticky footer + scroll-x strip + keyboard/focus after rotate swap unstated. | Spec: sticky confirm bar; `aria-live` on feedback line (keep); no `scrollIntoView` hijack; 44px rotates already OK. |
| S2 | L4 / backlog §9.7 | 5s full-list swap → layout jump / open-UI flash not addressed. | Note: stable card min-heights; don’t auto-open `<details>` on poll; accept full replace (no morph) in REQ-014. |
| S3 | §4.2 / §4.6 | Empty-state CTA copy named in backlog, not locked here. | One line each: tickets, ingest, cameras, lenses (German next-step). |
| S4 | §4.7 Admin | “etc.” + simulate missing from screen copy; confirms still EN in templates. | Full DE label list: refresh, simulate, delete-all, users, logout + DE `hx-confirm` strings. |
| S5 | §5 inventory | Missing `.order-form`, `.ingest-form`, `.archive`, `.ticket-more`, `.nav-more`, `.dev-panel`. | Add or explicitly “legacy OK”. |
| S6 | §3 nav | `details.nav-more` a11y (focus trap / Esc) unstated. | Document: native details only; ensure tap target ≥44px; close on navigate is enough. |
| S7 | L5 / motion | Rotate `transform` vs server-refetch image — may fight cache-bust `t=`. | Prefer image replace + `t=`; optional CSS transition only if it doesn’t desync. |
| S8 | §4.3 Importieren | “Hide free-text unless manuell” — no control for toggling manuell. | Spec checkbox/summary “Manuell” that reveals `camera_label`, or only when `cameras` empty (simpler — prefer latter). |
| S9 | I-Loop exits | No exit check for surface `name=` attrs / ST-014-c. | Add grep/review exit to I2 and I3. |
| S10 | §1 / logged-in | Brand “hero” only logged-out — good; ensure topbar brand mark isn’t the only identity at 360px. | Keep wordmark text visible at ≤640px (not icon-only). |

## Answers to open questions (§8 of definition)

1. **Auto-open preview (UI-13):** Do **not** auto-scroll or steal focus on mobile. Do show a persistent banner/CTA in the list (“N Bilder warten auf Freigabe”). Optional silent `hx-get` into the empty panel **once** is OK if it does not `scrollIntoView` / move focus — user taps Confirm/rotate deliberately. Lock that in §4.5.
2. **Ticket rename:** Keep **always-visible compact** rename (name + Speichern). Pickup-only users rename; burying it in details adds noise when they expand. Delete stays in overflow/details.
3. **Film-strip holes:** **Flatten** to plain dark rail. Lab bench rejected costume; holes fight “less film theme” (REQ-014 visual lock).
4. **Accent `#d4a853`:** **Keep gold.** REQ-014 already locked warm amber for brand continuity; cooler “lab” teal would restart token bikeshed.
5. **Pickup-only loop:** Covered by L1 + Discord wait + rename; no new API. Ensure Analog section stays visually secondary (or collapses to the PP-off note). Lead copy already explains Discord pickup — don’t add import pressure in Section A.

## Loop review (L1–L8)

- **L1:** OK — check vs save split clear; fix Enter (M9).
- **L2:** OK — disclosure + gear POST; ensure OOB `#tickets-list` preserves closed `<details>` expectation (document “resets closed”).
- **L3:** OK — convert → ingest list; remind `photoprism_configured` gate.
- **L4:** OK — poll 5s; risk = layout jump (S2).
- **L5:** OK — invariant correct; lock UI-13 (M10); confirm/cancel confirms missing (M8).
- **L6:** Gap — must align with `can_delete` (M4); “queued→delete→reimport” OK.
- **L7:** OK — gear page; add out targets (M7).
- **L8:** Risk — admin simulate + DE confirms under-specified (S4).

## Process loops (P/I/R)

- **P-Loop:** Sound. Author must revise definition (not defer M1–M10) before freeze.
- **I-Loop:** Order is right (chrome → tickets → ingest → preview → gear/admin → polish). Missing: explicit “surface field audit” exit on I2/I3; fold a11y smoke (keyboard tab order, 360px nav) into I1 + I6.
- **R-Loop:** Good happy path. Add one pickup-only pass (L1 only, no import) and one PP-off home glance.

## Capability coverage

Under-specified vs FRONTEND-SURFACE (not necessarily missing forever — freeze needs explicit mention):

| Surface item | Definition gap |
|--------------|----------------|
| Direct ingest `camera_label` vs `camera_id` | §4.2 form not enumerated (M3) |
| Convert fields (`secure_id`, `album`, gear, `camera_label`) | Sketched in §4.3; “manuell” toggle unclear (S8) |
| Job album on row (UI-09) | ASCII shows Album — OK; say “or Standard when empty” |
| Preview cancel → `failed` + panel clear OOB | L5 mentions Abbrechen; say panel clear |
| Delete ingest clears preview OOB | L6 silent — add one line |
| `#dm-out` / Discord test | Footer OK; keep collapsed |
| Admin simulate + user row `#user-{id}` | L8 / §4.7 thin (S4, M7) |
| Gear create feedback outs | Missing (M7) |
| Status DE labels wire map | Stepper nodes use DE — good; keep `.status-*` wire values |
| `GET /login` vs `/` logged-out | §4.1 lumps them — OK if same composition |
| HTML validation cheat-sheet | Missing (M8) |

No surface fields appear intentionally dropped; the risk is implementer omission from vagueness.

## Out of scope pushes

- Definition correctly parks PhotoPrism deep links, light theme, Tailwind, new APIs — good.
- Stepper + album echo are presentation-only — fine inside REQ-014.
- Do **not** solve poll flicker with HTMX morph extensions in this REQ (backlog UI-24) — call that out as deferred so I3 doesn’t grow scope.
- “Manuell” camera path must not invent a new API field — only reveal existing `camera_label`.

## Final recommendation

**No-go for freeze until Must-fix M1–M10 are written into the definition.** After that: **go** — Approve for implement. Should-fix can land in the same revision pass or as explicit “I6 defer” notes; do not start I2 with a broken §4.3 diagram or an open UI-13 decision.
