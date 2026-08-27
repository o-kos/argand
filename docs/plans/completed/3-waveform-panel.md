# Issue #3: Render waveform alongside the spectrogram by default

Resolves #3.

## Overview

`aspec` renders a spectrogram, an averaged PSD and a dB colour bar, and offers no
time-domain view at all. The PSD and the colour bar occupy space even when only the
signal's shape and its spectrogram are wanted.

This change adds a waveform strip drawn above the spectrogram (horizontal) or to its
right (vertical), sharing the spectrogram's time axis, and replaces `--mode` with
`--panels`, which selects the panels drawn *beside* the spectrogram. The PSD and the
colour bar become opt-in.

## Context

- `crates/cli/src/render.rs` owns `Mode`, `Orientation`, `Layout` and every drawing
  routine. `Layout::compute` reserves the panel rectangles before the transform runs so
  the spectrogram is computed at exactly the pixel size it will occupy.
- `crates/dsp/src/stft.rs::analyze` streams the selected range in blocks, folding frames
  into image columns as they are computed. It already tracks `time_peak`, and it already
  carries `#[allow(clippy::too_many_arguments)]` over ten positional parameters.
- `crates/core/src/view.rs` owns the toolkit-neutral render models. `AGENTS.md` assigns
  `WaveformEnvelope` to `argand-core` and min/max construction to `argand-dsp`.
- The stderr report and the `--json` report are built from the STFT results
  (`peak_bin`, `floor`, `contrast_hint`, `stft.frames`, `stft.enbw_hz`).

## Decisions

- The spectrogram is always drawn. It is the point of the tool, so it is not a panel the
  user selects; `--panels` names only what is drawn beside it. `spectrogram` is therefore
  not a valid token, and `--panels none` renders the spectrogram alone.
- `none` matches the spelling already used by `--normalize none`, so no new idiom is
  introduced.
- Because the spectrogram is unconditional, the STFT always runs and the report stays
  complete under every `--panels` value, and `Layout::transform_size` no longer needs its
  `(64, 64)` fallback.
- The waveform is a fixed 64-pixel strip -- a mini-map, not a full panel. It has a frame
  and a centre line but no labelled amplitude axis of its own.
- The strip's vertical scale is linear, with the edge standing for the `--ref` level:
  full scale under `--ref fs`, the loudest sample under `--ref peak`. The reference is
  read in the time domain, because a sample is what the strip draws.
- ➕ That scale was logarithmic over the `-d` window first, so the strip and the colour
  bar would size together. It was changed after seeing it on a real capture: a min/max
  span in decibels pins anything above the noise to the edges -- a capture at -6 dBFS
  fills 90% of the half-height -- and the strip became a solid band with no shape in it.
  A linear strip of the same capture shows the level rising past the 25th minute,
  individual bursts, and the I and Q traces apart from one another.
- `WaveformEnvelope` stores linear min/max. Mapping to dB is a presentation choice and
  belongs to `argand-cli`; a GUI may want the linear values.
- The envelope is built during the STFT pass rather than in a pass of its own, so the
  default render still reads the file once. The tail shorter than one hop, which the
  frame loop never reads, is read separately after the loop so the last columns are real
  rather than borrowed from a neighbour.
- `analyze` takes an `AnalysisRequest` struct instead of ten positional parameters.
- With a spectrogram and a PSD both present, the PSD keeps sharing the spectrogram's
  frequency axis (its `y`/`h` when horizontal, its `x`/`w` when vertical) rather than
  stretching across the waveform strip as well. Bin-for-bin alignment matters more than
  the small empty corner beside a 64-pixel strip.
- A complex signal is drawn as one track: each column spans whichever of I and Q reached
  further.
- ➕ Two colour-coded traces, alpha-blended where they overlap, were built first. Dropped
  after review: on a real capture the two envelopes very nearly coincide, so the second
  colour was hidden under the first everywhere except at the extremes, and it cost a
  legend and a blend for a distinction nobody could read. `AGENTS.md` already asks for
  one combined track for complex signals.

