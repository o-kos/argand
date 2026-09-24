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

### Bottom ruler in vertical orientation, added during owner review

In vertical orientation the bottom ruler carries frequency. It kept the old
placement: labels centred under ticks, a row 9 pixels below the line, and a
different band height. The owner asked for it to match the horizontal time
ruler from #145, in this Pull Request.

- `bottom_band` and `bottom_row_center` no longer branch on orientation. Every
  bottom ruler reserves `1 + 2 × BOTTOM_LABEL_DROP + ink` and centres its ink in
  the band.
- Bottom labels use `after_tick(BOTTOM_LABEL_START)` in both orientations. Tick
  spacing and edge marks follow the same policy as the horizontal time ruler.
- `right_label_room` measures the vertical time labels' clearance against the
  new bottom caption row, so they still clear the frequency unit.
- The constants are renamed `BOTTOM_LABEL_*`: they now describe the bottom
  ruler, not the time axis.

### Zoom direction, added during owner review

The owner asked that zooming in may only widen the ruler, and zooming out may
only narrow it. Zooming out can still lengthen labels: a clock format gains
hours at a one-hour span, and a frequency unit or integer digit can grow. So
zooming out lets the ruler refit its new labels rather than forbidding growth,
because forbidding it would clip labels.

- `PlotView::view_intent` is the single path for time and frequency intents,
  covering keys, wheel, zoom buttons and menu fits. It releases the hold when
  the axis on the right zooms out (factor > 1) or is fitted.
- Zooming in, panning, minimap steps and the other axis keep the hold.

### Bottom foot, added during owner review

The owner found the bottom gap too small. The two gaps were geometrically equal
(7 device pixels each at 1.25×). The ruler digits still sat closer to the status
bar than the status bar's own text does (about 9 device pixels), and
geometrically centred text reads low. The top stays at 6 pixels
(`BOTTOM_LABEL_DROP`). The space below the ink becomes 8 (`BOTTOM_LABEL_FOOT`),
and rounding surplus is shared between the two gaps. This was compared on real
captures at 1.25× before the change. The Alt badge is no longer equidistant
(3 above, 5 below). The owner accepted that: the badge is transient and belongs
to the ruler line.

### Badge ring, added during owner review

The owner saw the right badge as about 16 px tall against 18 for the bottom one.
Screenshots showed both at 18 px, with the text in the same rows. The right badge
lay over the bright spectrogram, and its anti-aliased white edge merged into the
light background. Each badge now gets a one-pixel ring in the paper colour
(`BADGE_RING`), painted before the guide lines, which then run over it into the
fill. The ring is invisible on the dark ruler and defines the edge on bright
pictures. Size, text row and the 6/8 geometry are unchanged. This was checked on
a 1.25× capture.

### Guide colours and right badge margin, added during owner review

- The guide lines swap to the badge's colour sequence: a paper-coloured 3-pixel
  stroke around an ink core (black–white–black in the dark theme). They are
  taken from the theme, so the light theme stays consistent with its dark
  badges. The fixed white–black–white is gone. The owner accepted it in the
  native window.
- The right badge now stands as far from the panel's right edge as the bottom
  badge stands from the bottom ruler's edge. The margin is taken from the bottom
  badge's actual position, so rounding surplus matches too. This replaces #141's
  "4 pixels past the ruler labels" rule at the owner's request.

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
  - zooming out or fitting the right axis and reorienting release the floor;
  - zooming in, panning and the other axis keep it.
- [x] Update AGENTS.md and CHANGELOG.
- [x] Match the vertical-orientation bottom (frequency) ruler to the horizontal
      time ruler, and test both orientations and the vertical time clearance.
- [x] Give the bottom labels an 8-pixel foot (6 above), tested in both
      orientations and for the badge.
- [x] Ring the Alt badges in the paper colour so they keep their edge over bright
      pictures.
- [x] Paint guide lines in paper/ink, and give the right badge the bottom
      badge's edge margin (tested).
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
  - it refits on zooming out or fitting the right axis, and does not narrow on
    zooming in, panning or any action on the other axis;
  - it narrows on an orientation switch, a time-format switch with time on the
    right, and opening another file;
  - vertical-orientation frequency labels sit beside their ticks, with the Alt
    badge centred in the band.

## Post-completion

None.
