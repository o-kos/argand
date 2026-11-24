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
A production-view regression must establish the actual failing transition before
selecting a correction. Installed toolkit/SDK sources remain immutable.

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
| Full filename when it fits | Long names at narrow and wide widths, including window resize | Pending | Production Xvfb and rendered geometry required |
| Useful differentiation and bounded rows | Same basenames in distinct directories, Unicode and long names | Pending | Keep filename/extension and full-path recovery |
| Correct tooltip after scrolling | Ten entries at 640×400, show hint, wheel while pointer stays fixed, repeat both directions | Pending | Old tooltip must disappear or update, normal hover delay remains |
| Normal activation remains | Pointer click, Alt+digit and chooser; File menu recent consumer | Pending | Preserve standard Button behavior and opening hints |
| Themes and empty state | Light/dark and 0/1/10 rows, short windows | Pending | Native Wayland/Windows/macOS not established by Xvfb |

## Implementation steps

- [ ] Reproduce defects in focused production-view regressions and inspect locked toolkit cause.
- [ ] Separate start-page full labels from deliberately retained compact menu labels.
- [ ] Let the recent group consume available width while retaining bounded content sizing.
- [ ] Correct obsolete tooltip ownership using supported mechanisms.
- [ ] Update architecture and changelog.
- [ ] Complete local gate and production Xvfb acceptance, recording evidence and limits.
- [ ] Prepare exact independent review handoff and address findings.
- [ ] Move this plan to `docs/plans/completed/` before final review after acceptance is complete.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after the checks above
- [ ] Focused recent-label and production-view layout/tooltip regressions
- [ ] Current production binary Xvfb interaction scenarios

## Post-completion

After accepted merge, preserve any unique/untracked data and remove the requested
linked worktree with normal `git worktree remove`, following conditional branch
cleanup policy. No merge or Ready transition is authorized by this plan alone.
