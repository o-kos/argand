# Issue #1: Define the issue-driven development workflow

Resolves #1.

## Overview

Define a repository-wide development process that connects GitHub Issues, focused branches, versioned implementation plans, atomic commits, Pull Requests, review, and completed-plan archival.

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
- [ ] Validate links, templates, and repository instructions for consistency.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- Review all workflow documents for consistent terminology and paths.
- Verify that `CLAUDE.md` resolves to `AGENTS.md`.
- Verify GitHub template YAML syntax.
- Run repository formatting, linting, and tests because agent instructions are part of this change.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
