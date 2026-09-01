# Issue #9: Increase axis tick density without overlapping labels

Resolves #9.

## Overview

Every axis in `aspec` asks `nice_ticks` or `nice_time_ticks` for a fixed number of
marks -- 8 for a horizontal time axis, 6 for a vertical one, 6 for frequency, 3 or 4 for
decibels, 5 for the colour bar -- and those numbers do not know how long the axis is. A
2048-pixel time axis gets the same eight labels as a 400-pixel one, so a wide render is
mostly empty; a narrow one, or a capture whose frequency labels read `12.579887 MHz`,
puts labels on top of each other and off the edge of the canvas.

This change replaces the fixed targets with a measured policy: format the candidate
labels, measure them with the embedded font at the size the renderer draws with, and
take the densest round step whose labels still keep a documented gap and stay inside the
canvas. Time labels gain a fixed clock format, and the gutters that hold stacked labels
are sized from what those labels actually measure instead of from three constants that
predate the frequency formatter.

Minor unlabelled grid lines stay out of scope. Every grid line drawn is a labelled major
tick, or the same value on a panel that shares the axis.

## Context

- `crates/cli/src/render.rs` owns `Layout` and every drawing routine, including
  `nice_ticks`, `nice_time_ticks`, `ticks_from_step` and `time_label`. Nothing outside
  that file calls them.
- Six axes are drawn: time (horizontal and vertical), frequency (horizontal and
  vertical), the spectrum panel's decibels (horizontal and vertical) and the colour
  bar's decibels (vertical only).
- `crates/cli/src/text.rs::TextRenderer::width` already measures a string with the
  embedded DejaVu Sans at a given size, so the measurement the Issue asks for needs no
  new dependency.
- `Layout::compute` runs before the transform, because the transform is asked for
  exactly the pixels the spectrogram will occupy. It currently reserves gutters with
  `FREQ_LABEL_W = 78`, `DB_LABEL_W = 44` and `CBAR_LABEL_W = 60`. A frequency label at
  12.579887 MHz measures about 110 pixels, so on a real HF capture the left gutter
  already overflows past the canvas edge.
- `SignalMeta::frequency_span` and the resolved `SampleRange` give the frequency and
  time extents before the transform runs, so the gutters can be measured in
  `main.rs::process` and handed to `Layout::compute`.
- Panels that share an axis share its pixel extent exactly: the waveform strip has the
  spectrogram's `x`/`w` when time runs across and its `y`/`h` when time runs down, and
  the spectrum panel has the spectrogram's frequency extent in both orientations.

## Decisions

- Tick selection moves to a new `crates/cli/src/ticks.rs`. It stays in `argand-cli`
  rather than `argand-core` because it measures glyphs: the policy is inseparable from
  the font and size the renderer draws with, and `AGENTS.md` keeps presentation out of
  the core crates.
- The engine takes an `Axis` (pixel length, value range, and how far a label may
  overhang each end), an `AxisKind` (what the axis prints, which fixes both its ladder
  and its labels) and a `LabelRun` (whether labels sit side by side along the axis or
  stack across it). It returns the accepted `Tick`s with their value, label and pixel
  offset, so the caller cannot draw a grid line the label policy did not accept.
- The minimum gap between two labels is the width of two digits at the label size,
  measured from the embedded font. Two digits is the smallest gap at which neighbouring
  values read as two numbers rather than one, and deriving it from the font means it
  scales with `FONT_SIZE` instead of being a constant that silently stops being right.
- A stacked label's extent along its axis is the ink height of a digit, measured from
  the font, not the font's line height. Digits have neither ascender nor descender, so
  the line height would reserve half again more room than a row of numbers occupies.
  The baseline offset used when drawing is derived from the same measurement, so the ink
  really is centred on the tick and the overlap model matches what is drawn.
- Candidate steps run densest first and the first acceptable one wins. Decimal axes use
  `1`, `2` or `5` times a power of ten; time axes use a clock ladder of whole seconds
  (1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 10800, 21600, 43200,
  86400, then doubling days).
- Tick values are `k * step` for integer `k`, not a running sum, so a tick at zero is
  exactly zero and the last tick has not drifted by an accumulated epsilon.
- A step is rejected when two adjacent labels come out identical. That is one rule
  instead of a per-axis resolution floor: it stops a 0.5 dB step printing `-60` twice
  and a sub-hertz step printing the same megahertz value twice, and it keeps working if
  a formatter changes.
- Labels whose ink would leave the canvas or its gutter are dropped, and their grid
  lines with them, rather than being clipped or nudged. A missing outermost label is
  legible; half a label is not.
- ➕ Grid lines in the spectrum panel are drawn before its trace rather than after. With
  the panel's frequency grid added, drawing the grid last cut the trace into dashes
  exactly where it is read.
- ➕ The slack that keeps a tick landing exactly on an end of the range is measured in
  multiples of the step, not in seconds or hertz, and is capped at a thousandth of a
  multiple. Expressed in values it has to be wide enough to cover the division at one
  end of the axis, and that is wide enough to reach past a narrow range at the other and
  return a coordinate outside it -- a one-hertz window a terahertz up gained a tick a
  whole hertz below its own minimum. The cap bounds what is left: an end tick may sit up
  to `step / 1024` outside the range, which is under a pixel on any axis, and an index
  can never move by a whole multiple and inflate the count the search rejects on. Both
  ends are also checked against `2^53` before they are cast, since past that an index no
  longer round-trips through `f64`.
