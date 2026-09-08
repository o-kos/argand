# Issue #69: Select spectrogram aggregation

Resolves https://github.com/o-kos/argand/issues/69.

## Overview

Allow a live comparison of Peak (MAX) and Mean power on short and repeated captures. Average squared spectral amplitudes before converting to dB; preserve the existing CLI mean in dB.

## Context

The current GUI hardcodes MAX on both axes. The progressive analyzer compacts consecutive MAX rows within a worker while keeping the PSD frame count independent. Requests already cancel superseded analysis and retain the displayed picture.

## Decisions

- Stack this focused branch on `perf/67-fft-scheduling`, retaining the version currently under owner evaluation without merging either pending PR.
- Keep Peak as the default; add a Spectrogram menu with mutually exclusive aggregation choices and a configuration default.
- Keep the selected mode for subsequent files in the running application. Persisting all live analysis settings remains part of #32.
- Add `mean-power` to the shared DSP/CLI reducer, averaging bin power within each output row and frame power within each output column. This is a mean squared amplitude display, not integrated power over the output frequency band.
- Preserve legacy `mean` and the waveform envelope. Do not change sampling density, FFT parameters, normalization or resize behavior.

## Rejected alternatives

- Reinterpreting CLI `mean`: it would silently change existing output.
- Combining this with resize caching or the entire #32 UI: these are independent changes needing their own validation.

## Implementation steps

- [x] Add linear-power reduction to plain and progressive DSP paths, including analytic correctness tests.
- [x] Add a live GUI menu and configurable initial aggregation, using existing cancellation and image retirement.
- [x] Update CLI help, user documentation, changelog and architectural context.
- [ ] Compare source and 1 GB repeated m39 with identical FFT/display settings; preserve recordings.
- [ ] Complete local validation and independent read-only review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after checks, using the isolated release target used for performance work
- [ ] Native menu switching, cancellation, and visual comparison with the current release build
- [ ] External review using Codex GPT-5.6 Sol with High reasoning effort

## Post-completion

Keep the PR Draft for owner feedback. Integration waits for the parent work to be accepted.
