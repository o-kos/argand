# Issue #270: Move analysis status into the spectrogram

Resolves #270.

[Pull Request #272](https://github.com/o-kos/argand/pull/272).

## Overview

Move the existing analysis text and timing hint from the status bar to the bottom-right corner inside the actual spectrogram. Preserve text, progress, completion and input dismissal. Add the owner-requested bounded lifetime for completed feedback, as recorded below. Keep status-bar metadata/controls and all analysis contracts unchanged.

## Roles and risk

- Class A: the combined branch touches more than five code files, and the owner-feedback revision adds a cancellable presentation timer. Shell retains state ownership. Analysis processing, scheduling, generation, source-safety and data contracts remain unchanged.
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
- [x] Exercise the affected production scenarios on isolated Xvfb and record the limits below. The owner accepted the submitted result, including the final three-second behavior, on 2026-11-25. Unperformed probes remain explicit limits.
- [x] Independent read-only review of owner-feedback revisions. The muted/timed feedback and subsequent three-second adjustment each completed with no substantive findings. The earlier evidence-coverage wording correction was resolved.
- [x] Publish Draft PR with accurate evidence and remaining acceptance limits.
- [x] Record owner acceptance and move this plan to completed.

## Post-completion

The owner authorized finishing and merging this accepted PR. Request full Ready CI, squash-merge only after current `ci/full` passes and conversations are resolved, then preserve evidence and remove the accepted worktree and branch safely. Release remains outside this task.

## Initial validation and acceptance boundary

The full local gate passed: formatting, all-target Clippy, 974 tests with zero failures/ignores, then a successful release build. The exact production release was exercised on a task-owned Xvfb X11 display with disposable data and isolated supported XDG config/state/cache roots. Raw commands, binary identity, logs, screenshots and explicit unperformed checks are retained in the independent review handoff. Native Wayland and other OS behavior were not established by this local run. Owner acceptance was subsequently recorded below. This historical evidence does not claim unperformed checks passed.

## Owner feedback: muted status text and bounded completion lifetime

The owner requested gray text matching the status bar and automatic disappearance when a shortcut changes the scale and no later pointer input arrives. The coordinator relayed the working interpretation that this refers to the corner analysis status notice, rather than its separate hover timing Tooltip, and the concrete five-second delay. Continue under that stated interpretation unless the owner corrects it.

- Use the same `muted_foreground` theme token as the status bar, only for analysis status. The shared Saved notice keeps its established styling.
- Every accepted Ready presentation gets a three-second lifetime, following the later owner adjustment below. Ongoing Opening/Analyzing progress and failure details do not expire through this timer. Existing input dismissal remains immediate.
- Shell continues to own presentation state. Retain one ordinary cancellable toolkit task, tied to document identity and presentation epoch, so an older timeout cannot hide newer completion or another document. Cancel pending expiry on input, document replacement and accepted non-Ready status. Timeout changes only Ready visibility and never clears a Saved notice.
- Keep analysis generations, scheduling, cancellation, source data and toolkit graph unchanged. No polling or new widget is introduced.

### Feedback implementation and acceptance

- [x] Implement analysis-only muted text and Shell-owned bounded Ready lifetime.
- [x] Verify genuine toolkit-clock expiry, non-expiring progress, input cancellation, newer completion/document safety and Saved notice independence. Rejected delivery does not postpone the current deadline.
- [x] Reproduce shortcut zoom with a stationary pointer in the current production Xvfb binary, observe completion then disappearance after five seconds without more input.
- [x] Verify readable muted text in both themes and orientations, retaining existing placement and pass-through behavior.
- [x] Refresh full local gate, production binary identity, independent review and Draft PR description for this feedback revision.

Earlier gate and production observations above describe the preceding submitted behavior. The feedback-specific evidence follows below and the final acceptance is recorded at the end.

Previous five-second feedback validation passed formatting, all-target Clippy,978tests with zero failures/ignores, then release build. The exact release showed muted Ready text in both themes/orientations on isolated native X11. With the pointer parked outside the spectrum, a frequency-zoom shortcut produced visible Ready at roughly0.7–0.8seconds and it disappeared by6.3–6.4seconds without intervening input. Toolkit-clock tests separately verify the exact five-second boundary, non-expiring progress and stale/input/document safety. This pass did not repeat native minimum-size/save scenarios or the original unperformed checks above. Other-platform evidence was not established in that local run. The owner subsequently accepted the submitted result below.

## Owner feedback: three-second completion lifetime

The owner now explicitly requests three seconds instead of five. This supersedes the previous delay decision only. Keep the established Shell task, cancellation/identity guards, input dismissal, muted styling, progress and Saved behavior unchanged. Risk remains Class A for the combined branch and timer behavior, with the same coordinator/executor/reviewer assignments above.

- [x] Change the Ready timeout to three seconds and update fake-clock boundary/restart/rejection tests.
- [x] Update current architecture/changelog wording while preserving earlier five-second evidence as historical.
- [x] Run the mandatory full local gate, then observe the affected stationary-pointer shortcut scenario in the exact rebuilt production binary on task-owned Xvfb.
- [x] Obtain current-revision independent review and update the existing Draft PR, retaining the explicit validation limits.

Three-second adjustment validation passed formatting, all-target Clippy,978tests with zero failures/ignores, then the release build. A scoped current-production dark/horizontal X11 probe showed Ready at0.779seconds after a keyboard frequency zoom and absent at4.252seconds without intervening pointer/key input, before the old five-second deadline. Toolkit-clock tests establish the exact three-second boundary. Previous appearance/theme/orientation evidence remains historical and other acceptance limits are unchanged.

## Final acceptance and completion

On 2026-11-25 the owner accepted Pull Request #272, including the final muted analysis notice and three-second completion lifetime, and authorized the finishing/merge workflow. The latest source gate passed formatting, all-target Clippy, 978 tests with zero failures/ignores, then release. Independent review of the submitted three-second revision returned no substantive findings. The coordinator assessed the exact source, timer guards, evidence and limits.

The recorded unperformed native probes remain unperformed: acceptance does not turn them into test passes. The owner accepted the in-scope result with those limits documented. This completion change moves plans and records acceptance only, leaving the validated product source unchanged. Full required remote CI must pass on the final PR revision before merge. Post-merge cleanup follows the conditional branch/worktree checks above.
