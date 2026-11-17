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
| Sole version input and metadata consistency | release-metadata.py plus exact-revision fixture and malformed lock/date/duplicate/pending tests | Passed locally, wired into CI | Remote CI pending |
| Exact merged SHA with explicit administrator initiation | Full SHA validation, immutable git-show inputs, admin/protection/ruleset rejection tests | Passed fixtures | Approval policy requires the actual merged SHA |
| Same-SHA canonical successful required CI | Run path/event/branch/suite/app/SHA rejection tests and live read-only main CI query | Passed | Live production creation not attempted |
| Stale/failed/missing/running CI cannot release | Negative API fixtures, no fallback to another SHA or older successful attempt | Passed | Main may advance without substituting approved merged SHA |
| Immutable tag and safe retries | Real disposable bare-remote annotated tag push/conflict test, explicit matching-tag retry tests | Passed locally | No production tags created |
| Existing Release including drafts never replaced | Initiator, verify and write-scoped publication checks | Passed fixtures | Existing-release recovery deliberately manual |
| Administrator authority retained on dispatch/rerun | Original and triggering actor guard, repeated immediately before publish | Tested rejection | Workflow API authority still requires remote operational demonstration |
| Supported initial push/retry handoff | Ordinary administrator git push and tag-ref workflow_dispatch | Locally established, remote pending | No isolated non-production remote authorized |
| Approved SHA/tag/version/artifact source agreement | Metadata/tag fixture, existing BUILD-INFO and binary smoke checks | Partial | Successful remote non-production publication handoff pending |
| Existing application behavior | fmt, Clippy, workspace tests | Passed | SDK mode gate, portable release separate |

## Implementation steps

- [x] Validate manifest, workspace lockfile versions and dated changelog in PR CI.
- [x] Implement administrator initiation with full SHA, canonical successful CI, immutable tag creation and explicit safe retry.
- [x] Retain tag equality checks and provide supported tag-push/manual retry workflow handoff.
- [x] Test success and rejection scenarios without publishing a production release and update release documentation.

- [ ] Complete independent review and relevant validation.
- [ ] Move this plan to completed before final review when all acceptance evidence exists.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, last
- [ ] Change-specific policy and integration checks.

## Post-completion

Only the owner authorizes merge and production release. Remote non-production handoff is an unfinished acceptance criterion until independently demonstrated, not a post-completion checkbox.
