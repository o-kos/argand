# Issue #35: Check the coordinates `SpectrogramImage` is addressed by

Resolves #35.

## Overview

`SpectrogramImage::get` and `SpectrogramImage::put` in `crates/core/src/view.rs`
work the buffer offset out as `(y * self.width + x) * 4` and check neither
coordinate. `SpectrogramImage` was the last view model in `argand-core` that
addresses its own buffer unchecked. Two contracts meet here rather than one.
`SpectrogramImage::get` and `put` settle the whole buffer through `shape()` and
answer nothing until it matches, so they are strict about the image as a whole.
`DbGrid::value` and `DbGrid::column` do not consult `shape()`; they check the one
coordinate, settle that offset with `checked_mul` and take the cell from the
slice, so a cell inside the declared shape stays readable where the buffer runs
longer than that shape. `WaveformEnvelope::column` follows the second form.

A column index at or past `width` is a valid offset into a later row, so
`get(width, 0)` answers with the first pixel of row 1 and the value looks exactly
like data; a large `y` panics on the slice; and both products can overflow before
any bounds test could see them, because `width` and `height` are public fields a
caller sets.

This change gives `SpectrogramImage` the contract its neighbours have: a
`shape()` that settles whether the buffer covers the declared shape, a private
`pixel()` that works one pixel's byte range out with checked arithmetic, and
`get` / `put` answering `Option` on top of them.
Every existing caller's behaviour is unchanged: `shade` builds the buffer from a
shape it has already settled and `shade_columns` indexes inside it, and the CLI's
`blit` already range-checks the source coordinate before calling `get`.

Boundaries: `argand-core` first, then the callers whose signature changes. The
review rounds widened it past the Issue's own type: `WaveformEnvelope::column`
indexed its buffers the unchecked way this Issue describes, and `shade_columns`
sliced a grid by hand, so both were brought onto the same checked contract. No
render output changes.

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

Other code addresses `image.rgba` directly rather than through the two accessors:
`crates/app/src/spectrogram.rs` (`padded_bgra`, `column_texture`, `bgra`) and
`crates/dsp/src/overview.rs` (`par_chunks_mut`). They are outside this Issue, which
is about `get` and `put`. `padded_bgra` and `bgra` settle the buffer length against
the shape with `checked_mul` before working any offset out, and `par_chunks_mut`
walks the whole buffer, so none of them depends on an unchecked product.

`column_texture` was the one that computed `(row * image.width + column) * 4` with a
plain multiplication, and that path was a reachable defect after all. The strip
built beside the read is sized by `image.height`, so a width past any buffer passes
the entry test; with `width = usize::MAX`, `height = 2`, a two-pixel buffer and
`column = 1`, the second row computed `usize::MAX + 1`, which panicked on a checked
build and wrapped onto a foreign pixel on a release one. The fields are public, so
a caller can build that value. No production path does build one, and that is the
only reason the defect never fired. It is consolidated onto `get`, which checks the
product and refuses, and a test covers the overflowing width.

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
      workspace crate, and the byte-identical render proves the checked outputs
      agree with the base build, not that no observable difference is possible
      on an input the render does not cover.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Byte-identical `aspec` output against the merge base over every capture in
      `tests/signals/`. The base `aspec` was built at `origin/main` (`5ba088a`,
      the merge base of this branch) into its own `CARGO_TARGET_DIR`, and the
      branch `aspec` into a second one; both were run over every non-PNG file in
      `tests/signals/` (19 captures) with
      `-f 1024 -d 60 -i 640x360 --json --quiet -o <dir>/<name>.png`, with
      `--start 0 --duration 10` added for the captures above 100 MiB so the ten-
      and one-gigabyte files finish in reasonable time. All 19 PNGs compared
      byte-identical. The JSON reports differ only in `output.path`, which is the
      output directory that differs by construction, and `elapsed_seconds`, which
      is a wall-clock measurement; with those two fields dropped all 19 reports
      are equal.
- [x] The comparison was repeated after the first two rounds, because both
      changed the shading path and the first result no longer described the
      code. Those two commits moved the image-covers-grid check to the entry of
      `shade_columns` and replaced the hand-written grid slice with
      `DbGrid::column`, so the branch `aspec` was rebuilt from that revision and
      every capture rendered again against the same base build. The result is the
      same: 19 PNGs byte-identical, 19 reports equal once the two fields above
      are dropped.
- [x] The later rounds changed only `crates/app/src/spectrogram.rs`, its tests and
      this plan. `aspec` does not link the application crate, so the comparison
      above still describes what it would print; it was not re-run for those
      rounds, and this line is that inference rather than a second measurement.
      The 10-second window on captures above 100 MiB is the other limit on what
      the result covers.

