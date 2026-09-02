# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Until `1.0.0`, a minor version may change behaviour that a `1.x` release would
treat as breaking. Command line options and report fields are the surface this
promise applies to.

## [Unreleased]

### Added

- A second binary, `argand`: the graphical application. This release opens its
  window and nothing more -- no signal, no analysis, no panels -- so that the
  toolkit is proven on Linux, Windows and macOS before the views are built on
  it. `aspec` is unchanged.
- `argand.toml`, read from beside the binary or from the platform configuration
  directory, sets the theme, the colour scheme, the dynamic-range mode, the
  transform defaults and the panel proportions. Names are spelled as `aspec`
  spells them. The application only ever reads this file.
- The window remembers its size and state between runs, in `session.toml` in the
  platform state directory. Its position is remembered where the platform
  supplies one, which does not include Wayland. Neither file can prevent the
  application starting: a missing, unreadable, malformed or future-versioned one
  is logged and replaced by the defaults.

### Changed

- The Rust toolchain is now 1.97.1, which the GUI toolkit requires. `aspec`
  renders identically.

## [0.0.2] - 2026-09-01

### Changed

- The report is now one section per file: a header naming the file and its
  facts indented under it, comma-separated and each printed once. A single
  input takes six lines instead of twelve, and the batch line uses the same
  field names, order and units. The render's path now reaches stdout only
  when stdout is not a terminal, where a single file used to echo it even on
  a terminal that had just been told where the render went, and `-v` restores
  the sample count, the divisor, the scaling, the reduce mode, the levels in
  the file's own units and the render's full path.
- The range recommendation reads `try -d N to fit the drawn range` in the
  report, beside the range it argues with. The image footer keeps `sugg -d N`.
- Sample counts put a space before their unit: `43.2 Mspl`, not `43.2Mspl`.
- Dynamic-range selection now uses one option: omitting `-d` keeps the
  absolute `0...-110 dBFS` scale, a numeric value selects that range below the
  measured peak, and `-d auto` calculates and applies a peak-to-floor range.
  The separate `--ref` option was removed. Excessive non-auto ranges now carry
  a recommendation in the report and a yellow `(sugg -d N)` in the image
  footer; JSON reports requested, effective and recommended ranges separately.
- Axis tick density now follows the length of the axis and the measured width
  of its labels rather than a count fixed per axis, so a large render carries
  many more coordinates and a small one no longer overlaps them. Labels that
  would not fit whole inside the canvas are dropped with their grid lines
  instead of being clipped.
- Time axis labels read as a clock: `1:02:09` over a span of an hour or more
  and `3.07` below that. Ticks never step finer than one second and labels no
  longer carry fractional seconds.
- Axis units are named once beside the axis instead of on every tick: `MHz` at
  the head of the frequency labels and `dB` above the colour bar, with bare
  numbers under them. One unit is chosen for the whole frequency axis, so
  neighbouring values can no longer print as `999.999 Hz` and `1.000 kHz`.
- The plot's footer names the vertical scale as `dBFS, ENBW <bandwidth>` in one
  field, replacing `bin <bandwidth> · dBFS/bin`. The scale divides by the
  window's coherent gain rather than by a bandwidth, so its levels are not a
  density per hertz, and the bandwidth quoted is the window's equivalent noise
  bandwidth rather than the raw `Fs / N` bin spacing.
- Panels sharing an axis are given one set of tick values, so the waveform
  strip's time grid and the spectrum panel's frequency grid line up with the
  spectrogram's exactly. The spectrum panel gained frequency grid lines.
- The gutters holding axis labels are measured from the widest label the axis
  can print. Frequency labels on a tuned capture, such as `12.579887 MHz`, no
  longer run off the left of the image.

## [0.0.1] - 2026-08-28

### Added

- `aspec`, a command line tool that renders a signal file to a PNG: a
  spectrogram with an optional waveform strip, averaged spectrum and colour
  bar beside it.
- Container detection by content rather than by extension, covering WAV, FLAC
  and headerless files through `--raw <token>[@<rate>]`.
- Ten sample types across real and complex domains: `u8`, `i16`, `i32`, `f32`
  and CoolEdit `f16x8`, each as `rl_` or interleaved `iq_`.
- Two-sided `fftshift`ed spectra for complex captures and one-sided spectra
  for real ones, with frequency axes in physical hertz derived from
  `--center` and the capture sample rate.
- Streaming STFT that folds frames into image columns as they are computed, so
  memory follows the output size rather than the length of the capture.
- Waveform, PSD and colour-bar panels sharing the spectrogram's axes, selected
  with `--panels`.
- Six colour schemes, four window functions, configurable transform size, hop,
  dynamic range, reference level and column reduction.
- Span selection with `--start` and `--duration`, normalization control with
  `--normalize` and `--gain`, and format overrides with `--sample-type`,
  `--rate` and `--offset`.
- Batch processing of several inputs, each an exact path or a non-recursive
  filename mask, with a compact line per file and a run summary.
- A machine-readable report on stdout under `--json`.
- Continuous integration on Linux and Windows, and a tag-driven release
  workflow producing archives and checksums for both platforms.
- A workspace-wide lint policy: ten maintainability lints with explicit
  thresholds, enforced identically on a developer's machine and in CI.

[Unreleased]: https://github.com/o-kos/argand/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/o-kos/argand/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/o-kos/argand/releases/tag/v0.0.1
