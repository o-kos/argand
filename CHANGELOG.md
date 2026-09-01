# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Until `1.0.0`, a minor version may change behaviour that a `1.x` release would
treat as breaking. Command line options and report fields are the surface this
promise applies to.

## [Unreleased]

### Changed

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

[Unreleased]: https://github.com/o-kos/argand/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/o-kos/argand/releases/tag/v0.0.1
