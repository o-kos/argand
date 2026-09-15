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

- Reproduced on the right time ruler: m39.wav is 424703 samples at 7200 Hz
  (58.9865 s); the native window requested at 1200 x 800 logical pixels
  (restored as 1200 x 789) omits 0:02 and 0:58.
  A 1152 x 720 axis-panel fixture reproduces both omissions.
- The old bounds reserve a whole line plus nine pixels above and nine pixels
  inside the plot below. Instead, fit numeric ink between the actual caption
  ink with four pixels of clearance, accounting for the painted 0.5-pixel offset.
- Keep zero hidden where it collides with the top unit. Frequency rulers and
  horizontal layout retain their previous behavior.
- Keep the shared layout toolkit-neutral and preserve existing CLI behavior.

## Rejected alternatives

- Do not force labels visible without measuring available space and collisions.

## Implementation steps

- [x] Reproduce and identify the omitted labels and their available geometry.
- [x] Correct the fitting policy and add focused regression coverage.
- [x] Update the changelog and relevant architectural documentation.
- [x] Complete local validation and external review; the final round returned no
  substantive findings, with no findings requiring acceptance or rejection.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Check m39.wav in vertical mode and horizontal nonregression, including
  constrained geometry and neighboring labels/unit captions. Native before/after
  screenshots at 125% scale confirm 0:02 and 0:58; horizontal layout and CLI
  nonregression are covered by the unchanged full test suite.

## Post-completion

Squash-merge after owner acceptance and successful full CI; remove the branch.
