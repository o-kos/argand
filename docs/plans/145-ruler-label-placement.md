# Issues #145 and #141: Ruler label placement and aligned Alt badges

Resolves [#145](https://github.com/o-kos/argand/issues/145) and
[#141](https://github.com/o-kos/argand/issues/141).

## Overview

In horizontal orientation a time label starts 9 logical pixels right of its tick,
and its ink starts about 7 pixels below the tick's end. Each label reads as
floating between two ticks. Place each label so it hangs beside its own tick, and
give the space this frees to the spectrogram.

Implementation class: **B**. It changes one GUI module with local invariants and
concrete geometry. At the owner's request the change is implemented in-session.
`gpt-5.6-terra` high reviews it before owner review.

## Context

- `axes.rs::Frame::measure_view` reserves the bottom band as
  `OUTER_PAD + row_height + LABEL_PAD` and centres the bottom label row at
  `plot.bottom() + LABEL_PAD + row_height / 2`. Horizontal time labels use
  `LabelMetrics::after_tick(LABEL_PAD)`, and `axes::paint` starts them at
  `x + LABEL_PAD`. Ticks are 1 pixel wide and `TICK_LEN` (6) long.
- The Alt time badge (`cursor_guides.rs`) and the unit hint rectangles use the
  full label row (`LINE_HEIGHT` / `row_height`) centred on `time_row`. The whole
  row must therefore stay inside the panel.
- In vertical orientation the bottom row carries frequency labels centred under
  their ticks. Raising that row would put the 6-pixel tick through the digits.

## Decisions

The owner chose this placement from a rendered comparison of four candidates:
current, a long-tick flag, labels centred under their ticks, and adjacent
placement.

- In horizontal orientation, clear space is 4 pixels from the tick to the label
  (`TIME_LABEL_GAP`), and 4 pixels from the ruler line to the top of the digits'
  ink. The label start after the one-pixel tick is `TIME_LABEL_START` = 5. The
  6-pixel tick stays, so the digits begin beside its lower part.
- The bottom band reserves the label's full row below that ink position plus
  `OUTER_PAD`. This keeps badges and unit hints inside the panel and gives the
  plot about 8 pixels more height.
- Tick spacing and edge-mark behaviour keep the same `after_tick` policy. Only its
  offset changes, so panning and edge clipping behave as before.
- Unchanged:
  - vertical orientation;
  - the right-hand rulers, whose labels already sit 3 pixels after the tick end;
  - the unit captions' horizontal position;
  - the CLI (`aspec`), which keeps centred labels from the shared default.

### Final label row, after the owner's visual check

The owner judged the ruler too low with the ink 4 pixels under the line, and
confirmed that the badge text and the labels share one row (the apparent offset
was optical). The label ink moves 2 pixels lower (`TIME_LABEL_DROP` = 6). The
band below the plot is `1 + 2 × TIME_LABEL_DROP + ink`, and the ink is centred in
the band that remains after device-pixel rounding. An Alt badge (`BADGE_PAD` = 3
around the ink) therefore has equal room to the ruler line and to the band's
bottom edge, 3 logical pixels each when rounding adds nothing. The horizontal
4-pixel gap after the tick is unchanged. This replaces the earlier "4 pixels below
the line" rule.

### Alt badges (#141), added after owner review

Owner review showed the horizontal Alt badge covering the ruler line. The badge
box was a full `LINE_HEIGHT` row centred on the raised label row, so its top came
within half a pixel of the line. The owner asked to close #141 in this Pull
Request.

- `cursor_guides::badge_rects` derives both badge rectangles from the measured
  frame. Each box wraps the digits' ink with `BADGE_PAD` (3) on every side.
- The bottom badge is centred on `Frame::time_row`, the bottom ruler's text row.
  The badge text uses the same `Labels::line_top` as the ruler labels, and the
  former +1 pixel optical correction is removed. In horizontal orientation this
  leaves 1 clear pixel below the ruler line.
- The right badge is centred on the drawn guide line (the device-snapped pointer
  plus half a pixel), as right-hand labels are centred on their ticks. It ends at
  the panel edge, `OUTER_PAD` past the reserved label column, which is #141's
  4-pixel rule.
- The fixed right-gutter width and deep-zoom label shortening raised in the same
  review are split out to #148.

## Rejected alternatives

- Centred under the tick (the CLI's default): at the plot edges a centred label
  overhangs by half its width into the 4-pixel margin or the frequency gutter. It
  is then emptied by the edge-mark rule, so end labels flicker in and out while
  panning.
- A long tick running down to the label baseline: clear, but heavier than needed
  once the label sits right beside the tick.
- Moving only horizontally (3 pixels) with the label below the tick's end: the
  label still hangs diagonally off the tick.

## Implementation steps

- [x] Add `TIME_LABEL_GAP` / `TIME_LABEL_START`, and use them in the horizontal
      layout, paint and tests.
- [x] Make the bottom band and row centre depend on orientation, leaving vertical
      geometry unchanged.
- [x] Add a geometry test for both gaps, the in-panel row and the unchanged
      vertical row.
- [x] Update AGENTS.md and CHANGELOG.
- [x] Derive Alt badge rectangles from the frame, sharing the label rows (#141).
- [x] Test badge rows, ruler-line clearance, the right-edge margin, clamping,
      both orientations, all time modes and fractional scales.
- [x] Lower the label row to `TIME_LABEL_DROP` and centre it in the band so the
      badge has equal room above and below. Verified on an X11 capture at 1.25×.
- [ ] Complete validation and move this plan to `docs/plans/completed/`.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Native:
  - horizontal clock, seconds and samples rulers at 1× and a fractional scale;
  - labels near both plot edges while panning;
  - Alt badges on both rulers in both orientations, including near plot corners;
  - the time unit hint;
  - vertical orientation unchanged.

## Post-completion

None.
