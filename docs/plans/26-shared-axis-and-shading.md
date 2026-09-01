# Issue #26: Extract axis tick layout and spectrogram shading into shared crates

Resolves #26.

## Overview

Two pieces of presentation logic the GUI will need are locked where only `aspec`
can reach them. Axis tick layout lives in `crates/cli/src/ticks.rs` and measures
its candidate labels with the CLI's own `TextRenderer`; spectrogram shading is a
private `render()` at the end of `analyze`, so decibel values reach a caller only
as finished RGBA.

Move both to where either front end can reach them, without changing a single
output pixel:

- tick layout moves to `argand-core::axis`, with the concrete font dependency
  replaced by a narrow `LabelMeasure` trait that `aspec` implements over its
  existing ab_glyph renderer;
- the decibel grid becomes `argand_core::view::DbGrid`, carried on `Analysis`;
- shading becomes a public `argand_dsp::shade` taking a `DbGrid`, which `analyze`
  calls as its last step.

Out of scope: any GUI code, any change to what `aspec` prints or draws, and any
change to the tick policy itself.

## Context

- `crates/cli/src/ticks.rs` (439 lines) and `crates/cli/src/ticks_tests.rs`
  (526 lines) hold the whole tick policy. The only font-dependent part is
  `LabelMetrics`, which asks `TextRenderer` for a label width and a digit ink
  height. Label formatting already lives in `argand-core::fmt` and does not move.
- `crates/cli/src/render.rs` is the only consumer: `Gutters::measure` calls
  `ticks::widest_labels` and `ticks::caption`, and `Scene` builds `LabelMetrics`
  and calls `ticks::ticks`.
- `crates/dsp/src/stft.rs` holds private `DbGrid<'a>`, `Shading` and `render()`.
  `render()` also stamps the image's time and frequency extents, which it reads
  from `SignalMeta` and `SampleRange`.
- `argand-core` currently depends on `hsl`, `serde` and `thiserror`. It must gain
  nothing else.
- The CLI's real-font coverage stays where the font is: `render_tests.rs` already
  checks, on a rendered plot, that labels clear each other along and across an
  axis.

## Decisions

- `LabelMetrics` holds `&dyn LabelMeasure` rather than a generic parameter, so
  `ticks()` keeps one signature and callers keep the shape they have. A virtual
  call per label measurement is nothing against the transform beside it.
- `DbGrid` owns its values and carries the time and frequency extents alongside
  them. Shading then needs no `SignalMeta`, no `SampleRange` and no sample
  source, which is what the acceptance criteria ask for, and a GUI recolouring a
  grid it already holds has the axis extents in the same place as the numbers.
- `Analysis` keeps `spectrogram` as well as gaining `db`, so `analyze` stays a
  one-call path for `aspec` and every existing consumer is untouched.
- The moved tick tests measure with a `LabelMeasure` built from DejaVu Sans
  advance widths rather than from the font itself, because `argand-core` may not
  open a font. `crates/cli/src/text_tests.rs` pins the same figures against the
  real renderer, so a change to the font asset fails there and names the table to
  re-derive.
- The one moved test that cannot follow the code is the assertion that the CLI's
  reserved decibel floor equals `argand_dsp::DB_FLOOR`: `argand-core` may not
  depend on `argand-dsp`. It stays in the CLI, where both constants are visible.

## Rejected alternatives

- A `dev-dependency` on `ab_glyph` in `argand-core`, with the font asset shared
  across crates, so the tick tests could keep measuring the real font. It keeps
  the tests unchanged but puts a font in the crate that must not know about one,
  and reaches the CLI's asset directory by relative path from another crate.
- Leaving the tick tests in `argand-cli` as the code moved to `argand-core`. The
  tests are about the layout policy, not about the CLI, and a crate whose logic
  is tested from another crate's test module cannot be trusted on its own.
- Passing `SignalMeta` and `SampleRange` to the public shading function. It keeps
  `DbGrid` to numbers alone, but hands a colouring routine the whole signal
  description so it can copy four floats out of it.

## Implementation steps

- [ ] Add `argand-core::axis` with `Axis`, `AxisKind`, `LabelRun`, `Tick`, the
      `LabelMeasure` trait and `LabelMetrics` over it, carrying `ticks`,
      `caption` and `widest_labels` unchanged; re-export from the crate root.
- [ ] Move the tick tests to `crates/core/src/axis_tests.rs` over a DejaVu Sans
      `LabelMeasure`, and pin those figures against the real font in
      `crates/cli/src/text_tests.rs`.
- [ ] Delete `crates/cli/src/ticks.rs`, implement `LabelMeasure` for
      `TextRenderer`, and port `render.rs` to the new locations.
- [ ] Add `DbGrid` to `argand-core::view`: column-major `width * height` values
      with the time and frequency extents, and re-export it.
- [ ] Make shading `argand_dsp::shade(&DbGrid, Shading) -> SpectrogramImage`,
      have `analyze` build the grid and call it last, and add the grid to
      `Analysis`.
- [ ] Update `AGENTS.md` where it describes what each crate owns.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Every capture in `tests/signals/`, rendered in both orientations and each
      of the eight panel combinations, is byte-identical to the same render from
      `main`. The baseline comes from a release binary built at `main` before the
      first commit on this branch, and the comparison is over `sha256sum`.
- [ ] `argand-core`'s dependency list is unchanged: no font, image, toolkit,
      `argand-dsp` or `argand-io` entry.

## Post-completion

- The GUI milestones that follow (#27 onwards) consume `argand_core::axis` and
  `argand_dsp::shade` directly; nothing else is owed after merge.
