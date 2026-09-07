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

- [ ] Update shared label placement and GUI geometry
- [ ] Cover label bounds and spacing, preserving existing CLI behavior
- [ ] Validate native rendering at normal and scaled DPI
- [ ] Complete local gate, release build and external review
- [ ] Move the plan to completed

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the gate
- [ ] Native GPU screenshots at 1x and 2x, with narrow panels
- [ ] External review with the required model
