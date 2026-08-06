# REQ-002 — Orders & tickets

- **ID:** REQ-002
- **Status:** accepted

## Goal

Let logged-in users check dm Foto order status, watch ERROR/open orders via tickets, get Discord DMs on completion, and manage their own tickets in the HTMX UI.

## Acceptance criteria

### Order tracking

- [x] Accept order numbers matching `^\d{6}-\d{6}$` (exactly 13 chars).
- [x] Query dm Foto order API with configurable `DM_KEY_ACCOUNT_ID` (default 1320).
- [x] Show German timeline labels aligned with dm’s progress wording.
- [x] When status is `ERROR`, allow creating a watch ticket for the logged-in user.
- [x] Manual ticket create / rename (label) / delete for own tickets.
- [x] Background refresh of uncompleted tickets every 3 hours.
- [x] On transition to done (`DELIVERED` / `PICKED_UP`), mark completed and attempt Discord DM if bot token configured.
- [x] Archive completed tickets older than 7 days in the UI (hidden/archive tab).

### Discord DM

- [x] “Send me a Discord DM” uses bot token + shared guild requirement (Discord policy).
- [x] Ticket completion DMs reuse the same bot path.
- [ ] Graceful UX when bot cannot DM (403 / privacy) — keep messages clear; do not crash.

### API

- [x] Order check + ticket CRUD endpoints used by HTMX UI (see ROUTES-AND-DATA).

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| T-002-a | + | Valid order number accepted | `src/dm_order.rs` |
| T-002-b | − | Bad order numbers rejected | `src/dm_order.rs` |
| T-002-c | + | READY/DONE helpers | `src/dm_order.rs` |
| T-002-d | + | Timeline labels (home vs pickup) | `src/dm_order.rs` |
| T-002-e | + | Parse ERROR payload fixture | `src/dm_order.rs` |
| T-002-f | +/− | `completed_before` true when old / false when recent or open | `src/models.rs` |

- [x] T-002-a … T-002-f

## Out of scope

- Changing order-number format or DONE/READY state constants without a product ask.
- Exploit PoCs against Discord/dm APIs.

## Touches

- `src/` order/ticket handlers, jobs, Discord bot path
- Templates/UI for order timeline and tickets
- `.env`: `DM_KEY_ACCOUNT_ID`, Discord bot token

## Depends on

- REQ-001 (logged-in user for tickets)
