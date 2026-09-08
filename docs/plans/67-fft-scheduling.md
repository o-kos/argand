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

- [ ] Audit scheduling, FFT library behavior, allocation, serial work and UI delivery.
- [ ] Build a reproducible benchmark and collect baseline/thread/batch/placement measurements.
- [ ] Implement justified scheduling changes and configurable worker count.
- [ ] Assess optional topology-aware affinity and document automatic planning proposals.
- [ ] Verify numerical equivalence, cancellation, responsiveness and configuration behavior.
- [ ] Document measurements, limits and architecture; complete independent review.
- [ ] Move the plan to completed before owner review.

## Validation

- [ ] Formatting, Clippy and full tests with locked dependencies.
- [ ] Release build after checks, in a separate target directory.
- [ ] Multiple runs, warm-up and thermal/cache observations on the reference CPU.
- [ ] Native UI latency measurement and visual inspection.
- [ ] External review with GPT-5.6 Sol High, clean final round.

## Post-completion

Keep the PR Draft for owner feedback. Its base follows #63 until #29 is accepted;
do not merge or resume #32/#62 without the owner's corresponding approval.
