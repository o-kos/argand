# Issue #242: Exact Commit Release

Resolves #242 (https://github.com/o-kos/argand/issues/242).

## Overview

Make the release Pull Request the single version input and provide explicit administrator initiation for an exact validated merged main SHA, preserving tag rulesets and publishing authority.

The owner requested Issues #259 and #242 together in one separate linked worktree, based on the main checkout's local branch. No merge or release is authorized by this implementation task.

## Roles and risk

- Class: A, release authority and cross-host runtime safety.
- Coordinator: `gpt-6.1-sol` medium, owner communication and final assessment.
- Executor: `gpt-6.1-sol` medium, investigation, implementation, validation and PR operations.
- Reviewer: `gpt-6.1-sol` high, fresh read-only context.
- Consultant: `gpt-6-astra` medium only if a concrete uncertainty requires it.

## Context and decisions

- Preserve the approved Rust/toolkit graph and immutable installed SDK. All Cargo artifacts stay in this worktree's target directory.
- Preserve main branch protection and Release tags ruleset. No secrets, new bypass actors, force tag updates or production release during development.
- Existing authenticated administrator git push triggers the tag workflow. A GITHUB_TOKEN-generated push is not a supported handoff.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| Issue acceptance criteria | Targeted policy/integration tests and exact-revision review | Pending | Native desktop and real remote publication evidence are separate from fixtures |
| Existing application behavior | Full local gate | Pending | Local SDK mode is not portable source packaging |

## Implementation steps

- [ ] Validate manifest, workspace lockfile versions and dated changelog in PR CI.
- [ ] Implement administrator initiation with full SHA, canonical successful CI, immutable tag creation and explicit safe retry.
- [ ] Retain tag equality checks and provide supported tag-push/manual retry workflow handoff.
- [ ] Test success and rejection scenarios without publishing a production release and update release documentation.

- [ ] Complete independent review and relevant validation.
- [ ] Move this plan to completed before final review when all acceptance evidence exists.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, last
- [ ] Change-specific policy and integration checks.

## Post-completion

Only the owner authorizes merge and production release. After merge, verify the installed workflow's handoff under administrator authority. Native desktop evidence unavailable during implementation must remain an explicit acceptance limit.
