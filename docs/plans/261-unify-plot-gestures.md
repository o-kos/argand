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
| Spectrogram wheel selects time or frequency correctly | Default/Shift pan and Ctrl/Ctrl+Shift zoom in both orientations | Pending | Native platforms separate |
| Rulers operate on their own axis | Modifier combinations for wheel, selection, resize, Alt and pan | Pending | Native platforms separate |
| Selection creation and modification | Time, band, rectangle, no selection, edge/interior, Alt, modifier release | Pending | Capture samples remain immutable |
| Existing navigation/focus paths remain usable | Keyboard/menu zoom, fit, arrows, Escape and Alt guides | Pending | Native interception reported separately |
| Production UI matches the agreed scheme | Isolated Xvfb production release run with disposable fixtures and settings | Pending | Does not establish Wayland, Windows or macOS input behavior |
| Discoverable help matches gestures | README/current navigation documentation and existing help surface | Pending | No new custom widget |

## Implementation steps

- [ ] Correct wheel routing and mouse-down selection priorities.
- [ ] Add time-ruler edge/Alt support and retain latched gesture semantics.
- [ ] Add focused event-level and geometry regression coverage.
- [ ] Update discoverable help, current navigation documentation and changelog.
- [ ] Complete local gate and isolated production UI acceptance.
- [ ] Complete independent review and coordinator assessment.
- [ ] Move this plan to completed before final review, once acceptance is complete.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the preceding checks
- [ ] Production Xvfb interaction scenarios, bound to exact revision and binary

## Post-completion

Merge and branch cleanup require the coordinator/owner completion decision and
current full CI. Release and tagging are outside this task.
