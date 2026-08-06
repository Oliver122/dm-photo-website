# Git and branches

## Default branches

| Branch | Role |
|--------|------|
| `main` | Stable / integration baseline |
| `feat/*` | Feature work (current long-lived: `feat/discord-auth-scaffold`) |
| `fix/*` | Bugfixes |
| `chore/*` | Tooling, docs, `.ai`, deps without product behavior |
| `docs/*` | Documentation-only |

## Rules for agents

1. **Do not commit unless the user explicitly asks.**
2. **Do not push / force-push / amend** unless the user explicitly asks (and follow user git safety rules).
3. Prefer small PRs: one concern per branch when splitting is requested.
4. Branch from latest `main` for new work unless the user says to continue an existing feature branch.
5. Never commit `.env`, `data/*.db*`, credentials, or `target/`.

## Commit messages

Recent style in this repo:

- `feat: …` / `feat(tickets): …` — new capability
- `chore: …` — scaffolding / repo hygiene
- Conventional Commits, imperative, focus on **why**

Examples:

```
feat(tickets): track dm orders, labels, UI
feat: add Rust HTMX site with Discord OAuth
chore: initial repository setup
```

## PR expectations

- Title matches intent (feat/fix/chore).
- Body: short summary + test plan (manual: login, order check, ticket refresh, admin).
- Call out migration files and new env vars.
- Link or attach design notes from `.ai/reviews/` when relevant.

## Working tree notes

- Feature work has lived on `feat/discord-auth-scaffold` ahead of `main`.
- Root `README.md` may trail feature commits — verify against `src/main.rs` and `.ai/ROUTES-AND-DATA.md`.

## Review artifacts

Store AI/human review write-ups under [`.ai/reviews/`](reviews/). Naming: `YYYY-MM-DD-topic.md`. Keep PR discussion on GitHub; copy durable decisions into reviews so future sessions don’t rely on chat history.
