# Issue #1: Define the issue-driven development workflow

Resolves #1.

## Overview

Define a repository-wide development process that connects GitHub Issues, focused branches, versioned implementation plans, atomic commits, Pull Requests, review, and completed-plan archival.

This bootstrap branch was created before Issue #1 and therefore retains the earlier name `docs/project-workflow`. New branches follow the issue-numbered convention defined by this change.

## Decisions

- Every implementation starts from a GitHub Issue.
- Open-ended ideas may begin in a GitHub Discussion, but implementation requires an Issue.
- Active plans live in `docs/plans/`; accepted plans live in `docs/plans/completed/`.
- Plan progress is committed alongside the corresponding implementation work.
- Pull Requests use squash merge. The plan is the durable task-level record after branch commits are squashed.
- A completed plan moves to `docs/plans/completed/` before final review and merge.

## Implementation steps

- [x] Translate the shared agent instructions to English and expose them to Claude Code through `CLAUDE.md`.
- [x] Document the issue-driven workflow and contribution requirements.
- [x] Add reusable active-plan guidance and a plan template.
- [x] Add GitHub Issue and Pull Request templates.
- [x] Validate links, templates, and repository instructions for consistency.
- [x] Move this plan to `docs/plans/completed/` before final review.
- [x] ➕ Move the long-term implementation roadmap into `docs/plans/`, translate it to English, and update its references after review.

## Validation

- Documentation terminology, paths, and relative links reviewed for consistency.
- `CLAUDE.md` verified as a symbolic link to `AGENTS.md`.
- GitHub Issue template YAML parsed successfully.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --all-targets --locked -- -D warnings` passed.
- `cargo test --locked` passed: 165 tests and 3 doc-test suites.
- Follow-up roadmap paths and references verified; `cargo fmt --all -- --check` and `git diff --check` passed after the move.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
