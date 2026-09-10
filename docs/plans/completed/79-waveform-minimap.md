# Issue #79: Full-capture waveform minimap

Resolves #79: https://github.com/o-kos/argand/issues/79.

## Overview

Keep the waveform's time extent and amplitude scale independent of spectrogram
navigation. Keep the requested time viewport bright and darken only the waveform outside it,
without a border or fill. Full capture has no dimming; deep zoom keeps a visible pixel.
Every file opening still resets the time view; no saved zoom is restored.

## Context

The current shell replaces its waveform with every spectral result and stretches
both panels during navigation. The document worker cancels full-capture analysis
when the time range changes, so its cache alone cannot guarantee a complete minimap.
The existing toolkit-neutral EnvelopeBuilder preserves real/IQ extrema.

## Decisions

- Build the full-capture envelope in an independent cancellable document task,
  using a bounded reader and at most 65536 min/max cells per channel. Publish a
  sparse full-width preview before sequential completion; navigation never
  restarts this task. File replacement cancels it without joining the UI thread.
  Reopen with known length and normalization to avoid unbounded pre-scan work.
- FLAC seeks recreate the parser with known metadata: reusing its buffered packet
  state after preview can truncate refinement or the full minimap scan.
- Retain the envelope independently of FFT, palette, reducer and range changes.
  Rebin cached extrema conservatively for display and cache the resulting spans
  by panel size. Initial preview/refinement belongs to file loading only.
- Draw the viewport from integer sample bounds, clamp it to the capture and give
  sub-pixel spans a minimum visible width without changing the actual range.
- Outside click/Ctrl+click pans one/five ruler divisions through the keyboard path.
  Inside single/double click only arms dragging; outside double-click centres at
  the pointer. The viewport
  and both rulers show an open hand, and active drags a closed hand. Spectrum and
  time-ruler wheel/drag behavior stays as accepted in #30. Cursor time above the
  minimap refers to the full capture.
- Keep the existing splitter, merged I/Q rendering and no-grid waveform style.

## Rejected alternatives

- Holding the first spectrum waveform cannot finish a minimap when navigation
  interrupts the initial pass. Recomputing a full FFT solely for the minimap
  would waste work and delay requested spectral detail.
- Storing every sample or every display width violates the bounded cache policy.
- Delaying every outside click until the double-click timeout would slow ordinary
  navigation. The first press responds immediately; a second press centres from
  full-capture coordinates only if it remains outside the updated viewport.
  Containment is checked on each press, so moving the viewport under the pointer
  makes the second press stationary.

## Implementation steps

- [x] Add bounded, independently cancellable full-capture waveform generation and tests.
- [x] Retain minimap data across range/style changes and reset it on file replacement.
- [x] Paint cached full-range extrema with inverse viewport highlighting.
- [x] Match minimap input and cursor coordinates to the full capture.
- [x] Update issue wording, README, changelog and architectural invariants.
- [x] Complete native verification and independent review; address substantive findings.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Tests cover real/IQ extrema, bounded reads, cancellation, conservative rebinning,
  viewport ends/deep zoom, same-process reopening and generation independence.
- [x] Native real-GPU checks cover stable waveform pixels through zoom/pan/style
  changes, early navigation, minimap gestures, resizing and real/IQ captures.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after the gate passes

## Results

- Formatting, strict Clippy and all 492 local tests pass. The release binary was
  rebuilt after the gate; no lint policy was relaxed.
- Native real-GPU checks on real and I/Q captures preserve the waveform geometry
  and background through zoom, pan, palette and resizing. Outside click/Ctrl+click
  exactly match one/five keyboard divisions in both directions; inside clicks
  do not navigate; outside double-click sequences centre correctly. Cursor glyphs
  show an open hand before pressing and a closed hand during dragging.
- A 1 GB capture finishes its minimap despite early zoom; a 10 GB scan stops on
  replacement without delaying the new capture. Same-process reopening resets
  the view. A 2x display retains one bright physical pixel at the capture end.
- Unknown-length FLAC default and Auto opening each perform one length scan.
  Native preview/refinement and minimap completion pass after parser resets;
  fixture regressions cover complete remaining samples after repeated seeks.
- Review corrections remove duplicate count/normalization work, replace the
  pending extrema hint after failure, and accurately document preview completion.
  Shared device-pixel geometry fixes the HiDPI hitbox. The final independent
  review found no substantive issues, including the FLAC parser fix. The proposed
  single-click delay was declined with the reasoning above and challenged in a
  follow-up review; no correctness failure beyond the deliberate intermediate
  step was identified.

## Owner feedback: clicks and drag anchoring

- [x] Keep both single and double clicks inside the bright interval stationary;
  retain double-click centring outside it and dragging from inside it
- [x] Check a long drag in both directions before changing its coordinate mapping
- [x] Validate the updated click handling, rebuild release and review the follow-up

On the existing release, dragging 600 logical pixels in either direction retained
exactly 46 pixels between the pointer and the interval's left painted edge at every
sampled position. Sample-rounding error stayed below 0.002 logical pixels. No
interior drift was reproduced; clamping at the capture boundaries is expected.

The updated release also passed native checks for inside double-clicks with and
without Ctrl, full-capture double-clicks, outside centring and drag anchoring.
Formatting, strict Clippy and all 492 tests pass. Review identified an inaccurate
description of a second press landing inside the moved viewport; the README and
plan now document the per-press containment rule. The final review round found
no substantive issues.

## Post-completion

Continue with #81, then #80, the remaining grid/ruler backlog, and #82.
