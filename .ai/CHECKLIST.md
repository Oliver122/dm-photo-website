# AI agent checklist

Use before finishing a feature or PR.

## Build / verify

- [ ] `cargo test` (required — unit `T-*` and system `ST-*`, including +/- cases)
- [ ] `cargo check`
- [ ] GitHub CI (REQ-016) is the PR test gate; frontend coverage = HTTP system tests
- [ ] New routes registered in `src/main.rs`
- [ ] New SQL only via `db.rs` + migration if schema changed
- [ ] New/changed REQ acceptance → matching `T-*` tests added or updated
- [ ] `.env.example` updated if new env vars
- [ ] `.ai/` docs updated if routes/schema/behavior changed

## Product

- [ ] Auth correct (`AuthUser` / `AdminUser`)
- [ ] Ticket mutations scoped to owning `user_id`
- [ ] Order numbers validated before external calls
- [ ] HTMX responses use fragments / OOB list refresh when needed

## Git

- [ ] No secrets staged
- [ ] Branch named per `.ai/GIT-AND-BRANCHES.md`
- [ ] Commit only if user requested