## Review rounds

Reviewer agreed with the owner: Codex `gpt-6-sol` at medium reasoning effort. The
rounds run read-only through the `codex` CLI and none of them found a major issue
in the accessors.

**Round 4, owner decision.** The process sets three rounds as the limit and says
that findings still standing after the third mean the task was stated badly. The
owner ran a fourth anyway, because round 3's disposition of `column_texture` rested
on a claim the plan had not checked, and fixing that turned out to be a change of
kind rather than a missing line. The extra round is recorded here so the exception
is visible rather than assumed.

**Round 1.** Two minor findings, both accepted and fixed.

- The only production caller of `put` was `let _ = image.put(...)` in
  `shade_columns`, so the reason the contract exists, that a wrongly sized image
  cannot drop a write unnoticed, did not hold where it mattered. The
  image-covers-grid question is now settled once at the entry of the private
  function, with a test for an image one column short, one row short, larger than
  the grid and exact. The test was mutation-checked.
- The new workspace rule in `AGENTS.md` claimed every view model checks every
  coordinate with `checked_mul`, and the plan described a `row()`/`row_mut()`
  pair the code does not have. The rule was narrowed to the view models that keep
  those checks, and the plan's summary corrected to the private `pixel()``.

**Round 2.** Two minor findings, both accepted and fixed.

- `WaveformEnvelope::shape` answers for the whole buffer while `column` answers
  for the cell it was asked for, so a cell inside the declared shape stays
  readable where the buffers run longer. The doc comment said anything indexing
  the envelope has to ask `shape`, which is not true, and now follows `DbGrid`'s
  wording about sizing a buffer. A test records the difference. `AGENTS.md` was
  narrowed again, to the accessors by name, because `pixel_spans` still multiplies
  a display step by `columns` and is not one of them.
- The recorded byte-identical result described the code before the two rounds,
  and the later commit changes the shading path. The comparison was repeated on
  that revision and is recorded above.

Round 2 also confirmed what the round 1 fixes relied on: `SpectrogramImage` bounds
both dimensions and the slice length, a large or short `DbGrid`, empty shapes and
an image larger than the grid all fail safely in `shade_columns`, `pixel_spans` and
`minimap::rebin` derive their indices from `columns`, and the added per-column
check is constant work.

**Round 3.** Two minor findings, both accepted and fixed.

- The envelope's overflow test could not tell checked arithmetic from a wrapping
  one, because it cleared the buffers first, so a wrapped offset still answered
  with nothing while the comment claimed the multiplication was what refused. It
  now keeps one cell and asks for a column inside the declared width whose
  product wraps onto exactly that cell, where a wrapping index answers with that
  cell's values instead of refusing.
- The plan's boundaries paragraph still claimed `argand-core` alone while the
  rounds had widened the change, and its note on direct buffer access claimed
  every such site checks the length in the same expression, which is false of
  `column_texture`. That site was named as separate work, and the commit hashes
  the plan had recorded were removed, which `CONTRIBUTING.md` forbids.

Round 3 also held the branch back: it confirmed the accessors, the criteria and
the recorded render, and asked for the overflow test and the plan's wording
rather than for a change to the code.

**Round 4.** One substantial finding and two moderate ones, all accepted and fixed.

- The substantial one refuted the argument the round 3 disposition rested on.
  That disposition called `column_texture`'s hand-written product unreachable
  because the strip built beside the read would refuse an impossible shape first,
  but the strip is sized by the height, not the width, so a width past any buffer
  passes the entry test and the product overflowed on the second row. The claim
  is corrected, the site is consolidated onto `get`, and a test now covers the
  overflowing width, which fails on the old arithmetic.
- `AGENTS.md` and the plan's overview claimed every named accessor settles its
  product behind `shape()`. `DbGrid::column`, `DbGrid::value` and
  `WaveformEnvelope::column` do not consult it; they check one coordinate and
  take the cell from the slice. Both documents now describe the two contracts
  apart, since the difference is deliberate.
- The plan still spoke of two rounds, its record lacked round 3, and its render
  evidence and changelog reasoning overstated what the comparison proves. The
  round count is corrected, round 3 is recorded, the render line names the
  revision it covers and the inference that the later application-only change
  preserves it, and the changelog reasoning now says the comparison proves the
  checked outputs agree rather than that no difference is possible.

Round 4 also listed the buffer-indexing sites this branch did not touch, with the
guard each one relies on, and asked that they become their own tasks rather than
ride along here.

## Post-completion

- The Pull Request goes through the agreed external review round with GPT-6 Sol
  at medium reasoning effort before it leaves Draft for the owner.
- `ci/quick` reports on the Draft revision; `ci/full` is requested when the Pull
  Request is marked Ready.
