# Issue #107: About Argand panel

Resolves #107: https://github.com/o-kos/argand/issues/107.

## Overview

Add a compact About panel, composed like File information, with a permanent
main-toolbar button and a direct application-menu command. Show the approved
icon, product name, Cargo version, description, MIT licence, repository URL and
build commit identity. No keyboard shortcut or independent application window.

## Roles and risk

- Class: A, because build metadata, overlay lifecycle and integration are expected
  to span more than five code files or roughly 400 code lines.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Context

The implementation starts from current main with locked GPUI Kit 0.6.6.
Upstream Rust 1.98.1 baseline work was merged before final validation; the
toolkit lock and About implementation are unchanged by that integration. The
independent GPUI Kit migration remains a separate Draft. Reuse `PinnedHint`,
standard Popover, Button and the keyboard-accessible Base Link, with existing
panel surface styling. The main working directory belongs to separate work;
the owner explicitly authorized a separate worktree after its branch changed.

## Decisions

- Toolbar button and main-menu command open one retained panel. No shortcut.
- Hover shows a tooltip containing exact HEAD tag (or `untagged`), short hash and
  commit timestamp. Hover does not open the panel.
- The panel closes on any click within it (the repository Link also opens its
  URL), Escape, focus leaving its entire subtree, or window deactivation.
- Existing panel/menu opening transitions close About and restore normal focus.
- Version, description, licence and repository come from Cargo package metadata.
- Git metadata is captured at build time only: short HEAD hash, commit timestamp
  in ISO 8601 with offset, exact tag or `untagged`. No nearest-tag substitution.
- Rebuild tracking covers worktree HEAD, its symbolic ref, shared refs/tags and
  packed refs; missing Git/source archive data has explicit unavailable labels.
- Native input is unavailable in this environment. Do not bypass native access
  restrictions. Record native acceptance as incomplete and retain Draft status;
  dispatch full remote CI separately if Ready acceptance cannot be established.

## Rejected alternatives

- A separate native About window: contradicts the transient-surface requirement.
- A centred Dialog: superseded by the owner's File information panel analogy.
- Legacy component Link: locked 0.6.6 supports pointer activation only; use the
  standard Base Link with its focus, keyboard and accessibility contract.
- Runtime Git subprocesses or a duplicate hardcoded package version.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Toolbar and main menu open the same compact panel | Production-path GPUI integration tests; native interaction | Headless passed | Native interaction unavailable |
| Approved icon and complete Cargo/build information | Metadata tests and production composition inspection | Automated/static passed | Native appearance pending |
| Tooltip gives exact tag, hash and commit date | Build metadata tests and tooltip text assertion | Passed | Native tooltip pending |
| Repeated invocation never duplicates the panel | Integration test | Passed | Native pending |
| Any surface click, repository activation and Escape dismiss | Pointer and keyboard integration tests | Passed | Native pending |
| Focus leaving subtree and window deactivation dismiss | GPUI focus/activation integration tests | Passed | Native compositor pending |
| System, light and dark follow existing application theme | Semantic tokens and rendered themed scenarios | Headless light/dark passed | Native visual acceptance pending |
| Linux, Windows and macOS build and logic checks | Full remote CI on current branch | Pending | Native platform acceptance remains separate |

## Implementation steps

- [x] Add Cargo/build metadata with exact-tag and missing-Git behavior.
- [x] Compose the About panel and integrate toolbar/menu/lifecycle.
- [x] Add meaningful metadata and production-path interaction coverage.
- [x] Update architecture, control inventory and changelog.
- [x] Complete the local gate and record evidence/limits.
- [x] Prepare exact neutral independent review handoff; round 1 has no findings.
- Full remote CI must succeed without merging; exact revision and results are
  recorded in the Pull Request, keeping its CI evidence current.
- [ ] Native acceptance and completed-plan move remain pending while unavailable.

## Validation

- [x] `cargo fmt --all -- --check` (Rust 1.98.1 after upstream integration)
- [x] `cargo clippy --all-targets --locked` (Rust 1.98.1 after upstream integration)
- [x] `cargo test --locked` (Rust 1.98.1 after upstream integration)
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Metadata tests, rendered pointer/keyboard/focus integration tests.
- Independent review: round 1 is clean; focused upstream-integration revalidation
  and its exact snapshot/result are recorded in the Pull Request review handoff.
- Full Linux/Windows/macOS remote CI: current-revision run/result is recorded in
  the Pull Request. Native acceptance remains a separate pending requirement.

Current automated evidence: the Rust 1.98.1 workspace gate passes 1,167 tests (one existing
ignored test). Six About production-path UI scenarios cover both entry points,
Escape, repeated opening, pointer/Enter/Space repository activation, focus
subtree transitions, window activation, outside presses and light/dark layout.
The metadata module also passes four standalone tests without GPUI. A disposable
Cargo project verifies actual rebuilds for same-ref commits, first tags, packed
refs creation/removal and linked-worktree shared refs; missing Git returns explicit
unavailable values without a panic. The full gate and source snapshot are retained
in the external review packet. The Rust 1.98.1 release build passed in 9m24s.
A final release rebuild follows this validation-record commit so its embedded
identity names the submitted source revision; its artifact identity is retained
in the external handoff.

Independent review round 1 found no substantive metadata, lifecycle, UI-composition,
architecture or validation-evidence findings. There were no accepted/rejected
findings to carry forward. Upstream integration requires the full gate on the new
Rust baseline (formatting, lint, tests and release now pass) and a focused
current-snapshot review before the final CI result. Focused revalidation and
remote CI outcomes are retained in the Pull Request without subsequent commits
that would invalidate the checked revision.
Native acceptance remains incomplete.

## Post-completion

Merge is not authorized by this task. Native acceptance must be completed before
Ready/merge under the repository policy. Preserve the independent main checkout.
