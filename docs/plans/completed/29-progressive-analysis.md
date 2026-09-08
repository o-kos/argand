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
- [x] Integrate latest-request cancellation, progressive rendering and status-bar progress.
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

Keep the PR Draft and wait for owner acceptance before resuming #32, then #62.
Leave #30 deferred. Merge only after owner acceptance and full CI.

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

## Owner feedback

Removed the moving refinement line from both panels. Sequential refinement and
status-bar progress remain; the owner will assess the picture without the overlay.
Wait for owner acceptance of #29 before resuming #32.
The focused review of this removal found no substantive issues.

## Full-pass performance follow-up

The owner authorized fixing the measured full-pass overhead before reconsidering
display-width-limited spectral sampling. Retain every FFT frame, Max/Mean semantics,
progressive snapshots, cancellation, and the current eight-worker limit.

- [x] Defer Max row logarithms until column reduction and reuse FFT scratch buffers.
- [x] Cache grid values and colours, refreshing changed columns and invalidating colours when the scale changes.
- [x] Verify intermediate and final output against uncached rendering and ordinary analysis.
- [x] Repeat the full gate, release measurements on m39 at four/eight workers, and native inspection.
- [x] Complete external review and present the Draft iteration for owner acceptance.

The follow-up passes formatting, Clippy and all 402 tests. The 1 GB m39 final
f32 dB grid is byte-identical to the pre-optimization result. In sequential
release DSP measurements outside the sandbox at 1214x662, four workers changed
from 12.672 s to 10.578 s and eight from 12.585 s to 9.613 s. All 976,559 frames
and intermediate snapshots remain enabled. These individual measurements depend
on host load and output dimensions; they are not a timing guarantee. A rebuilt
1280x800 GPU window completed the same recording in 8.019 s on eight available
CPUs, with the preview and final image inspected. GUI affinity also constrains
rendering threads, so it is not the same experiment as changing only the DSP pool.
The cache still clones owned grids/images for delivery and uploads whole textures;
this iteration avoids repeated DSP/logarithm/colour work, not all copy/upload cost.

External review accepted one coverage finding: the first cache test changed its
shading between updates and did not independently exercise selective recolouring.
A dedicated fixed-shading test now checks an updated column, its empty followers,
and an unchanged control column against a full render. No findings were declined.

The focused follow-up review confirmed the coverage finding resolved and found no
remaining substantive issues. The final round was clean.
