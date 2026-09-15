# Issue #103: Endpoint scale labels in vertical mode

Resolves [#103](https://github.com/o-kos/argand/issues/103).

## Overview

Restore missing endpoint labels where the vertical layout has room, without
overlapping neighboring labels, unit captions, or adjacent panels.

## Context

The report concerns `tests/signals/m39.wav`. `crates/app/src/axes.rs` supplies
label geometry to the shared layout in `crates/core/src/axis.rs`; vertical mode
places frequency below the plot and time in the right gutter.

## Decisions

- Confirm the affected ruler and reproduce the rejection before changing policy.
- Keep the shared layout toolkit-neutral and preserve existing CLI behavior.

## Rejected alternatives

- Do not force labels visible without measuring available space and collisions.

## Implementation steps

- [ ] Reproduce and identify the omitted labels and their available geometry.
- [ ] Correct the fitting policy and add focused regression coverage.
- [ ] Update the changelog and relevant architectural documentation.
- [ ] Complete local validation and external review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Check m39.wav in vertical mode and horizontal nonregression, including
  constrained geometry and neighboring labels/unit captions.

## Post-completion

Squash-merge after owner acceptance and successful full CI; remove the branch.
