# Issue #183: Show a selection's start, end and length on the plot, and handle it there

Resolves #183 (with #197 folded in).

Class A (expected diff above 400 lines, gestures across `navigation_ui.rs`, `plot_view.rs`, `plot_ui.rs` and the shell). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, to be confirmed by the owner before the first round.

## Overview

A time selection carries its own readouts on the plot, as ocenaudio does: the start and the end in framed labels at the selection's edges, the length in a framed label centred over it. The selection is also handled on the plot: its edges are dragged, a drag inside moves it, and Shift+click extends it.

## Owner decisions

- The length label is a readout only, without buttons.
- The status bar drops the time part of the selection readout; the band's edges and width stay there.
- The labels are not editable.
- Gestures in this Issue: dragging the time edges (spectrogram and minimap), Shift+click extending from the nearer edge, a drag inside the selection moving it whole, and dragging the frequency edges of a band or a rectangle (spectrogram and frequency ruler).

## Context

- `Shell::selection` (a `SampleSpan`) and `Shell::band` (a `FrequencyBand`) hold the selection; `PlotView` only receives it as fractions (`PlotSnapshot::selection`, `waveform::Selected`) and reports gestures as `PlotIntent::Select`.
- `PlotView::press` decides a gesture once per press (`Selecting` with time and frequency anchors); `View::boundary` snaps to whole samples and stops at the visible edge; a press that moves under 3 pixels is a click and clears the selection.
- `time_ruler::Mode::selection` formats start, end and length exactly to the sample for the status bar.
- `cursor_guides.rs` paints the Alt badges in the axis font with tabular figures (`axes::Labels`), ink fill inside a paper ring.

## Decisions

### Labels

- Three badges in the Alt badge look (ink fill, paper ring, axis font, tabular figures): start, end and length. The start badge sits on the low-frequency side of the plot with its far edge on the selection's start edge, outside the selection; the end badge likewise outside the end edge. The length badge sits on the high-frequency side, centred over the visible part of the selection. Horizontal orientation puts start and end at the bottom of the plot and the length at the top; vertical orientation, with time down and frequency to the right, puts start and end on the left at the edges' rows and the length on the right.
- Values come in the time ruler's mode, exact to the sample, from `time_ruler::Mode::selection`'s rules split into three values. Each value is padded on the left to the width the capture's own length takes in that mode and precision, digits of the padding drawn as `0`; the leading run of zeros and separators, up to the first significant digit and never the units digit before the decimal mark, is drawn in a dimmed ink. The width therefore does not change while an edge is dragged.
- An edge outside the visible time has no badge. The badges are clamped inside the plot; where the start and end badges would overlap, the end badge moves past the start badge and is dropped when the plot has no room for it. The length badge is dropped when it is wider than the plot.
- The badges take no input and are painted over the selection tint and the grid, under the corner zoom pairs, the Alt guides and their badges.
- The status bar shows only the band part of the readout.

### Gestures

- Grab zones, in this priority: the corner zoom pairs and unit captions as now; a selection edge within 4 logical pixels; the inside of the selection; anywhere else.
- Time edges: on the spectrogram across the selection's frequency extent, and on the minimap. The pointer shows the horizontal resize cursor (vertical in vertical orientation); a drag moves that edge, snapped by `View::boundary`, and dragging it past the other edge swaps them.
- Frequency edges of a band or a rectangle: on the spectrogram across the selection's time extent, and on the frequency ruler. Resize cursor across the frequency axis; the edge follows the pointer in hertz, clamped to the capture's band, swapping as above. Where a time edge and a frequency edge meet, the corner drags both, with a diagonal resize cursor.
- Inside the selection on the spectrogram, the pointer shows an open hand and a drag moves the whole selection, its length and width kept, clamped to the capture's time and band; the cursor closes while moving. A click there without moving still clears the selection. The minimap keeps its viewport drag inside the selection; only its edges are grabbed there.
- Shift+click on the spectrogram moves the nearer time edge, and with a band also the nearer frequency edge, to the pointer; Shift+click on the frequency ruler moves the nearer band edge. Holding the button after Shift+press continues as that edge drag.
- None of these is an edit: they report `PlotIntent::Select` as a drag does now. Space+drag and the middle button still pan everywhere.

## Rejected alternatives

- Labels inside the selection: they would cover the selected picture and collide in a narrow selection.
- A separate move cursor: GPUI has none; the open and closed hands already mean "drag this".

## Implementation steps

- [x] `time_ruler`: split the selection readout into start, end and length counters with their padded width and dimmed prefix; tests.
- [x] Shell: the selection in samples and hertz and the counters in `PlotSnapshot`; the status bar without the time part.
- [x] Painting the three badges with their placement rules; placement tests.
- [x] Gestures: grab zones and cursors, edge drags for time and frequency, corners, moving, Shift+click, on spectrogram, minimap and frequency ruler; headless tests.
- [x] `AGENTS.md` and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Manual, release binary: labels in both orientations and all three ruler modes, a narrow selection, edges off screen; dragging time and frequency edges and corners, swapping edges, moving a selection, Shift+click, on spectrogram, minimap and frequency ruler.

## Post-completion

None.
