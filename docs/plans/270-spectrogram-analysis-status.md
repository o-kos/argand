# Issue #270: Move analysis status into the spectrogram

Resolves #270.

## Overview

Move the existing analysis text and timing hint from the status bar to the bottom-right corner inside the actual spectrogram. Preserve text, progress, visibility, completion and dismissal. Keep status-bar metadata/controls and all analysis contracts unchanged.

## Roles and risk

- Class B: local GUI presentation changes with existing state ownership and no processing, scheduling, generation, source-safety or data contracts changed.
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

Shell/Document continue to derive status and own dismissal. Add the existing presentation to PlotSnapshot and render it using the already existing bottom-right saved-notice composition and stock Tooltip. Stack save and analysis notices if simultaneous. Position against PlotGeometry.spectrum so axes/minimap/corner zoom controls remain clear in both orientations. Use theme surface/foreground/border tokens and rem scale. No occlusion, focus target or input handler is introduced; plot input retains its existing bubbling contract.

- [ ] Move only the status presentation to PlotSnapshot and existing plot notice composition.
- [ ] Preserve stock timing Tooltip and tooltips eligibility.
- [ ] Remove old status-bar analysis fragment without changing metadata or controls.
- [ ] Verify rendered overlay input pass-through and progress/completion/dismissal regressions.
- [ ] Verify orientations, themes, narrow windows and coexistence with selection/cursor readouts, saved notice and corner controls.
- [ ] Update changelog and UI architecture with the new presentation location.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| One analysis status at spectrum bottom-right | Rendered production UI in both orientations and themes | Pending | Xvfb establishes X11 only |
| Progress, completion, hint and dismissal unchanged | Existing lifecycle tests plus production initial/re-analysis and input scenarios | Pending | No scheduling changes |
| Navigation/selection pass through notice | Rendered actual plot pointer/wheel/drag regression and Xvfb interaction | Pending | No occlusion or handlers |
| Readable and clear of essential UI | Narrow/normal windows, light/dark, existing ruler controls and badges | Pending | Owner acceptance remains distinct |

## Rejected alternatives

A notification system or new custom widget would change dismissal/focus behavior. Moving status state into PlotView would unnecessarily change ownership. Anchoring to the outer plot panel would overlap rulers or minimap in one orientation.

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
