# Issue #15: Define backlog triage for out-of-scope findings

Resolves #15.

## Overview

Small pre-existing problems discovered during implementation or review currently have
no explicit destination. They can either expand an active Issue and Pull Request beyond
their stated goal or disappear after being judged out of scope.

This change introduces a lightweight GitHub `backlog` label and documents when an
out-of-scope finding becomes a separate backlog Issue instead of work in the active
branch.

## Context

- `CONTRIBUTING.md` defines the project workflow and already requires independent work
  to use a separate Issue rather than silently expanding the current one.
- `AGENTS.md` is the mandatory condensed project context for coding agents.
- Issue #14 is the first concrete example: a small CLI maintainability improvement found
  while Issue #8 was in progress.
- The `backlog` label exists in GitHub and Issue #14 carries it.

## Decisions

- **Use one `backlog` label, not a second planning system.** The repository has few open
  Issues, and GitHub Issue filters provide sufficient separation without the overhead of
  a Project board or a backlog milestone.
- **Absence of the label means normal triage.** Existing Issues do not need a bulk label
  migration. Open backlog work is found with `is:open is:issue label:backlog`; the
  complementary view uses `is:open is:issue -label:backlog`.
- **Eligibility is defined by scope and risk, not merely size.** A finding may enter the
  backlog only when it is pre-existing, unrelated to the active objective, non-urgent,
  and irrelevant to the current acceptance criteria and claimed behaviour.
- **Required findings remain in the active work.** Regressions introduced by the branch,
  unmet acceptance criteria, false claims, required-check failures, and security,
  correctness, or data-safety problems cannot be deferred under this rule.
- **Tests and review keep an audit trail.** A backlog Issue records the problem, desired
  outcome, discovery context, and source Issue or Pull Request. A deferred review finding
  is resolved with a link to that Issue.

## Rejected alternatives

- A GitHub Project with a Backlog status. It provides ordering and workflow automation
  that the current repository does not need.
- A Backlog milestone. Milestones describe delivery targets, while this classification
  explicitly has no committed delivery window.
- A repository backlog document. It would duplicate GitHub Issues and lose their links,
  discussion, and searchable state.
- Adding every discovered finding to the active plan. That makes a focused plan absorb
  unrelated work and defeats the Issue boundary already required by the workflow.

## Implementation steps

- [x] Create the GitHub `backlog` label and apply it to Issue #14.
- [x] Document detailed backlog eligibility, exclusions, and recording requirements in
      `CONTRIBUTING.md`.
- [x] Add the mandatory condensed routing rule to `AGENTS.md`.
- [x] Verify the GitHub label and Issue #14 classification.
- [ ] External review round, then act on the findings.
- [x] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] `git diff --check`
- [x] GitHub exposes Issue #14 under `is:open is:issue label:backlog` and excludes it
      from `is:open is:issue -label:backlog`.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
