# Issue #57: Spectrogram scale alignment

Resolves #57.

## Overview

Move the frequency scale to the right, align numeric ink vertically with ticks, and place time labels to the right of ticks without clipping or overlaps.

## Context

`crates/app/src/axes.rs` owns GUI drawing; `argand-core::axis` chooses readable ticks for both GUI and CLI. The branch was reconciled onto main after accepted parent PRs #50, #54 and #56 merged; file-tree comparisons confirmed that reconciliation changed no implementation.

## Decisions

- Reserve a right gutter for frequency labels and their unit
- Add an explicit opt-in label anchor to shared layout, retaining centered CLI defaults
- Use shaped-line metrics for the baseline actually used by GPUI painting
- Keep time labels within the plot width to avoid crowding the frequency gutter

## Rejected alternatives

- Moving paint coordinates alone would invalidate collision and edge checks
- A fixed vertical offset would depend on font and DPI

## Implementation steps

- [x] Update shared label placement and GUI geometry
- [x] Cover label bounds and spacing, preserving existing CLI behavior
- [x] Validate native rendering at normal and scaled DPI
- [x] Complete local gate and release build
- [x] Complete external review
- [x] Move the plan to completed

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the gate
- [x] Native GPU screenshots at 1x and 2x, with narrow panels
- [x] External review with the required model

## Validation evidence

- Formatting, Clippy and all 380 local tests pass; release rebuilt afterward
- GPU-backed Linux Wayland screenshots cover 300, 640 and 1200 pixel windows, dark/light themes, and 100%/200% DPI
- Numeric ink was approximately 3 logical pixels above the ticks before the fix; screenshot measurements after the fix place its center within 0.5 logical pixels of the tick center at both scales
- Native Windows/macOS rendering was not exercised

## External review follow-up

- Accepted a fractional-DPI gutter finding: rounding the plot edge outward could reduce reserved label space
- Added a regression test that first reproduced a frequency label extending beyond a 640px panel at 125% DPI
- Round the right plot edge inward to retain the complete label bounds at fractional scales
- No findings declined; focused follow-up review confirmed the fix with no substantive findings
- Fresh release also checked at 125% DPI with six-decimal frequency labels and hour-scale time labels
