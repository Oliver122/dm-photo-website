# Frontend redesign note — Lab bench (REQ-014)

- **Date:** 2026-08-07
- **Branch:** `feat/req-014-lab-bench-ui`
- **Definition:** [2026-08-07-frontend-ui-definition.md](2026-08-07-frontend-ui-definition.md)

## Shipped

- Lab-bench tokens in `static/styles.css` (hand CSS, no Tailwind).
- Shell: sticky topbar, wordmark, mobile `nav-more`.
- Home: Aufträge primary / Analog secondary; Enter → Status prüfen.
- Tickets: progressive disclosure (Ausrüstung / Importieren); delete in `ticket-more`.
- Ingest: card list, stepper, album line, preview banner, no table.
- Preview: flat dark rail (no sprocket holes); sticky confirm bar.
- Gear + admin German; action feedback slots + `ST-014-*` tests green.

## Dropped from old look

- “Darkroom” eyebrow / film sprocket costume.
- Always-open ticket gear grids.
- Ingest HTML table.
- English admin chrome.
