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
Shift starts a new band and Ctrl+Shift a rectangle before edge/interior grabs.
Alt adjusts existing nearest boundaries, with no selection yielding no gesture.
Unmodified presses retain direct edge resizing and interior movement. Rulers select,
resize or adjust their own physical axis. Wheel modifiers never redirect a ruler.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits |
| --- | --- | --- | --- |
| Spectrogram wheel selects time or frequency correctly | Default/Shift pan and Ctrl/Ctrl+Shift zoom in both orientations | Passed automated tests and production Xvfb run | Native platforms separate |
| Rulers operate on their own axis | Modifier combinations for wheel, selection, resize, Alt and pan | Passed automated modifier matrix and production Xvfb scenarios | Native platforms separate |
| Selection creation and modification | Time, band, rectangle, no selection, edge/interior, Alt, modifier release | Passed automated tests and fitted/zoomed production Xvfb scenarios | Disposable capture matches regenerated original bytes |
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
- [ ] Complete independent review and coordinator assessment.
- [ ] Move this plan to completed before final review, once acceptance is complete.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked` (936 passed after the minimap correction)
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
The first review identified modifier handling that also affected the minimap.
Restored its earlier Ctrl/Shift navigation priority and Alt navigation/edge grabs.
A new regression failed before the correction and passed afterwards. The complete
local gate passed again, and the rebuilt production binary passed the affected
minimap scenarios in both orientations plus selection/Alt smoke checks. The
unaffected broad interaction record is retained separately from this refresh.

## Post-completion

Merge and branch cleanup require the coordinator/owner completion decision and
current full CI. Release and tagging are outside this task.
