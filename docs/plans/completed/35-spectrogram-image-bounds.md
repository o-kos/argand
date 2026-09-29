# Issue #35: Check the coordinates `SpectrogramImage` is addressed by

Resolves #35.

## Overview

`SpectrogramImage::get` and `SpectrogramImage::put` in `crates/core/src/view.rs`
work the buffer offset out as `(y * self.width + x) * 4` and check neither
coordinate. `SpectrogramImage` is the last view model in `argand-core` that
addresses its own buffer unchecked: `WaveformEnvelope::column` answers `Option`,
and `DbGrid::value` / `DbGrid::column` answer `Option` with their arithmetic
performed once behind `DbGrid::shape`.

A column index at or past `width` is a valid offset into a later row, so
`get(width, 0)` answers with the first pixel of row 1 and the value looks exactly
like data; a large `y` panics on the slice; and both products can overflow before
any bounds test could see them, because `width` and `height` are public fields a
caller sets.

This change gives `SpectrogramImage` the contract its neighbours have: a
`shape()` that settles whether the buffer covers the declared shape, a checked
`row()` / `row_mut()` pair, and `get` / `put` answering `Option` on top of them.
Every existing caller's behaviour is unchanged: `shade` builds the buffer from a
shape it has already settled and `shade_columns` indexes inside it, and the CLI's
`blit` already range-checks the source coordinate before calling `get`.

Boundaries: `argand-core` only, plus the one CLI call site whose `get` signature
changes. No other view model is refactored, and no render output changes.

## Context

Affected file: `crates/core/src/view.rs`, the `SpectrogramImage` impl.

The contract to match is the one in the same file:

- `DbGrid::shape()` (line 41) returns `None` unless
  `width.checked_mul(height) == values.len()`, because the fields are the
  caller's to set and the product is what sizes the buffer.
- `DbGrid::column(x)` (line 66) returns `Option<&[f32]>` and does its arithmetic
  once with `checked_mul`, so a height near `usize::MAX` fails the accessor
  rather than overflowing past the bounds test.
- `WaveformEnvelope::column(column, channel)` (line 164) returns
  `Option<(f32, f32)>`, checking the channel against the declared count before
  touching the vectors.

Callers of the two accessors, all of which index inside the image they just
sized:

| site | call | range today |
| --- | --- | --- |
| `crates/dsp/src/stft.rs:1056` `shade_columns` | `image.put(x, y, colour)` | `x` from a `0..width` / dirty-column iterator, `y` from `0..height`, with `image` built by `SpectrogramImage::new(grid.width, grid.height)` after `DbGrid::shape()` settled the grid |
| `crates/cli/src/render.rs:1098` `blit` | `img.get(sx as usize, sy as usize)` | `sx` / `sy` are `i64` already compared against `img.width` / `img.height` two lines above |

Other code addresses `image.rgba` directly, by row: `crates/app/src/spectrogram.rs`
(`padded_bgra`, `column_texture`, `bgra`) and `crates/dsp/src/overview.rs`
(`par_chunks_mut`). Those are row-at-a-time or slice-at-a-time operations over a
buffer whose length is checked against the shape in the same expression, so they
are outside this Issue's two accessors and are not touched.

Nothing in the repository can reach either failing case today, which is why this
is a latent trap in a public view model rather than a defect in current
behaviour. `argand-core` is the crate the future GUI milestones index from new
code, where the coordinates stop coming from a loop over the image's own size.

Class A (a public `argand-core` API). Implementer: this branch's agent, chosen by
the owner. Reviewer: GPT-6 Sol, medium reasoning effort, agreed with the owner.

## Decisions

- **`put` returns `Option<()>`, and a rejected write leaves the buffer
  untouched.** `Option` matches `DbGrid::value`, `DbGrid::column` and
  `WaveformEnvelope::column`, which is the consistency the Issue asks for; a
  silent no-op would let a caller loop over an image it sized wrongly and draw
  nothing at all without noticing, which is the same class of silent wrong answer
  the Issue is about. `put` is not `#[must_use]`, so `shade_columns` keeps its
  current statement unchanged and no caller has to handle a value today; the
  contract is available to the GUI milestones that will need it. A `bool` return
  was rejected: it reads as a question about whether the pixel was written rather
  than about whether the coordinate belonged to the image, and it invents a
  vocabulary the neighbouring view models do not have.
- **`get` returns `Option<[u8; 4]>`** for the same reason, and because a
  read cannot invent a value: the only alternatives are a transparent pixel that
  looks like a real one, or a panic. `blit` in the CLI already skips coordinates
  outside the image, so it becomes a `let ... else { continue }` with identical
  behaviour.