- ➕ The decimal ladder holds its decade inside the normal range. Below it `powi`
  underflows to zero, and a decade of zero neither yields a step nor grows when it is
  multiplied, so a span too small to label at all sent the search spinning for ever.
- ➕ The clock ladder keeps doubling days until it has passed the step it was asked to
  start from, instead of falling back on its last written-down entry. A span longer than
  the ladder reached otherwise got a step too small for it and came back empty.
- Panels that share an axis are given one tick set computed once, from the spectrogram's
  geometry, rather than computing the same thing twice and hoping the two agree. The
  spectrum panel gains frequency grid lines at those values; it previously had none.
- Time labels are `h:mm:ss` when the span reaches an hour and `m.ss` below that, with no
  fractional seconds and no step finer than one second.
- The gutters holding stacked labels are measured rather than assumed:
  `Gutters::measure` formats the widest label each axis could print over its range and
  measures it. `FREQ_LABEL_W`, `DB_LABEL_W` and `CBAR_LABEL_W` are removed.
- The spectrum panel's decibel gutter is bounded as three digits and a sign, because its
  scale follows its own trace and answers to no setting: `f32`'s smallest normal is about
  `1e-38`, which is -760 dBFS. It is also reserved only when that panel is up.
- The colour bar's gutter is measured from the window it can be asked to show, which
  `--dynamic-range` and `--ref` decide between them. Full scale pins the top of that
  window at 0 dBFS; this file's own peak does not, and is not known until the transform
  has run, so `--ref peak` reserves from the `f32` floor instead.

## Rejected alternatives

- Keeping the fixed targets and scaling them by axis length. It picks the tick count
  before knowing how wide the labels are, which is exactly what fails on
  `12.579887 MHz`.
- Shrinking or rotating labels that do not fit. A plot whose labels change size between
  renders is harder to read, and rotated text needs a glyph rasteriser this binary does
  not have.
- Clamping an oversized gutter so the plot always renders. It trades a visible failure
  for clipped labels, which is the defect being fixed. An image too small for its labels
  already fails with a message naming `--image-size`.
- ➕ Choosing the time format from the largest absolute time on the axis rather than from
  the span, so that a minute count never passes sixty. Implemented, then dropped on
  review: it makes the format depend on where a window sits, so panning a one-minute
  selection across the hour mark rewrites every label without any change of zoom, and it
  contradicts the Issue's acceptance criterion outright. A minutes field reading `60.00`
  is the smaller oddity.
- Putting the tick policy in `argand-core` so a future GPUI front end inherits it. It
  measures glyphs with the CLI's font; the part worth sharing is the ladder, and that is
  a few lines. Moving it now would put a font dependency in a crate that must not have
  one.

## Implementation steps

- [x] Add `TextRenderer::digit_height`, measuring the ink a numeric label puts on the
      canvas from the embedded font.
- [x] Add `crates/cli/src/ticks.rs`: `Axis`, `AxisKind`, `LabelRun`, `Tick`, the decimal
      and clock ladders, the densest-acceptable-step search, and `widest_label`.
- [x] Format time labels as `h:mm:ss` and `m.ss`, with whole seconds only and a
      one-second minimum step.
- [x] Replace `FREQ_LABEL_W`, `DB_LABEL_W` and `CBAR_LABEL_W` with `Gutters`, measured
      from the time and frequency extents, and thread it through `Layout::compute` and
      `main.rs::process`.
- [x] Compute the shared time and frequency tick sets once per render and draw every
      axis, grid line and tick mark from them, including the spectrum panel's frequency
      grid and the colour bar.
- [x] Cover the policy with tests: short and long axes, narrow and wide labels, real and
      complex frequency ranges, subsecond, minute-scale and hour-scale spans, decibel
      ranges, boundary clipping, exact label formatting, the one-second floor,
      cross-panel alignment, non-overlap over a matrix of sizes and panels, and density
      growing with the axis.
- [x] ➕ Widen that matrix to all eight panel sets rather than three, down to the
      smallest image that still leaves a plot, and cover a decibel window far wider than
      the default. Which panels are up decides what neighbours an axis has, and so how
      much room its outermost labels can borrow, which the three obvious sets left
      untested.
- [x] ➕ Pin the defects review found: no tick outside the range it was given, a tick on
      an exact end kept, a span no ladder was written for still terminating, and a
      decibel window `--dynamic-range` opened wider than the f32 floor.
- [x] ➕ Pin what the second round found: a range ending one ulp short of a multiple
      staying inside the cap, a quotient too large to index refused rather than
      saturated, the colour-bar gutter following `--ref`, the spectrum gutter ignoring
      what the colour bar was asked for and going unreserved without its panel, and the
      layout matrix proving it is not skipping the combinations it enumerates.
- [x] Update `README.md` and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Render the captures in `tests/signals/` in both orientations and at several image
      sizes, and check by eye that labels are dense, unclipped and aligned across
      panels. Covered baseband, an HF centre frequency, a 2.4 GHz centre frequency,
      hour-scale and sub-second spans, `-d 10000`, and images from 200x200 (which is
      refused with a message naming `--image-size`) to 2048x512.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
