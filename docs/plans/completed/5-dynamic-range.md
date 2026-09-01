# Issue #5: Simplify dynamic-range selection and show recommendations

Resolves #5.

## Overview

Replace the public `--ref` plus `--dynamic-range` combination with one
dynamic-range option whose absence keeps the absolute full-scale default, whose
numeric form is peak-relative, and whose `auto` form applies a measured
recommendation. Report the requested, effective, and recommended ranges and
show a short yellow recommendation both on the rendered image and in the human
report when a non-auto range is materially wider than the useful signal span.

The change is limited to scale selection and its presentation. It does not
change sample normalization, gain, STFT magnitude calibration, PSD averaging,
or the peak and floor estimators.

## Context

- `Args` currently exposes `-d/--dynamic-range` as a number defaulting to 110
  and exposes `--ref` separately as `fs` or `peak`.
- `AnalysisRequest` carries those two values into `argand-dsp`; the DSP chooses
  the colour window only after collecting the spectrogram grid, while the
  averaged PSD already supplies the peak and median-floor measurements used by
  the existing console recommendation.
- Layout gutters are measured before analysis, so peak-relative modes must
  reserve for the DSP floor and their greatest possible requested range rather
  than for a peak that has not yet been measured.
- The waveform strip follows the same reference policy as the colour scale:
  full scale in default mode and the measured time-domain peak in fixed and
  automatic modes.
- The image header is drawn independently of optional waveform, PSD, and colour
  bar panels, which makes it the stable place for the recommendation.

## Decisions

- Introduce one toolkit-independent dynamic-range request type in
  `argand-dsp`: `Default`, `Fixed(f32)`, or `Auto`. The CLI stores it as an
  optional `-d` value so omission is distinguishable without accepting
  `-d default` as public syntax.
- Resolve the range once in the DSP after both the spectrogram grid and averaged
  PSD are available. Publish the requested mode, effective range, and
  recommendation with `Analysis` so rendering and reports cannot recalculate
  or disagree about them.
- Preserve 110 dB as the default effective range with a top of 0 dBFS. Fixed
  and automatic modes place the top at the measured spectral peak; automatic
  mode uses the recommendation as its effective range.
- Keep the existing recommendation formula: peak-to-median-floor distance,
  50% headroom, round upward to 10 dB, clamp to 20...120 dB.
- Treat a non-auto effective range as materially excessive when it exceeds the
  rounded recommendation by at least one 10 dB step. Automatic mode never
  recommends the value it already applied.
- Add `dynamic_range_mode`, retain `dynamic_range_db` as the effective value,
  and add `recommended_dynamic_range_db` to the machine-readable STFT report.
  This preserves the meaning of the existing numeric field while exposing all
  three requested values separately.
- Draw `Suggested: -d N` at the right side of the image header in a named yellow
  warning colour and pass the same text to the human report. Header placement
  keeps it independent of every optional panel.

## Rejected alternatives

- Keep `DbReference` internally and translate the new CLI back into the old
  pair. That would preserve two sources of truth and make invalid combinations
  representable after the public combination was removed.
- Calculate the recommendation independently in the CLI report and renderer.
  That duplicates policy and can let `-d auto`, JSON, console text, and pixels
  describe different values.
- Relayout the image after analysis to reserve a separate warning row. Layout
  determines transform pixel dimensions, so doing that correctly would require
  a second analysis; the existing header can carry the short annotation.

## Implementation steps

- [x] Add the dynamic-range request/result model and recommendation resolution
  to `argand-dsp`, including default, fixed, automatic, rounding, and clamp
  tests.
- [x] Replace `--ref` and the numeric default in the CLI, wire the resolved
  mode through analysis, gutter measurement, and waveform scaling, and test
  parsing and removal of the old option.
- [x] Expose requested, effective, and recommended ranges in JSON and human
  reports, and render the conditional yellow recommendation independently of
  optional panels.
- [x] Update end-to-end coverage for default, fixed, automatic, warning, and
  panel-independent behaviour.
- [x] Update CLI help, README, and the Unreleased changelog entry.
- [x] ➕ Include the same range suggestion in compact batch reports.
- [x] ➕ Measure the time-domain peak over the complete selected span,
  including samples after the last full STFT frame.
- [x] ➕ Keep long image titles clear of the right-aligned suggestion.
- [x] ➕ Preserve fractional fixed ranges in the human report.
- [x] ➕ Read the post-STFT tail for peak reporting even when no waveform
  panel is requested.
- [x] Complete validation and change-specific render checks.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Verify `--ref` is rejected while omitted `-d`, numeric `-d`, and
  `-d auto` produce the documented JSON modes and effective windows.
- [x] Render warning and no-warning cases without optional panels and verify
  the annotation colour and text placement from the current release binary.

## Post-completion

After owner acceptance and green required checks, squash-merge the Pull Request,
switch the main working directory to `main`, and delete
`feature/5-dynamic-range` remotely and locally after the required safety checks.
