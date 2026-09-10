# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Until `1.0.0`, a minor version may change behaviour that a `1.x` release would
treat as breaking. Command line options and report fields are the surface this
promise applies to.

## [Unreleased]

### Added

- A complete `argand.toml` template with English comments and explicit built-in defaults in the application installation assets.

- View → Time scale format and the time-ruler context menu select hours/minutes/seconds, elapsed seconds or zero-based sample numbers, with the unit shown once at the right. The Alt time badge follows the selection, and the format survives restarts without preserving file zoom or position.
- GUI numbers, hints and analysis settings follow the system numeric locale, including decimal marks, digit grouping and localized numeric input. The configuration-only `number_format` option can override it with an explicit locale such as `ru-RU`.

- Hold Alt over the spectrogram to show guide lines to the time and frequency rulers with rounded coordinate badges, vertically centred text and white/black/white lines visible across palettes.

- Two compact status-bar groups: combined file details with sample count, file size
  and sample extrema, and analysis controls. Adjust FFT, window,
  overlap, aggregation, colours and display range; settings survive restarts.
  Colour/range changes reuse analysis, and a yellow range offers a clickable recommendation.

- Select Peak (MAX) or Mean power from the GUI analysis settings window. The new mean
  averages squared spectral amplitudes across time and frequency before conversion
  to dB, and is also available as `aspec --reduce mean-power`. The existing CLI
  `mean` keeps its dB averaging behavior. The top-level `aggregation` configuration
  sets the GUI's initial choice.

- The GUI shows a sparse signal preview before full analysis, then refines both
  panels from left to right. New requests cancel obsolete work, interim display
  scales remain stable, and opening-level scans have a 64 MiB budget.

- A linear waveform above the GUI spectrogram, preserving short transients and
  merging I and Q into one trace, matching `aspec` without extra labels. The waveform has no grid or extra captions; both panels share the time
  scale, and a subtle draggable separator ends at the waveform edge and remembers their proportions between runs. Ruler borders match the tick marks, with the frequency border joined to the separator.

- Starting without a file shows links to existing recent captures, with Alt+1
  through Alt+9 for the first nine. Availability checks run in the background,
  independently for each path, so offline network locations do not block startup.
  Recent rows highlight their text-sized area on hover without underlines;
  `or` aligns with the heading and the file chooser with the filenames below the list. Content-sized shortcut hints show
  the binding separately at the right edge and wrap long paths within the window.
- Ctrl+O (Cmd+O on macOS) opens the file chooser. The File menu and start-page
  button use the same action and show its registered shortcut.

- A second binary, `argand`: the graphical application. It opens a signal file
  and shows its spectrogram, with time and frequency axes placed by the same
  tick policy `aspec` uses -- two-sided around the centre frequency for an I/Q
  capture, one-sided for a real one. `aspec` is unchanged.
- `argand <file>` takes the same seven options `aspec` takes for reading a
  capture: `--raw`, `--sample-type`, `--rate`, `--center`, `--offset`,
  `--normalize` and `--gain`. Started with no file, it opens an empty window.
- The transform runs on a thread of its own, so the window stays usable while a
  long capture is analysed and the status bar reports progress and the elapsed
  analysis time on completion. A
  large capture is still slow to appear: making the first frame fast comes
  next.
- The status bar names the container, the sample type, the sample rate and the
  duration, and the centre frequency of a capture that was tuned to one. A file
  that cannot be read is reported in the window, with the same message `aspec`
  would print, and leaves the application working.
- `argand.toml`, read from beside the binary or from the platform configuration
  directory, sets the theme, the colour scheme, the dynamic-range mode and the
  transform defaults. Names are spelled as `aspec`
  spells them. The application only ever reads this file.
- The window remembers its size and state between runs, in `session.toml` in the
  platform state directory, wherever the toolkit reports the window state
  reliably. Its position is remembered only where the toolkit reports enough to
  restore it, which excludes Wayland and excludes a second display on macOS.
  Neither file can prevent the application starting: a missing, unreadable,
  malformed or future-versioned one is logged and replaced by the defaults.

### Fixed

- FLAC seeks discard old decoder packets, preventing incomplete progressive analysis and full-capture waveform scans.

- Time zoom keeps the current detail until a complete replacement is ready instead
  of flashing sparse previews. Deep zoom computes short ranges in one pass and
  avoids repeating identical display work; a retained wider picture fills known areas on zoom out.

- Resizing the GUI window or dragging its panel separator reuses a bounded
  spectral overview instead of restarting complete file analysis. Refinement
  continues during resizing, and status timing and colour levels remain stable.
  Cached cell overlap preserves peaks and weights mean power before shading;
  detail within a cache cell is limited by the documented overview resolution.

- Replacing a spectrogram during resize no longer destroys its GPU texture
  while a preceding frame can still be using it, which could freeze the window.
- Resize cursors return to the normal pointer inside the window, and the
  right-hand title-bar area no longer starts a resize when the window is expanded.

### Changed
- The waveform is now a full-capture minimap that darkens the waveform outside the visible interval, without a frame. Zoom, pan and spectral settings leave its content unchanged; outside click/Ctrl+click pan one/five ruler divisions, outside double-click centres, inside clicks leave the range unchanged, and dragging moves the interval. Open-hand cursors mark the interval and rulers. Its independent bounded scan continues when the initial FFT analysis is interrupted.


- Every file opening resets time zoom and position. Ctrl+wheel zoom and wheel horizontal pan work over both the spectrogram and time ruler. Left/Right move by one ruler division, Ctrl+Left/Right by five, preserving ruler spacing and format. The crosshair is limited to the spectrogram. Zoom keys use Ctrl+Plus/Minus and Ctrl+0 (fit).

- Analysis settings preview until OK, with Cancel and Reset to defaults controls.
  Dynamic range now belongs to the current file rather than the saved session.
  Opening the editor dismisses its hint; Ctrl+R (Cmd+R on macOS)
  applies range advice, and a yellow ⚠ marks the warning.

- File details use aligned hint rows; FFT details appear on hover with muted status text. Analysis settings use standard dropdowns and numeric fields with full keyboard access. Yellow range warnings are limited to low-level signals in the absolute display scale.

- Reduce full-pass spectrogram overhead by reusing FFT scratch buffers and updating only changed display columns during refinement.

- The GUI frequency scale sits on the right with vertically centered labels,
  and time labels follow their ticks. The frequency unit sits inside the right
  gutter beside the image, while a separate time row and 4-logical-pixel outer
  margins keep the scales clear of adjacent panels and window edges.

- The window title is centred, normal client-decorated windows have subtle rounded
  corners, and the waveform placeholder stays 3 rem (normally 48 logical pixels) high. The legacy
  `panels.waveform_fraction` setting is still accepted but no longer controls it.
- File menu text matches the popup menu size. The Close hover/press background
  follows the window corner, and status metadata has separate fields with tooltips
  and compact duration units. Sample domain and storage format use a middle dot.
  Hints show a heading, a muted current value at the same size, and an optional
  smaller muted one-sentence explanation without a terminal period. The duration hint shows a
  millisecond clock; the completed-analysis timing now has an explanatory hint.
  Wrapped explanations contribute their full height and stay inside the hint background.
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
