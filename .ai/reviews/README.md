# Reviews and decision log

## Purpose

Durable notes from code reviews, design choices, and incident learnings. Agents should read recent files here before large refactors.

## How to add an entry

1. Create `YYYY-MM-DD-short-kebab-slug.md` in this folder.
2. Use the template below.
3. Link the PR/issue if one exists.
4. Do not store secrets, tokens, or personal Discord IDs.

## Template

```markdown
# <Title>

- **Date:** YYYY-MM-DD
- **Author:** human | ai
- **PR / branch:** …
- **Status:** proposed | accepted | superseded

## Context

What problem or change triggered this review.

## Findings

- Location / issue / recommended fix (one line each when possible)

## Decisions

What we agreed to do (or not do).

## Follow-ups

- [ ] …
```

## Index

| Date | File | Topic |
|------|------|-------|
| 2026-08-06 | [2026-08-06-ai-folder-bootstrap.md](2026-08-06-ai-folder-bootstrap.md) | Created `.ai` knowledge base |
