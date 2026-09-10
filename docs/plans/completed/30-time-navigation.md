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
  crop to the plot and use the retained wider picture for known time while the complete replacement is calculated.
  Never stretch old data across a different physical time interval.
- Submit ranges through the existing latest-request mailbox. Preserve the
  worker's single-pass and cancellation guarantees.
- Keep sample ranges in memory with recent-file entries during one run; reset them to full capture on restart. Ignore ranges saved by older sessions.
- Read cursor levels from the displayed grid at its own extents, including
  during placeholder rendering; do not report levels in uncovered areas.

## Rejected alternatives

- Waiting for a transform before moving the view would make input latency
  depend on the capture length.
- Frequency zoom and editing gestures would broaden the issue beyond time
  navigation; reserve them for their own increments.

## Implementation steps

- [x] Implement and test bounded navigation, cursor lookup and in-memory recent views.
- [x] Integrate synchronized immediate rendering and mouse/keyboard controls.
- [x] Check superseded requests, range-specific metadata and settings changes.
- [x] ➕ Add fractional time labels for zoomed GUI views; preserve CLI clock formatting.
- [x] Update relevant documentation.
- [x] ➕ Bound deep-zoom GPU coordinates with visible source-column textures and stage navigation persistence without filesystem writes.
- [x] ➕ Address review findings: exact full-capture provenance, representable time spans, FFT-preview cancellation, RF cursor precision, bounded texture-strip rendering, and redundant redraws.
- [x] Complete local validation and address independent review findings.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Native GPU checks: anchored zoom, drag, keyboard-only navigation,
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
accepted. Extremely large counts now retain a screen-resolution representable time span;
file-wide extrema use exact request provenance. Settings cancellation restores
automatic view expansion, cursor precision follows zoom, deep previews use a
bounded number of narrow textures, and pointer moves outside plots no longer
redraw the picture. The stale README statement was removed.

The reviewed build also passed the 24 MS/s cursor check and the native
2048-sample view -> FFT 65536 preview -> Escape scenario. Both FFT 2048 and the
exact opening range were restored and saved on close. The local gate passed
458 tests; the release binary was rebuilt after it.

The second review accepted the other fixes and requested that the extreme-count
floor cover every display column and 10% keyboard pans. The floor now reserves
two ULPs per device-pixel column, is reapplied on width changes, and is tested
with an interior u64-scale view, one-pixel and 10% pans, a 1500-column grid and
waveform at 24 MS/s and 1 GS/s. Waveform mapping samples pixel centres.
Cursor grid lookup interpolates relative to the held grid, avoiding an
absolute-time rounding trip; displayed time remains an absolute capture time.

The third independent review returned no actionable findings. All findings from
the preceding rounds were accepted and addressed; none were deferred or rejected.
The final release passed repeated native RF, resizing, endpoint navigation and
one-hour deep-zoom checks. Linux CI is blocked before compilation by an unrelated
Google Chrome APT index checksum mismatch, tracked separately in #78.

## Owner feedback: zoom performance

Owner testing rejected the current zoom/pan redraw: sparse replacement previews flicker and deep zoom completes too slowly. Earlier native checks did not establish acceptable continuous navigation.

- [x] Measure current short-range analysis and preview/render overhead.
- [x] Keep navigation from replacing held detail with sparse intermediate snapshots.
- [x] Add a bounded sequential path for small FFT workloads, retaining cancellation and resize/style cache semantics.
- [x] Validate real/IQ, Max/MeanPower, rapid zoom and cancellation; record timings and native limitations.
- [x] Repeat the local gate, release build and independent review for this iteration.

Full-capture waveform minimap behaviour belongs to #79 and a separate PR.

Evidence: [zoom measurements and native checks](../../performance/30-zoom.md).

## Navigation interaction follow-up

- [x] Restrict the crosshair to the spectrogram; retain the drag cursor during panning.
- [x] Accept left-drag panning and ordinary mouse-wheel panning on the time ruler.
- [x] Keep recent-file views only in memory during one run; ignore legacy saved ranges and start each new launch at full capture.
- [x] Validate ruler gestures, cursor regions and restart behavior on the rebuilt native application.

The restart requirement above supersedes the original per-file persistence acceptance criteria and its earlier restoration checks.

The final iteration passed formatting, strict Clippy, all 469 local tests and a fresh
release build. Native GPU checks covered the zoom replacement, palette swap,
ruler gestures, cursor regions and restart behavior described in
`docs/performance/30-zoom.md`. Two review rounds identified five issues across
backdrop readouts, style retention, replacement policy, progress and parked delivery
ownership; these were addressed. The next zoom review and the separate interaction
review returned no substantive findings. Keeping an already displayed sparse preview
as an approximate visual placeholder was retained deliberately, with numeric levels disabled.

## Alt cursor guides

- [x] Draw cursor-to-ruler guides while Alt is held over the spectrogram, with time and physical frequency badges on the rulers.
- [x] Keep badges within the panel at edges; hide guides on release, pointer exit and window deactivation without requesting analysis.
- [x] Verify coordinate mapping, native modifier transitions, full local gate, release build and independent review.

Vertical frequency zoom and panning are a separate follow-up issue and are not implemented in this iteration.

Native checks on the release build with a real GPU passed stationary-pointer Alt
press/release, both plot corners, pointer exit to the waveform, window deactivation
and reactivation after releasing Alt in another window. Pixel comparison confirmed
that release restores the original spectral/ruler pixels; analysis completion count
remained one throughout modifier and pointer checks. Physical mapping tests cover
negative I/Q frequency, RF centre frequency and fractional capture time.

The Alt-guide iteration passed all 473 local tests, formatting, strict Clippy and a
fresh release build. Review identified a stale menu handle suppressing guides and
logical-pixel badge precision on scaled displays. Both were fixed; no findings were
declined and the follow-up review was clean. Final native checks passed Alt release
and reactivation after both File and View menu dismissal, plus a 2x output displaying
five fractional time digits at the one-FFT zoom floor.
