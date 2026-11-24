# Issues #260 and #253: Recent file layout and tooltip anchors

Resolves #260. Resolves #253.

## Overview

Show complete recent filenames on the start page whenever the available row width
permits, and prevent a tooltip from describing an old row after wheel scrolling
beneath a stationary pointer. The owner explicitly authorized combining these two
Issues in one Pull Request and a separate linked worktree.

## Roles and risk

- Class B: local recent-file presentation and standard tooltip interaction in two GUI-related modules.
- Coordinator: `gpt-6.1-sol` medium, owner communication and final assessment.
- Executor: `gpt-6.1-sol` medium, investigation, implementation, validation and Draft Pull Request operations.
- Reviewer: `gpt-6.1-sol` high in a fresh read-only context.
- Consultant: none planned. A justified consultation uses `gpt-6.2-astra` high through the read-only CLI.

## Context

`recent.rs` prepares shared labels before layout, currently shortening names over
20 characters. The start page additionally caps its content width at 560 pixels.
The File menu also consumes these labels, with its own bounded middle truncation.
Start-page rows are standard Buttons with native Interactivity tooltip builders.
The locked native tooltip implementation retains source bounds captured when shown;
its wheel handler clears a tooltip only when the tooltip hitbox is not hovered.
Production-view and isolated production Xvfb checks establish that visible hints
are dismissed by wheel input, but pending show timers survive it and later render
the old filename and shortcut over the newly hovered row. The native wheel helper
explicitly leaves WaitingForShow untouched. The correction reuses the existing
shared external-tooltip registration teardown through an intervening frame. Installed toolkit/SDK sources remain immutable.

## Decisions

- Keep the existing content-sized centered group, number column, Button behavior,
  shortcuts, full filename/directory tooltip and short-window scrolling.
- Give the start page unshortened disambiguated labels and the available window
  width. Ellipsis is a layout decision only when a row cannot fit.
- Preserve the File menu's compact-label policy deliberately and test both consumers.
- Use supported GPUI/toolkit mechanisms or existing shared compositions only.
  Any newly required nonstandard widget needs a separate owner decision.
- Run production acceptance only on task-owned Xvfb with isolated configuration,
  state and cache. Preserve the owner's desktop and running applications.

## Rejected alternatives

- A larger fixed character count or width cap still discards text before layout.
- Toolkit dependency changes, registry patches and custom row controls exceed scope.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Full filename when it fits | Long names at 1200×600 and 420×400, including resize | Passed in production Xvfb and rendered geometry tests | Native Wayland/Windows/macOS unperformed |
| Useful differentiation and bounded rows | Same basenames in northern/southern receiver directories and long names | Passed in production Xvfb in light/dark themes; Unicode label unit test passed | Arbitrarily similar names can still need full-path help at narrow widths |
| Correct tooltip after scrolling | Ten entries at 640×400, visible and pending hint, wheel beneath a fixed pointer in both directions | Passed in production Xvfb; fresh hover restores correct filename/directory/shortcut after the normal delay | X11 only |
| Normal activation remains | Pointer click, Alt+0 and File > Recent files | Click selected capture 4 and Alt+0 selected capture 10, checked against isolated saved paths; compact recent menu inspected in both themes | Native chooser not invoked because portal routing could leave the isolated display; existing automated action coverage retained |
| Themes and empty state | Light/dark and 0/1/10 rows, short windows | Passed in production Xvfb; existing geometry regression passed | The owner accepted the PR; native Wayland/Windows/macOS checks remain unperformed |

## Implementation steps

- [x] Reproduce defects in focused production-view regressions and inspect locked toolkit cause.
- [x] Separate start-page full labels from deliberately retained compact menu labels.
- [x] Let the recent group consume available width while retaining bounded content sizing.
- [x] Correct obsolete tooltip ownership using supported mechanisms.
- [x] Update architecture and changelog.
- [x] Complete local gate and production Xvfb acceptance, recording evidence and limits.
- [x] Prepare exact independent review handoff and address findings.
- [x] Move this plan to `docs/plans/completed/` after implementation, validation and independent review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after the checks above
- [x] Focused recent-label and production-view layout/tooltip regressions
- [x] Current production binary Xvfb interaction scenarios

### Recorded results

All 973 workspace tests plus doctests passed, including the production-view
regressions for narrow/wide layout, pending/visible wheel teardown and fresh-hover
recovery. Formatting and strict all-target Clippy passed. The release build ran
last and passed. The current release binary was exercised using task-owned Xvfb,
real X11 pointer/wheel/keyboard events, disposable WAVE captures, and isolated
supported XDG configuration/state/cache directories. Source/binary identity,
raw logs, scenario script and screenshots are retained in the independent review
packet outside the worktree. The main checkout and owner's application were not
modified. Independent review by `gpt-6.1-sol` high in a fresh read-only context returned no substantive findings in its first round. No findings were accepted or rejected. The coordinator assessed the exact reviewed diff, validation evidence and production visuals. The owner subsequently accepted the PR. Native Wayland/Windows/macOS checks remain unperformed.

## Full-CI completion follow-up

The first Ready full run exposed an existing optional-state assumption in
`reference_ui_test.rs`: after saving is cleared, `adopted` temporarily takes the
editing state before asynchronous reader rebind restores it. The reference-save
wait predicate unwrapped that absent state. The recent-page regressions passed
in that Linux run. macOS passed; Windows was cancelled when the PR returned to
Draft for this correction, so the run does not establish full validation.

- [x] Match the existing sibling save/rebind waits by checking `editing().is_some_and(...)`.
- [x] Refresh the complete local gate, with release build last.
- [x] Review the exact test-only correction in a second independent round.
Current full Linux/Windows/macOS CI success is required before squash merge.

The refreshed formatting, strict Clippy, all 973 tests plus doctests and release-last build passed. The same `gpt-6.1-sol` high reviewer completed the second round without substantive findings, and the coordinator assessed the exact correction and current evidence. No review finding was declined.

The change affects only the test wait predicate. Saved-reference and unchanged
sample-byte assertions remain in place; no product save or recent-file behavior
changes. No additional user-visible changelog entry is needed.

## Post-completion

After accepted merge, preserve any unique/untracked data and remove the requested
linked worktree with normal `git worktree remove`, following conditional branch
cleanup policy. No merge or Ready transition is authorized by this plan alone.
