# Issue #251: Refresh retained File information size after saving

Resolves #251.

## Overview

Refresh File size after successful save/rebind while the File information panel remains open. Preserve its retained controls and conversion drafts. File size is presentation only and must continue to come from the accepted document summary, without new filesystem reads or save-path changes.

## Roles and risk

- Class A: the combined branch touches more than five code files. Existing state ownership and processing, scheduling, generation, source-safety and data contracts remain unchanged.
- Coordinator: `gpt-6.1-sol` medium, owner communication and final assessment.
- Executor: `gpt-6.1-sol` medium, investigation, implementation, validation and Draft PR.
- Reviewer: `gpt-6.1-sol` high in a fresh read-only context.
- Consultant: none unless focused investigation leaves a concrete uncertainty.

## Decisions

- The owner requested Issues #251 and #270 together in one separate linked worktree. Keep separate plans and implementation commits in one focused follow-up branch.
- Preserve the dirty main checkout and its #265 work completely.
- Use the installed pinned Rust 1.98.1, approved locked toolkit graph, immutable installed SDK and workspace-local target directory.
- Use standard GPUI composition and existing shared surfaces, with no new custom widgets.

## Context and implementation

`conversion_ui::Editor::prepare` already refreshes live Peak/RMS/reference rows from the same document. Include File size in that synchronization, excluding source-only rows while the capture is unsaved. Reuse the existing information panel, retained NumericEditor/Input/Select entities and save/rebind lifecycle.

- [x] Extend only live readout synchronization, preserving draft controls.
- [x] Extend production save/rebind regression to compare the new byte count and retained drafts.
- [x] Inspect related save entry paths. Save As intentionally closes information panels before its native chooser, so it has no retained panel to refresh. Ordinary Save retains the panel and uses the shared successful rebind path. Unsaved captures continue to omit source-only size/reference rows.
- [x] Update changelog and current architecture documentation.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| Retained File size follows successful metadata save/rebind | Production save/rebind regression and Xvfb ordinary Save of disposable WAVE | Passed: production ordinary Save changed 48000044 B to 48000216 B and retained readout matched | No data-safety behavior changes |
| Panel and draft controls survive refresh | Retained entity identity and draft-value assertions, focus/edit/Enter/Escape and Select scenario | Passed: retained Input/Select identities and unfinished draft in automated Save regression, live 49 kHz draft preserved across ordinary Save | Native Select interaction and Wayland/macOS/Windows not exercised in this run |
| Source-only File size is hidden until an unsaved capture is rebound | Shared summary synchronization regression | Source guard retained. Save As closes the panel before native chooser, so no retained panel crosses first publication | No new filesystem reads |

## Rejected alternatives

Reopening the panel would discard retained drafts. Reading filesystem metadata during render would add I/O and bypass the accepted document summary.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after preceding checks pass
- [ ] Production Xvfb acceptance: partial evidence below. Retain active plan until remaining acceptance is resolved.
- [ ] Independent read-only review of exact submitted revision.
- [ ] Publish Draft PR with accurate evidence and remaining acceptance limits.
- [ ] Move this plan to completed only after every in-scope acceptance item passes.

## Post-completion

Merge and release require the corresponding owner authority and are outside this task. Preserve worktree data until the PR is accepted, then follow safe cleanup policy.

## Current validation and acceptance boundary

The full local gate passed: formatting, all-target Clippy, 974 tests with zero failures/ignores, then a successful release build. The exact production release was exercised on a task-owned Xvfb X11 display with disposable data and isolated supported XDG config/state/cache roots. Raw commands, binary identity, logs, screenshots and explicit unperformed checks are retained in the independent review handoff. Native Wayland, other OS behavior and owner acceptance remain unestablished. Keep this plan active and the Pull Request Draft while acceptance remains incomplete.
