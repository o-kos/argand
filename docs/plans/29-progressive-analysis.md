# Issue #29: Progressive signal analysis

Resolves #29.

## Overview

Show a full-width preview promptly, then refine the spectrogram and waveform
from left to right without blocking the GUI. Keep CLI output unchanged.
The owner requested #29, then #32 and #62; navigation (#30) is deferred.

## Context

File opening already runs on the analysis thread. DSP `analyze` returns only at
the end; queued requests cannot stop an active pass. The existing FFT plan,
column accumulator and waveform envelope provide the numerical foundation.
Automatic normalization can scan hundreds of megabytes before analysis starts.

## Decisions

- Preserve `analyze` and CLI defaults; share its transform and reduction machinery.
- Preview frames lie on the full pass frame lattice and contribute exactly once.
- Refine sequentially, publishing whole paired snapshots at most 20 times/second.
- Hold preview colour and waveform scales during refinement, resolving final scales once.
- Cancel superseded requests and reject obsolete updates; keep at most one active analysis per document.
- Bound GUI automatic level scanning to 64 MiB without changing CLI normalization.
- Measure launch-to-picture separately from analysis-only timing; report measured limitations.

## Rejected alternatives

- Independent waveform passes duplicate sample I/O and can drift from the spectrogram.
- Re-running full analysis per snapshot multiplies work; updates must reuse accumulators.
- Implementing navigation here would expand the requested scope into deferred #30.

## Implementation steps

- [ ] Record baseline timings and preserve a CLI binary for output comparison.
- [ ] Implement bounded opening-level scans with unchanged CLI defaults.
- [ ] Add cancellable preview/refinement and shared paired snapshots to DSP.
- [ ] Integrate latest-request cancellation, progressive rendering and refinement indication.
- [ ] Cover exact final results, preview geometry, cancellation and bounded queues with tests.
- [ ] Measure first-picture latency, snapshot cost and native responsiveness.
- [ ] Update architecture, user documentation and #31 integration status.
- [ ] Complete independent review and validation; move this plan to completed before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] CLI image comparison against baseline fixtures
- [ ] Release timing on roughly 10 MB, 1 GB and 10 GB captures, including f16x8 opening
- [ ] Dark/light native checks of preview, refinement and replacement during active analysis
- [ ] Focused external review with GPT-5.6 Sol at High effort

## Post-completion

After owner acceptance and full CI, merge through the PR and clean its branch.
Proceed to #32, then #62; leave #30 deferred.
