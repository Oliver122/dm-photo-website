# AI agent checklist

Use before finishing a feature or PR.

## Build / verify

- [ ] `cargo check` (or `cargo test` if touching `dm_order` / logic with tests)
- [ ] New routes registered in `src/main.rs`
- [ ] New SQL only via `db.rs` + migration if schema changed
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
