# Issue #30: Time navigation and cursor readout

Resolves #30.

## Overview

Navigate a capture in time with pointer-anchored wheel zoom, drag panning,
keyboard zoom and pan, and Home/End. Show the cursor's capture time, physical
frequency and held-grid level. Restore each recent file's last sample range.
Frequency zoom and selection editing remain outside this change.

## Context

The document worker already coalesces requests, cancels superseded generations
and retains an overview for display-size and style changes. Both panels share
the same horizontal geometry. Analysis results include a decibel grid with
physical extents. The shell currently always requests the full capture.

## Decisions

- Keep range arithmetic, clamping, placeholder mapping and cursor lookup free
  of GPUI and test them directly. The minimum span is one FFT, capped at the
  actual capture length for shorter inputs.
- Apply the requested range to axes and both held pictures in the next frame;
  crop to the plot and leave uncovered time blank until the next preview arrives.
  Never stretch old data across a different physical time interval.
- Submit ranges through the existing latest-request mailbox. Preserve the
  worker's single-pass and cancellation guarantees.
- Persist sample ranges with recent-file entries and validate them against the
  opened capture and current FFT. Preserve compatibility with older sessions.
- Read cursor levels from the displayed grid at its own extents, including
  during placeholder rendering; do not report levels in uncovered areas.

## Rejected alternatives

- Waiting for a transform before moving the view would make input latency
  depend on the capture length.
- Frequency zoom and editing gestures would broaden the issue beyond time
  navigation; reserve them for their own increments.

## Implementation steps

- [ ] Implement and test bounded navigation, cursor lookup and view persistence.
- [ ] Integrate synchronized immediate rendering and mouse/keyboard controls.
- [ ] Check superseded requests, range-specific metadata and settings changes.
- [ ] Update relevant documentation.
- [ ] Complete validation and independent review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Native GPU checks: anchored zoom, drag, keyboard-only navigation,
  synchronized panels, cursor units/levels, rapid input, restoration and bounds.

## Post-completion

None.
