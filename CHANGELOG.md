# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Until `1.0.0`, a minor version may change behaviour that a `1.x` release would
treat as breaking. Command line options and report fields are the surface this
promise applies to.

## [Unreleased]

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

[Unreleased]: https://github.com/o-kos/argand/commits/main