### Divergence from the Issue text

The Issue's acceptance criteria describe `--panels waveform,spectrogram,psd,db`, a
`waveform,spectrogram` default, standalone waveform and PSD renders, and a rejection of
`db` without `spectrogram`. The decision that the spectrogram is always drawn supersedes
all four: `spectrogram` is not a token, the default is `waveform`, the only standalone
render is the spectrogram itself (`--panels none`), and `db` can never appear without a
spectrogram to explain. The remaining criteria are unchanged.

## Rejected alternatives

- Computing the envelope in a second pass over the file. Simpler, but it doubles the IO
  of the default render for no gain.
- Skipping the STFT when no spectral panel is requested. Moot once the spectrogram is
  unconditional, and it would have cost the report its `peak_bin`, `floor` and `hint`.
- A logarithmic strip over the `-d` window, so `-d` would scale both views of the signal
  at once. Implemented, rendered and rejected on the evidence; see the decision above.
- A `--waveform-scale linear|db` option offering both. Rejected as an option nobody would
  reach for once the decibel strip was seen: it has no reading the linear one lacks.
- Building the multi-level min/max pyramid now. That belongs to Phase 2 of the roadmap;
  the CLI needs exactly one level, at the output width.

## Implementation steps

- [x] Add `WaveformEnvelope` to `argand-core` with column accessors and neighbour fill
      for columns no sample landed in.
- [x] Add an incremental envelope builder to `argand-dsp` and fold it into the `analyze`
      pass, including the sub-hop tail the frame loop leaves unread.
- [x] Replace `analyze`'s positional parameters with `AnalysisRequest` and return the
      envelope on `Analysis`.
- [x] Replace `Mode` with `Panels` (`waveform`, `psd`, `db`, `none`) and wire
      `--panels` into the CLI with clear errors for unknown, empty and mixed `none`
      values.
- [x] Lay out the waveform strip above the spectrogram when horizontal and to its right
      when vertical, sharing the time axis, with the PSD and colour bar opt-in.
- [x] Draw the strip: min/max spans scaled to the reference level, joined between
      columns, merging I and Q into one track for complex signals.
- [x] Record the panel set in the JSON report.
- [x] Update `README.md`, the CLI help and examples, and the `aspec` description in
      `docs/plans/IMPLEMENTATION_PLAN.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use ➕ for tasks discovered after implementation begins and ⚠️ for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check` passed.
- [x] `cargo clippy --all-targets --locked -- -D warnings` passed.
- [x] `cargo test --locked` passed: 186 tests.
- [x] `cargo build --release --locked` rebuilt after the checks above, so the demonstrated
      binary matches the branch.
- [x] Envelope tests prove a single-sample peak survives decimation to a few hundred
      columns, that block boundaries do not change the result, and that the tail past
      the last whole frame still reaches the strip.
- [x] Layout tests prove the waveform and the spectrogram share the time-axis extent in
      both orientations, that the PSD stays on the spectrogram's frequency axis, and
      that the PSD and colour bar are absent by default.
- [x] Render tests prove both I and Q remain visible when their amplitudes differ, that
      a real signal draws one trace and no legend, and that a quiet capture still fills
      the strip under `--ref peak`.
- [x] End-to-end tests cover the `--panels` matrix in both orientations, the default
      panel set in the JSON report, and every rejected panel value.
- [x] Rendered `12.579000_25_08_26_06_41_10.iqw` (30 min, complex) in both orientations
      and `rl_f32-hfdl.wav` (real) horizontally. The strip's bursts line up with the
      spectrogram's, and the complex traces stay apart.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
- Decide, once the logarithmic strip has been seen on real captures, whether a
  `--waveform-scale linear|db` option is worth its own Issue.
