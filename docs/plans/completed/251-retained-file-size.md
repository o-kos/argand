# Issue #251: Refresh retained File information size after saving

Resolves #251.

[Pull Request #272](https://github.com/o-kos/argand/pull/272).

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
- [x] Exercise the affected production scenarios on isolated Xvfb and record the limits below. The owner accepted the submitted result, including the final three-second behavior, on 2026-11-25. Unperformed probes remain explicit limits.
- [x] Independent read-only review of submitted product source. Evidence-coverage wording was corrected and the final source review returned no substantive findings.
- [x] Publish Draft PR with accurate evidence and remaining acceptance limits.
- [x] Record owner acceptance and move this plan to completed.

## Post-completion

The owner authorized finishing and merging this accepted PR. Request full Ready CI, squash-merge only after current `ci/full` passes and conversations are resolved, then preserve evidence and remove the accepted worktree and branch safely. Release remains outside this task.

## Initial validation and acceptance boundary

The full local gate passed: formatting, all-target Clippy, 974 tests with zero failures/ignores, then a successful release build. The exact production release was exercised on a task-owned Xvfb X11 display with disposable data and isolated supported XDG config/state/cache roots. Raw commands, binary identity, logs, screenshots and explicit unperformed checks are retained in the independent review handoff. Native Wayland and other OS behavior were not established by this local run. Owner acceptance was subsequently recorded below. This historical evidence does not claim unperformed checks passed.

## Final acceptance and completion

On 2026-11-25 the owner accepted Pull Request #272, including the final muted analysis notice and three-second completion lifetime, and authorized the finishing/merge workflow. The latest source gate passed formatting, all-target Clippy, 978 tests with zero failures/ignores, then release. Independent review of the submitted three-second revision returned no substantive findings. The coordinator assessed the exact source, timer guards, evidence and limits.

The recorded unperformed native probes remain unperformed: acceptance does not turn them into test passes. The owner accepted the in-scope result with those limits documented. This completion change moves plans and records acceptance only, leaving the validated product source unchanged. Full required remote CI must pass on the final PR revision before merge. Post-merge cleanup follows the conditional branch/worktree checks above.
