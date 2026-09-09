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

- [x] Implement and test bounded navigation, cursor lookup and view persistence.
- [x] Integrate synchronized immediate rendering and mouse/keyboard controls.
- [x] Check superseded requests, range-specific metadata and settings changes.
- [x] ➕ Add fractional time labels for zoomed GUI views; preserve CLI clock formatting.
- [x] Update relevant documentation.
- [x] ➕ Bound deep-zoom GPU coordinates with visible source-column textures and stage navigation persistence without filesystem writes.
- [x] ➕ Address review findings: exact full-capture provenance, representable time spans, FFT-preview cancellation, RF cursor precision, bounded texture-strip rendering, and redundant redraws.
- [ ] Complete validation and independent review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Native GPU checks: anchored zoom, drag, keyboard-only navigation,
  synchronized panels, cursor units/levels, rapid input, restoration and bounds.

## Post-completion

None.

## Native validation

Checked on Linux in an isolated 1600 x 1000 Sway output using the real GPU:
real and complex 8 kHz fixtures, a 12.5 MHz centre frequency, dark/light themes,
760 x 620 and larger windows, pointer-anchored wheel zoom, drag pan, every View
keyboard command, a one-FFT end-of-file view and restoration after restart.
A one-hour real capture was zoomed directly to 2048 samples; the captured
in-progress frame already had the requested axes and transformed paired
pictures, followed by the refined view at the same time coordinates.

The first independent review found four medium and two low issues. All were
accepted. Extremely large counts now retain a representable minimum time span;
file-wide extrema use exact request provenance. Settings cancellation restores
automatic view expansion, cursor precision follows zoom, deep previews use a
bounded number of narrow textures, and pointer moves outside plots no longer
redraw the picture. The stale README statement was removed.
