# Issue #67: CPU-aware FFT scheduling

Resolves #67. Based on the #29 iteration in Draft PR #63 while owner validation continues.

## Overview

Reduce large-file visualization latency without monopolizing the desktop. Keep
exhaustive FFT coverage and progressive feedback. Separate thread-count, batching,
core-placement and cache effects before selecting defaults.

## Decisions

- Preserve the owner's current target/release binaries; build this iteration in a separate target directory.
- Benchmark fixed m39-repeat-1GB.wav, Hann 2048, hop 512 and fixed image dimensions.
- Use repeated balanced-order measurements, record CPU temperature and workload,
  and distinguish warm-file tests from file-local cold-cache requests and compute tests.
- Apply experimental affinity only inside compute workers; UI and unrelated apps retain their existing masks.
- No global scheduler changes. No hard-coded CPU identifiers in product defaults.
- Keep affinity optional unless evidence establishes a portable automatic policy.

## Implementation steps

- [x] Audit scheduling, FFT library behavior, allocation, serial work and UI delivery.
- [x] Build a reproducible benchmark and collect baseline/thread/batch/placement measurements.
- [x] Implement justified scheduling changes and configurable worker count.
- [x] Assess optional topology-aware affinity and document automatic planning proposals.
- [x] Verify numerical equivalence, cancellation, responsiveness and configuration behavior.
- [x] Document measurements, limits and architecture; complete independent review.
- [x] Move the plan to completed before owner review.

## Validation

- [x] Formatting, Clippy and full tests with locked dependencies.
- [x] Release build after checks, in a separate target directory.
- [x] Multiple runs, warm-up and thermal/cache observations on the reference CPU.
- [x] Native UI latency measurement and visual inspection.
- [x] External review with GPT-5.6 Sol High, clean final round.

## Post-completion

Keep the PR Draft for owner feedback. Its base follows #63 until #29 is accepted;
do not merge or resume #32/#62 without the owner's corresponding approval.

## Measurement decisions

- Keep eight unrestricted workers as the default; 10–12 workers increased CPU cost without a reliable time benefit.
- Combine Max rows within partials and use 1024-frame batches with dynamic groups; preserve independent FFT counts for PSD.
- Reject precomputed row boundaries: the isolated experiment did not improve throughput.
- Provide opt-in Linux Intel hybrid E-core discovery and portable fallback; no hard-coded masks in product code.
- Preserve every measured run and identify thermal/desktop variability rather than filtering convenient results.
- The detailed report and raw data live in `docs/performance/67-fft-scheduling.md` and `67-data/`.