- **The checked arithmetic is factored into a private `pixel(x, y)` that
  returns the pixel's byte range,** mirroring `DbGrid::column`: one place works
  the offset out, `checked_mul` keeps a shape near `usize::MAX` from
  overflowing past the bounds test, and `get` and `put` then address
  `rgba.get(range)` / `rgba.get_mut(range)`, which is the same "let the slice
  bound the answer" rule `DbGrid::column` uses. A separate `row` / `row_mut`
  pair was rejected: it would hand out `&mut [u8]` rows of a caller-sized
  buffer, which is the unchecked addressing this Issue removes, and it costs a
  second bounds pass per pixel. `pixel` is private; the public surface is
  `shape`, `get` and `put`, which is what a caller needs.
- **`shape()` is added, mirroring `DbGrid::shape`.** It answers whether
  `width.checked_mul(height)?.checked_mul(4)` equals `rgba.len()`, which is what
  makes the accessors' `None` for an inconsistent image meaningful rather than an
  unexplained refusal. It also names the overflow case the Issue asks a test to
  cover.
- **`SpectrogramImage::new` keeps its unchecked `width * height * 4` allocation.**
  Every construction site passes a shape it has just settled, and the allocation
  is the cheapest place to fail loudly on an impossible size; changing it to a
  `Result` would touch the DSP caches and the application for a case no caller
  can produce. `pixel` bounds the offset against `rgba.len()` as
  well as against the declared shape.
- **`get`'s existing call in `crates/cli/src/render.rs` keeps its sign check and
  drops its size check.** `sx` and `sy` are `i64` the orientation mapping
  produced, and only the negative case is a coordinate error the accessor
  cannot see; the upper bound is what `get` now answers. The `None` arm is the
  same `continue` the removed condition took, so the painted picture is
  unchanged.

## Rejected alternatives

- **Make the fields private and add a `from_shape` constructor** so the invariant
  holds by construction. Rejected: `crates/dsp/src/progressive.rs`,
  `crates/dsp/src/overview.rs` and `crates/app/src/spectrogram.rs` build and
  resize images through `new`, and `crates/app/src/backdrop.rs` and
  `crates/app/src/analysis.rs` move whole `SpectrogramImage` values around; the
  fields are read directly in five modules for `width`, `height` and `rgba`. The
  Issue offers this as an alternative to checking, but the check is the smaller
  change and keeps the type usable the way the rest of the workspace uses it.
- **Panic or debug-assert on an out-of-range coordinate.** Rejected outright:
  `AGENTS.md` forbids `unwrap` / `expect` on external data in non-test code, and
  the two fields are a caller's to set, so a panic would be reachable from a
  library boundary.
- **Have `put` return `bool`.** Rejected: see Decisions.
- **Rewrite the direct `rgba` accesses in `crates/app/src/spectrogram.rs` and
  `crates/dsp/src/overview.rs` to go through the new accessors.** Rejected as
  outside this Issue: those are row-wide slice operations, not pixel addressing,
  and the change would not be covered by the byte-identical `aspec` check, which
  does not exercise them.

## Implementation steps

- [x] Add `shape`, the private `pixel` range helper and the checked `get` / `put`
      to `SpectrogramImage` in `crates/core/src/view.rs`, with the contract
      documented in the same voice as `DbGrid`.
- [x] Update `shade_columns` in `crates/dsp/src/stft.rs` for the new `put`
      contract, keeping its behaviour identical.
- [x] Update `blit` in `crates/cli/src/render.rs` for the new `get` contract,
      keeping its behaviour identical.
- [x] Cover in `crates/core/src/view_tests.rs`: a column index past `width`, a row
      index past `height`, a shape whose product overflows, an inconsistent
      buffer, and `put` refusing to write past either edge.
- [x] Update `crates/core/src/view_tests.rs` and any other test that calls `get`
      or `put` for the new signatures.
- [x] Record the contract in `AGENTS.md`'s `argand-core` description. No
      `CHANGELOG.md` entry: the change is a latent trap in an unpublished
      workspace crate, and the byte-identical render proves no user outside the
      repository can observe it, which is the case `CONTRIBUTING.md` exempts
      from the changelog.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Byte-identical `aspec` output against the merge base over every capture in
      `tests/signals/`. The base `aspec` was built at `origin/main` (`5ba088a`,
      the merge base of this branch) into a separate `CARGO_TARGET_DIR`, and the
      branch `aspec` from this revision's `target/release`; both were run over
      every non-PNG file in `tests/signals/` (19 captures) with
      `-f 1024 -d 60 -i 640x360 --json --quiet -o <dir>/<name>.png`, with
      `--start 0 --duration 10` added for the captures above 100 MiB so the ten-
      and one-gigabyte files finish in reasonable time. All 19 PNGs compared
      byte-identical with `cmp`. The JSON reports differ only in
      `output.path`, which is the output directory that differs by construction,
      and `elapsed_seconds`, which is a wall-clock measurement; with those two
      fields dropped all 19 reports are equal.

## Post-completion

- The Pull Request goes through the agreed external review round with GPT-6 Sol
  at medium reasoning effort before it leaves Draft for the owner.
- `ci/quick` reports on the Draft revision; `ci/full` is requested when the Pull
  Request is marked Ready.
