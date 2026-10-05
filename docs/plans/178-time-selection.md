# Issue #178: Select a time range on the spectrogram

Resolves #178.

Class A (gestures and shared state across more than five code files). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

The first step of phase 5 in `IMPLEMENTATION_PLAN.md`. A left drag on the spectrum selects a time range; panning the spectrum moves to the middle button and Space+left drag. The selection is drawn on the spectrum and the minimap and described in the status bar. Band and rectangle selection, saving and editing are later steps.

## Context

- `plot_view.rs`: `PlotView` owns the gesture state (`pan`, `frequency_pan`), the `drag_tracker` that follows a drag beyond the plot, and emits `PlotIntent`s that `Shell::apply_plot_intent` (`navigation_ui.rs`) handles.
- `navigation_ui.rs`: `begin_pan` starts every drag from a left press, `pointer_moved` continues it and uses `MouseMoveEvent::dragging()`, which GPUI ties to the left button only. `PlotGeometry::cursor` picks the cursor; `PlotGeometry::fractions` maps a point to time and frequency fractions in either orientation.
- `plot_ui.rs` paints the spectrum canvas and builds `waveform::Panel` for the minimap; `waveform.rs` paints it with `minimap::viewport`.
- `settings_ui.rs` (`Shell::status_bar`) shows the cursor readout; `cursor_guides.rs` formats time in the ruler's mode.
- Nothing in the application handles the middle button or Space today.

## Decisions

- `argand_core::selection`: `SampleSpan` (whole samples, `start < end`, built from two boundaries in either order), `FrequencyBand` (hertz, not created yet) and `Selection { time, band }`. A complex sample counts once, so I and Q stay together.
- A pointer maps to a sample boundary as `view.start + round(fraction × view.len)`, the fraction clamped to the plot, so a drag beyond the plot stops at the visible edge. This lives in `navigation.rs` (`View::boundary`), free of GPUI.
- `PlotView` gains a `Selecting { anchor, origin, moved }` gesture beside `pan`, and remembers which button drives a pan so a middle-button pan is continued by `pressed_button`, not `dragging()`.
- A press decides the gesture once: middle button, or left with Space held, pans exactly as the left button does today; left on the spectrum without Ctrl selects; left with Ctrl on the spectrum does nothing (reserved for the rectangle); left on the rulers and the minimap pans as today.
- A left press and release that moved less than 3 logical pixels is a click and clears the selection; Escape on the focused plot clears it too.
- Space is followed with key down and key up on the plot surface and forgotten whenever the plot ends its gestures (blur, overlays, deactivation), so a release elsewhere cannot leave it stuck. While Space is held the spectrum shows an open hand.
- `PlotIntent::Select(Option<SampleSpan>)` reports the selection while dragging; Shell keeps it in `Shell::selection`, clears it when a file opens, and passes it to the plot in `PlotSnapshot` as fractions of the view (`View::fractions_of`), which keeps the spectrogram painter within its line limit.
- The band is the theme's `blue_light` at 0.3 opacity, because the spectrum and minimap are dark in both themes; it is over the picture, below the grid and guides, across the whole frequency extent, clipped to the plot; the minimap draws the same band under its waveform.
- The status bar shows `start – end (duration)` in the time ruler's format beside the cursor readout while a selection exists.

## Rejected alternatives

- A toolbar tool switch (hand, time, band, rectangle): modal, and the owner chose gestures by place.
- Tracking Space in the keystroke interceptor: it sees key down only, and a key up is still needed.
- Autoscrolling while a selection is dragged beyond the plot: left for later; the view can be panned with the wheel during the drag.

## Implementation steps

- [x] `argand_core::selection` with tests.
- [x] `View::boundary` with tests.
- [x] Gesture split in `PlotView`: selection, middle-button and Space pans, click and Escape clearing, cursor.
- [x] `PlotIntent::Select`, `Shell::selection`, reset on file opening, snapshot.
- [x] Drawing on the spectrum and the minimap.
- [x] Status-bar readout.
- [x] Headless tests for the gestures; existing left-drag pan tests moved to the new gestures.
- [x] Update `AGENTS.md` and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The owner checks the release binary: selecting in both orientations, clearing, panning with the middle button and Space.

## Post-completion

None.
