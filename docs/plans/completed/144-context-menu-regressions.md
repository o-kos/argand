# Issue #144: Restore context-menu lifetime and keyboard regressions

Resolves [#144](https://github.com/o-kos/argand/issues/144).

## Overview

GPUI Component 0.9.1 replaces the ContextMenu dismiss subscription's strong
`Rc` capture with a `Weak`. Restore the production-menu regression coverage
previously excluded because the old cycle leaked its PopupMenu. Correct the
current control inventory without rewriting historical completed plans.
No production behavior, controls, dependency graph or module hierarchy changes.

## Roles and risk

- Class: A, because the regression concerns entity retention and subscription lifetime.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate read-only context.

## Context

- The locked graph is GPUI Kit/Component 0.9.1 and GPUI pre 0.3.12.
- Component `menu/context_menu.rs` uses `Rc::downgrade` for its dismiss subscription;
  its upstream `open_without_dismiss_releases_the_menu_entity` regression closes
  the window with an open menu while the application remains alive.
- Existing PlotView tests provide a harness around the production view, actual
  geometry, focus, navigation intents and a session grid command counter.
- `App::assert_no_new_leaks` is a no-op in downstream test-support builds unless
  leak detection is explicitly enabled. Direct WeakEntity assertions are required.

## Decisions

- Put new test bodies in `plot_view_context_menu_test.rs`, included in the existing
  PlotView test module to reuse its private harness without broad extraction or API changes.
- Open both time-ruler and spectrum menus through actual pointer input.
- Prove keyboard isolation and recovery on the ruler menu, plus destruction after
  closing a window with either menu still open. Keep only weak popup handles.
- This is test restoration, not a UI behavior change. Native appearance and OS
  interaction acceptance remain unperformed and are not prerequisites for a
  test-only change; no native UI acceptance claim will be made.

## Rejected alternatives

- Building an unrelated PopupMenu or directly setting `open_menu`: bypasses the
  ContextMenu ownership path that leaked.
- Treating an ordinary successful leak-snapshot assertion as evidence: it can be a no-op.
- Requiring immediate destruction on Escape: ContextMenu may retain the last
  dismissed entity in its element state until replacement or window removal.
- Production module reorganization, extra widgets or toolkit patches: outside scope.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Ruler popup isolates plot and session keys | Real right-click; Ctrl+G, time/frequency zoom and Home; assert no intents or grid action | Passed | Headless input routing only |
| Dismissal restores plot keyboard ownership | Escape, focus restoration, one grid action and one time-zoom intent | Passed | Native focus behavior not established |
| Open time/spectrum menus do not outlive their windows | Capture live WeakEntity; remove window without dismissal; drain effects; assert upgrade fails while App lives | Passed for both entry paths | Entity lifetime, not RSS measurement |
| Current inventory describes the locked graph and coverage | Inspect exact dependency sources and update R8/current audit row | Updated | Historical plans stay unchanged |

## Implementation steps

- [x] Add production ruler-menu keyboard isolation and recovery regression.
- [x] Add time/spectrum open-menu window-close lifetime regressions.
- [x] Correct the current inventory's obsolete cycle/testing restriction.
- [x] Run focused regressions and the full local gate.
- [x] Complete independent review and coordinator assessment.
- [x] Move the plan to completed before Ready.

## Validation

- [x] `cargo test --locked context_menu_tests -- --nocapture`: all three new
  regressions passed; the remaining workspace tests were filtered out.
- Optional leak-detector rebuild omitted: direct WeakEntity destruction assertions
  are active without it. No snapshot assertion or detector-success claim is used.
- [x] `cargo fmt --all -- --check`, plus explicit rustfmt check of the included test file.
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`: 1162 passed, zero failed; one existing manual cost benchmark ignored.
- [x] `cargo build --release --locked`, after all previous checks pass.
- [x] Independent read-only review by `gpt-6.1-sol` high: round 1 clean;
  coordinator assessment accepted. The final completion commit changes only this plan.

## Post-completion

Wait for current-revision full CI on Linux, Windows and macOS before merge.
Close Issue #144 through the Pull Request, preserving the stated native limits.
