# REQ-016 — GitHub CI + coverage

- **ID:** REQ-016
- **Status:** accepted

## Goal

Gate pull requests and pushes to `main` with an automated `cargo test` run on GitHub. Publish Rust coverage from that same crate. HTTP system tests are the frontend coverage track.

## Acceptance criteria

- [x] GitHub Actions workflow `.github/workflows/ci.yml` with one job `test`.
- [x] Triggers: `pull_request` and `push` to `main`.
- [x] Job runs `cargo test` (same command as the Docker builder), then `cargo llvm-cov --lcov --html`.
- [x] Failed tests fail the check.
- [x] Coverage uploaded as artifacts (`lcov.info` + HTML) and summarized on the PR (llvm-cov text + one-line frontend note).
- [x] `rust-toolchain.toml` pins channel `1.94` (matches Docker).
- [x] No coverage floor, no Codecov, no Docker build, no CI secrets, no wrapper script.
- [x] Frontend coverage is explicitly the HTTP system tests (REQ-008), not a second toolchain.

## Tests

| ID | +/- | Case | Where |
|----|-----|------|--------|
| T-016-a | + | Workflow runs `cargo test` then `cargo llvm-cov --lcov --html` | `.github/workflows/ci.yml` |
| T-016-b | + | Coverage artifacts + step summary (llvm-cov text; frontend = system tests) | `.github/workflows/ci.yml` |
| T-016-c | − | Failed `cargo test` fails the GitHub check | `.github/workflows/ci.yml` |

- [x] T-016-a … T-016-c (enforced by the workflow; confirm on a PR)

## Out of scope

- Live Discord, CEWE, or PhotoPrism calls (tests must not make them).
- Docker image build (REQ-010 remains the release `cargo test` gate).
- Artifactory push (REQ-011).
- Deploy / Compose (REQ-012).
- Coverage floor or Codecov.
- A separate frontend / npm / Playwright toolchain (REQ-008).

## Touches

- `rust-toolchain.toml`
- `.github/workflows/ci.yml`
- `.ai/requirements/_index.md`, `.ai/CONVENTIONS.md`, `.ai/CHECKLIST.md`, `.ai/INDEX.md`, `README.md`

## Depends on

- REQ-008 (HTTP system tests are the frontend track)
- REQ-010 (image builder `cargo test` stays the release gate)
