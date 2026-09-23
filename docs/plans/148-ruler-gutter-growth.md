# Issue #148: Let the right ruler gutter only grow while zooming

Resolves [#148](https://github.com/o-kos/argand/issues/148).

## Overview

The right ruler's gutter changed width with zoom and was often much wider than
its labels.
- In horizontal orientation it reserved a worst case for the current frequency
  view: sign, every integer digit, and every decimal down to 1 Hz. The gutter
  therefore shrank when that worst case shrank, even as the placed labels grew
  (owner's example: `11` → `8,5`).
- In vertical orientation it followed the placed time labels exactly, so it
  widened and narrowed with every format change.

Size the gutter from the labels actually placed, and let it only widen while
the owner zooms and pans. Release the width only on explicit view changes.

Implementation class: **B**. It changes local GUI layout with concrete geometry;
per-document state lives in `PlotView`. It is implemented in-session at the
owner's request. The reviewer model is agreed with the owner before any review
round (interim process, #147).

## Context

- `axes.rs::ruler_gutter` reserved `widest_labels(Frequency, current view)` in
  horizontal orientation, and the placed right-hand tick labels in vertical
  orientation. Both orientations add all unit captions and `LABEL_PAD`.
- `Frame::measure_view` took the held time and frequency tick schemes as
  separate arguments.
- `PlotView` (#128) owns the measured layout and is created per document, so a
  new file resets any held layout state by construction.
- `Shell::measure_time_scheme` measures a frame of its own to hold the time
  scheme. It must see the same gutter as the painted frame, or the plot width,
  and with it the scheme, would differ.
- The CLI keeps `argand_core::axis::widest_labels` for its own layout. It is
  unchanged.

## Decisions

The owner rejected a fixed budget with shared-prefix labels as not obvious, and
chose a gutter that only grows.

- `ruler_gutter` uses the placed right-hand labels in both orientations, plus
  every unit caption and `LABEL_PAD`. There is no worst-case reservation.
- `axes::Held { time, frequency, gutter }` replaces the separate held-scheme
  arguments. `Frame::measure_view` takes the larger of the measured gutter and
  `Held::gutter`, and reports its result in `Frame::gutter`.
- `PlotView::gutter_floor` keeps the widest gutter laid out. The canvas passes it
  into measurement, and the deferred layout raises it. A wider label set
  therefore widens the ruler once and keeps it.
- The floor is released (set to zero, then re-measured from the current labels):
  - on a new file, because a new `PlotView` is created;
  - on an orientation change (`PlotView::reorient`);
  - on a time-format change while time is on the right;
  - when the owner fits the axis shown on the right: Ctrl+0 / View → Fit time in
    vertical orientation, Ctrl+Shift+0 / View → Fit frequency in horizontal
    orientation.
- Growth changes the display size only, through the existing coalescing
  mailbox, and never starts a new analysis generation.
- `Shell::measure_time_scheme` passes the same floor, so held schemes match the
  painted layout.

## Rejected alternatives

- Fixed budget with shared-prefix or offset labels: not obvious to read (owner).
- Worst-case reservation for maximum zoom: very wide from the start.
- Keeping the current per-view worst case: wider than the labels, and it changes
  in the opposite direction to them.

## Implementation steps

- [x] Size the gutter from the placed labels and introduce `axes::Held` with the
      gutter floor.
- [x] Keep and release the floor in `PlotView`, and pass it to
      `measure_time_scheme`.
- [x] Tests:
  - the gutter fits the placed labels without a worst-case reserve;
  - a held floor keeps the ruler from narrowing while wider labels still widen
    it (both orientations);
  - fitting the right axis and reorienting release the floor, while fitting the
    other axis does not.
- [x] Update AGENTS.md and CHANGELOG.
- [ ] Complete validation and move this plan to `docs/plans/completed/`.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Native, both orientations:
  - the gutter is narrow on open;
  - it widens at most once while zooming in, and stays while zooming out and
    panning;
  - it narrows on a fit of the right axis, and does not narrow on a fit of the
    other axis;
  - it narrows on an orientation switch, a time-format switch with time on the
    right, and opening another file.

## Post-completion

None.
