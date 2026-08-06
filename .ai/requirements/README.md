# Requirements — how to use

Open **one REQ file by ID** when working a topic. Do not load this whole folder into context.

1. Start at [`_index.md`](_index.md) for the checklist (ID → one-line summary → link).
2. Open only the `REQ-*.md` files that match the task.
3. Legacy pointer: [`../REQUIREMENTS.md`](../REQUIREMENTS.md) → `_index.md`.

## File shape

Each `REQ-*.md` is short (~30–100 lines):

- **ID / title / status** (`accepted` | `planned`)
- **Goal**
- **Acceptance criteria** (checkboxes)
- **Tests** (required automated coverage; IDs like `T-005-a` / `ST-008-a` — must pass `cargo test`)
  - Mark each row **+/-** (positive happy path / negative rejection)
- **Out of scope**
- **Touches** (implementation paths)
- **Depends on** (other REQ ids)

**Unit tests** live next to modules (`#[cfg(test)]`). **System tests** (REQ-008) hit the real Axum router + temp DB in `src/system_tests.rs`. Image builds run `cargo test` in the Dockerfile builder stage.

## Status

| Status | Meaning |
|--------|---------|
| `accepted` | Implemented / in production behavior |
| `planned` | Spec ready; implementation may follow |
