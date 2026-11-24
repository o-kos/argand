# Issue #270: Move analysis status into the spectrogram

Resolves #270.

[Draft Pull Request #272](https://github.com/o-kos/argand/pull/272).

## Overview

Move the existing analysis text and timing hint from the status bar to the bottom-right corner inside the actual spectrogram. Preserve text, progress, visibility, completion and dismissal. Keep status-bar metadata/controls and all analysis contracts unchanged.

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

Shell/Document continue to derive status and own dismissal. Add the existing presentation to PlotSnapshot and render it using the already existing bottom-right saved-notice composition and stock Tooltip. Stack save and analysis notices if simultaneous. Position against PlotGeometry.spectrum so axes/minimap/corner zoom controls remain clear in both orientations. Use theme surface/foreground/border tokens and rem scale. Preserve pre-header Opening status in the content placeholder using the same passive composition; central failure details remain unchanged without an extra duplicate failure label. No occlusion, focus target or input handler is introduced; plot input retains its existing bubbling contract.

- [x] Move only the status presentation to PlotSnapshot and existing plot notice composition.
- [x] Preserve stock timing Tooltip and tooltips eligibility.
- [x] Remove old status-bar analysis fragment without changing metadata or controls.
- [x] Verify rendered overlay input pass-through and progress/completion/dismissal regressions.
- [x] Verify orientations/themes and supported minimum with actual production selection and corner controls. Automated rendered regression separately covers simultaneous saved/analysis notices and direct input pass-through.
- [x] Update changelog and UI architecture with the new presentation location.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| One analysis status at spectrum bottom-right | Rendered production UI in both orientations and themes | Passed in production light/dark and both orientations, including actual 640×400 windows | Xvfb establishes X11 only |
| Progress, completion, hint and dismissal unchanged | Existing lifecycle tests plus production initial/re-analysis and input scenarios | Production progress, completion and settled Ready dismissal observed, automated presentation tests passed | Timing Tooltip, pre-header Opening and failure appearance not exercised natively; no scheduling changes |
| Navigation/selection pass through notice | Rendered actual plot pointer/wheel/drag regression and Xvfb interaction | Rendered production-composition pointer/wheel/drag regression passed; actual plot zoom and selection exercised | Native direct input through still-visible progress was not separately isolated; no occlusion or handlers |
| Readable and clear of essential UI | Narrow/normal windows, light/dark, existing ruler controls and badges | Production minimum passed in dark/light vertical layouts, with selection readouts additionally exercised in light; normal horizontal layouts observed | Simultaneous save/analysis native appearance unperformed; owner acceptance remains distinct |

## Layout limits

Production geometry is measured in the current canvas frame. Feedback tries full, half and quarter available widths and the nearest free positions above/left of existing selection badges and corner controls. Both supported 640×400 minimum and larger windows pass the rendered collision/input regression in both orientations/themes. Transient smaller surfaces retain feedback at the corner after bounded attempts, with essential badges/guides painted above it. This does not claim collision-free placement in arbitrary undersized geometry. Selection badge positions remain unchanged.

## Rejected alternatives

A notification system or new custom widget would change dismissal/focus behavior. Moving status state into PlotView would unnecessarily change ownership. Anchoring to the outer plot panel would overlap rulers or minimap in one orientation.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after preceding checks pass
- [ ] Production Xvfb acceptance: partial evidence below. Retain active plan until remaining acceptance is resolved.
- [x] Independent read-only review of submitted product source. Evidence-coverage wording was corrected and the final source review returned no substantive findings.
- [x] Publish Draft PR with accurate evidence and remaining acceptance limits.
- [ ] Move this plan to completed only after every in-scope acceptance item passes.

## Post-completion

Merge and release require the corresponding owner authority and are outside this task. Preserve worktree data until the PR is accepted, then follow safe cleanup policy.

## Current validation and acceptance boundary

The full local gate passed: formatting, all-target Clippy, 974 tests with zero failures/ignores, then a successful release build. The exact production release was exercised on a task-owned Xvfb X11 display with disposable data and isolated supported XDG config/state/cache roots. Raw commands, binary identity, logs, screenshots and explicit unperformed checks are retained in the independent review handoff. Native Wayland, other OS behavior and owner acceptance remain unestablished. Keep this plan active and the Pull Request Draft while acceptance remains incomplete.
