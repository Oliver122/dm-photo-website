# Requirements — how to use

Open **one REQ file by ID** when working a topic. Do not load this whole folder into context.

1. Start at [`_index.md`](_index.md) for the checklist (ID → one-line summary → link).
2. Open only the `REQ-*.md` files that match the task.
3. Legacy pointer: [`../REQUIREMENTS.md`](../REQUIREMENTS.md) → `_index.md`.

## File shape

Each `REQ-*.md` is short (~30–80 lines):

- **ID / title / status** (`accepted` | `planned`)
- **Goal**
- **Acceptance criteria** (checkboxes)
- **Out of scope**
- **Touches** (implementation paths)
- **Depends on** (other REQ ids)

## Status

| Status | Meaning |
|--------|---------|
| `accepted` | Implemented / in production behavior |
| `planned` | Spec ready; implementation may follow |
