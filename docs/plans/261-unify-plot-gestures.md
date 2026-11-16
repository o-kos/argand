# Issue #261: Unify plot navigation and selection gestures

Resolves #261.

## Overview

Use one physical-axis gesture scheme on the spectrogram and both rulers. Preserve
capture data, history, keyboard/menu commands, coordinate guides and the minimap.

## Roles and risk

- Class: A, because the interaction and regression-test change may exceed 400 lines.
- Coordinator (primary): `gpt-6.1-sol` medium, owner communication and final assessment.
- Executor: `gpt-6.1-sol` medium, implementation, validation and PR operations.
- Reviewer: `gpt-6.1-sol` high, read-only in a fresh context.
- Consultant: `gpt-6-astra` medium only if a concrete unresolved question warrants it.

## Context and decisions

Existing PlotView owns transient gestures and emits toolkit-neutral selection and
viewport intents. Reuse it and existing canvas interactions without new controls,
new dependencies or changes to DSP, sample ownership, analysis or rendering lifetime.
The prerequisite role-policy documentation commit remains preserved in this branch.

Choose the gesture at mouse-down. Space/middle pan wins. On the spectrogram,
Shift or Ctrl alone starts a new band and Ctrl+Shift a rectangle before edge/interior grabs.
Alt adjusts existing nearest boundaries, with no selection yielding no gesture.
Unmodified presses retain direct edge resizing and interior movement. Rulers select,
resize or adjust their own physical axis. Wheel modifiers never redirect a ruler.

The owner approved Ctrl+left drag as an alias for Shift+left drag during Draft
feedback. Plain left remains time selection. Refresh the affected gesture,
cursor and discoverability evidence in both orientations, including release of
modifiers over an existing selection. Preserve the reviewed minimap behavior.

Draft feedback also reproduced a pan dead zone after overshooting either capture
bound: the original pointer/view anchor retained the overshoot. Rebase only the
bounded axis to its actual view and current pointer, preserving other-axis movement,
fitted axes, ruler ownership and minimap mapping. Verify immediate reversal for
middle and Space+left in both orientations, including pointer travel off canvas.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits |
| --- | --- | --- | --- |
| Spectrogram wheel selects time or frequency correctly | Default/Shift pan and Ctrl/Ctrl+Shift zoom in both orientations | Passed automated tests and production Xvfb run | Native platforms separate |
| Rulers operate on their own axis | Modifier combinations for wheel, selection, resize, Alt and pan | Passed automated modifier matrix and production Xvfb scenarios | Native platforms separate |
| Selection creation and modification | Time, band, rectangle, no selection, edge/interior, Alt, modifier release | Passed automated tests and fitted/zoomed production Xvfb scenarios | Disposable capture matches regenerated original bytes |
| Ctrl and Shift band aliases | Existing rectangle, modifier release, Ctrl+Shift precedence, rulers, cursor and help in both orientations | Passed refreshed automated and production Xvfb checks | Owner acceptance remains pending |
| Pan reversal at capture bounds | Both bounds, spectrum and each ruler, both orientations, middle/Space, fitted axes and off-canvas travel | Regression failed before correction and passed after it; refreshed production Xvfb scenarios passed | No native desktop or platform-wide claim |
| Existing navigation/focus paths remain usable | Keyboard/menu zoom, fit, arrows, Escape and Alt guides | Passed automated tests and production Xvfb scenarios, including Edit menu selection panel | Window-manager interception remains untested |
| Production UI matches the agreed scheme | Isolated Xvfb production release run with disposable fixtures and settings | Passed scoped scenarios on 2026-11-16 | Does not establish Wayland, Windows or macOS input behavior or owner acceptance |
| Discoverable help matches gestures | README/current navigation documentation and existing help surface | Updated and production selection tooltips checked | No new custom widget |

## Implementation steps

- [x] Correct wheel routing and mouse-down selection priorities.
- [x] Add time-ruler edge/Alt support and retain latched gesture semantics.
- [x] Add focused event-level and geometry regression coverage.
- [x] Update discoverable help, current navigation documentation and changelog.
- [x] Complete local gate and isolated production UI acceptance.
- [x] ➕ Preserve modified minimap navigation and Alt edge resizing after the independent review finding, then refresh affected validation.
- [x] ➕ Add the owner-approved Ctrl band-selection alias, matching cursor/help, focused regressions and current production evidence.
- [x] ➕ Correct and validate immediate pan reversal after overshooting either axis boundary.
- [ ] Complete independent review and coordinator assessment.
- [ ] Move this plan to completed before final review, once acceptance is complete.

## Validation

The checked results below include the Ctrl alias and pan-boundary feedback.

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked` (937 passed after the Ctrl alias and pan correction)
- [x] `cargo build --release --locked`, after the preceding checks
- [x] Production Xvfb interaction scenarios, bound to exact revision and binary

The production run used a disposable 1 MHz complex chirp and isolated settings,
state, cache and X11 display. Both orientations were exercised with fitted and
zoomed axes. Fast single-move synthetic drags did not consistently establish a
selection in the vertical run, so those attempts are excluded from passing
evidence. Paced drags with intermediate moves established the recorded results.
This run does not diagnose the earlier host freeze. Independent review completed
in two rounds, with the first finding corrected and the second round clean. The
coordinator's assessment and owner acceptance remain pending; keep the PR Draft.
The approved Ctrl alias and pan-boundary refinements passed a refreshed sequential
local gate and production release run. The additional pan regression covers both
bounds, orientations, spectrum/rulers and middle/Space+left input, including travel
beyond the canvas. Production cursor observations match Ctrl to Shift selection
feedback. The third independent review round returned no substantive findings
on these refinements and confirmed the earlier minimap finding remains resolved.
No findings were declined. Coordinator assessment and owner acceptance remain
pending; the accumulated review count is three rounds.
The first review identified modifier handling that also affected the minimap.
Restored its earlier Ctrl/Shift navigation priority and Alt navigation/edge grabs.
A new regression failed before the correction and passed afterwards. The complete
local gate passed again, and the rebuilt production binary passed the affected
minimap scenarios in both orientations plus selection/Alt smoke checks. The
unaffected broad interaction record is retained separately from this refresh.

## Post-completion

Merge and branch cleanup require the coordinator/owner completion decision and
current full CI. Release and tagging are outside this task.
