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
- A first coarse preview uses at most 128 evenly spread frames; the remaining per-column preview frames follow in the background. All lie on the full-pass lattice and contribute exactly once.
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

- [x] Record baseline timings and preserve a CLI binary for output comparison.
- [x] Implement bounded opening-level scans with unchanged CLI defaults.
- [x] Add cancellable preview/refinement and shared paired snapshots to DSP.
- [x] Integrate latest-request cancellation, progressive rendering and refinement indication.
- [x] Cover exact final results, preview geometry, cancellation and bounded queues with tests.
- [x] Measure first-picture latency, snapshot cost and native responsiveness.
- [x] Update architecture, user documentation and #31 integration status.
- [x] Complete independent review and validation; move this plan to completed before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] CLI image comparison against baseline fixtures
- [x] Release timing on roughly 10 MB, 1 GB and 10 GB captures, including f16x8 opening
- [x] Dark/light native checks of preview, refinement and replacement during active analysis
- [x] Focused external review with GPT-5.6 Sol at High effort

## Post-completion

Keep the PR Draft for owner review. Continue with a dependent focused branch for #32,
then #62; leave #30 deferred. Merge only after owner acceptance and full CI.

## Implementation notes

The GUI limits FFT workers to eight and switches mmap access hints between sparse
preview and sequential refinement. Each bounded preview batch hints its selected
ranges before reading, allowing Unix mmap pages to arrive concurrently. Readers
without prefetch support retain the same numerical behavior. Display-only waveform scaling is held in the
document separately from measured sample peaks. Empty ruler boundaries are drawn
before opening; physical numeric labels necessarily wait for file metadata.

Release measurements used an isolated GPU-backed Wayland compositor at 1280x800,
with fully written synthetic I/Q f32 captures and no competing build. Launch to the
first image paint was 126.1 / 129.0 / 143.4 ms for 10 MB / 1 GB / 10 GB. These are
controlled measurements, not a latency guarantee for every storage device or codec;
the marker records the application's paint call, not a compositor presentation fence.
The baseline finished analysis in 122.5 ms / 4.68 s / more than 15 s respectively;
its marker is analysis completion, so it is not an exact first-presentation comparison.

Early measurements exceeded the 150 ms goal. Bounded preview and sparse mmap access
made latency independent of capture size, and per-batch Unix prefetch removed the
remaining serialized page-read delay without reducing the 128-frame first preview.

## External review, round 1

Accepted two findings: remaining preview frames needed bounded, cancellable batches,
and a bounded channel alone did not bound retirement textures during paused frames.
Preview now batches at most 1 MiB of samples (or one larger FFT frame); shell render
coalesces snapshots before upload, preserving the two-frame retirement discipline.
No findings were declined. The follow-up found both fixes resolved and no substantive issues.
A native 10 GB capture stayed between 175 and 184 MiB RSS during eight seconds hidden;
restoring the window and resizing it repeatedly resumed current refinement without failure.
Both themes were inspected. All 12 input captures in `tests/signals` produced byte-identical
640x400 CLI image pixels against the pre-change release binary.

Copying a 1600x600 dB grid and shading it measured 8.502 ms per snapshot, or 170 ms
CPU per second at 20 Hz; each RGBA snapshot holds 3,840,000 bytes. This excludes FFT,
level resolution, waveform cloning, GUI upload and presentation.
A correctly tagged 1 GB CoolEdit f16x8 capture took 8.8 ms to open with the 64 MiB
automatic normalization budget; its first paint was 130.6 ms from launch. The original
measurement fixture used an invalid fmt size and was discarded before this result.

A final focused review of the bounded prefetch addition found no substantive issues.
