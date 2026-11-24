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
- [ ] Cover unsaved-to-saved visibility and related save entry paths through the shared invariant.
- [ ] Update changelog and current architecture documentation if needed.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| Retained File size follows successful metadata save/rebind | Production save/rebind regression and Xvfb ordinary Save of disposable WAVE | Pending | No data-safety behavior changes |
| Panel and draft controls survive refresh | Retained entity identity and draft-value assertions, focus/edit/Enter/Escape and Select scenario | Pending | Native Wayland/macOS/Windows not established by Xvfb |
| Source-only File size is hidden until an unsaved capture is rebound | Shared summary synchronization regression | Pending | No new filesystem reads |

## Rejected alternatives

Reopening the panel would discard retained drafts. Reading filesystem metadata during render would add I/O and bypass the accepted document summary.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after preceding checks pass
- [ ] Production Xvfb acceptance with disposable inputs and isolated settings/session/cache.
- [ ] Independent read-only review of exact submitted revision.
- [ ] Publish Draft PR with accurate evidence and remaining acceptance limits.
- [ ] Move this plan to completed only after every in-scope acceptance item passes.

## Post-completion

Merge and release require the corresponding owner authority and are outside this task. Preserve worktree data until the PR is accepted, then follow safe cleanup policy.
