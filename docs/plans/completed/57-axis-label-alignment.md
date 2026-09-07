# Issue #57: Spectrogram scale alignment

Resolves #57.

## Overview

Move the frequency scale to the right, align numeric ink vertically with ticks, and place time labels to the right of ticks without clipping or overlaps.

## Context

`crates/app/src/axes.rs` owns GUI drawing; `argand-core::axis` chooses readable ticks for both GUI and CLI. This branch starts on accepted PR #56 while its parent chain completes full CI, and will be rebased onto main after those merges.

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
- [ ] Complete external review
- [x] Move the plan to completed

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the gate
- [x] Native GPU screenshots at 1x and 2x, with narrow panels
- [ ] External review with the required model

## Validation evidence

- Formatting, Clippy and all 379 local tests pass; release rebuilt afterward
- GPU-backed Linux Wayland screenshots cover 300, 640 and 1200 pixel windows, dark/light themes, and 100%/200% DPI
- Numeric ink was approximately 3 logical pixels above the ticks before the fix; screenshot measurements after the fix place its center within 0.5 logical pixels of the tick center at both scales
- Native Windows/macOS rendering was not exercised
