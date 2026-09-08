# Issue #74: Reuse analysis across plot sizes

Resolves https://github.com/o-kos/argand/issues/74.

## Overview

Make window and splitter resizing a display operation, including during progressive analysis. Keep complete FFT coverage and bounded memory; explicitly describe the approximation at cached cell boundaries.

## Context

The worker currently cancels and reruns analysis for each image size. Screen-sized reductions discard the data needed for another size. The aggregation implementation is accepted but stacked PR integration remains pending.

## Decisions

- Stack on `feature/69-spectrogram-aggregation` to retain the accepted current behavior; do not merge pending parents or include the separate settings branch.
- Keep a toolkit-neutral overview with at most 4096 time columns and 2048 frequency rows. Preserve native frequency bins when they fit; group larger FFTs into bounded rows. The dominant retained accumulator is at most 32 MiB for Peak or 64 MiB for Mean power, independent of file duration.
- Retain a separate sample min/max envelope of at most 65536 columns, and full PSD metadata. Do not retain every FFT or build canonical-size RGBA snapshots.
- Rebin before shading: conservative overlapping-cell maxima for Peak; overlap- and frame-count-weighted linear powers for Mean power. Aligned cache boundaries are exact apart from existing floating-point rounding. Crossing a cache boundary is an explicit approximation; increasing the window beyond cache resolution cannot create new detail.
- Coalesce display requests without changing the analysis generation. Actual file/range/FFT/reducer/style changes still invalidate analysis. Retain the original analysis duration on redraw.
- Keep the ordinary DSP/CLI analysis contract unchanged. Legacy CLI mean-in-dB is not an overview reducer.

## Rejected alternatives

- Store every FFT: approximately 4 GB for the reference recording, growing with duration.
- Cache only the original RGBA image: cannot preserve reduction semantics or recover native frequency information.
- Sparse FFT sampling: changes the analyzed signal coverage.
- Recompute after a resize debounce: still repeats the whole file and interrupts refinement.

## Implementation steps

- [ ] Add bounded overview analysis and rebinning, sharing the existing progressive transform loop.
- [ ] Separate display requests from analysis invalidation in the document worker.
- [ ] Verify cache boundary arithmetic, both signal domains/reducers, cancellation and resizing during refinement.
- [ ] Measure baseline/current native resize and initial load behavior, including memory and UI timer observations.
- [ ] Document resolution, performance and unverified conditions; update architectural context.
- [ ] Complete independent review and move the plan to `docs/plans/completed/`.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after checks
- [ ] Repeated real GPU comparisons with the retained 1 GB recording and short real/IQ fixtures
- [ ] Independent read-only GPT-5.6 Sol High review, iterated until clean

## Post-completion

Keep the PR Draft for owner feedback. Integration follows acceptance and full CI of the parent stack.
