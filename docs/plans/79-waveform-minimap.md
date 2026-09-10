# Issue #79: Full-capture waveform minimap

Resolves #79: https://github.com/o-kos/argand/issues/79.

## Overview

Keep the waveform's time extent and amplitude scale independent of spectrogram
navigation. Show the requested time viewport as a coloured overlay on the whole
capture, including a visible marker when its width is less than one pixel.
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
- Retain the envelope independently of FFT, palette, reducer and range changes.
  Rebin cached extrema conservatively for display and cache the resulting spans
  by panel size. Initial preview/refinement belongs to file loading only.
- Draw the viewport from integer sample bounds, clamp it to the capture and give
  sub-pixel spans a minimum visible width without changing the actual range.
- Minimap click/drag moves the time viewport over the full capture. Spectrum and
  time-ruler wheel/drag behavior stays as accepted in #30. Cursor time above the
  minimap refers to the full capture.
- Keep the existing splitter, merged I/Q rendering and no-grid waveform style.

## Rejected alternatives

- Holding the first spectrum waveform cannot finish a minimap when navigation
  interrupts the initial pass. Recomputing a full FFT solely for the minimap
  would waste work and delay requested spectral detail.
- Storing every sample or every display width violates the bounded cache policy.

## Implementation steps

- [ ] Add bounded, independently cancellable full-capture waveform generation and tests.
- [ ] Retain minimap data across range/style changes and reset it on file replacement.
- [ ] Paint cached full-range extrema plus a contrasting viewport overlay.
- [ ] Match minimap input and cursor coordinates to the full capture.
- [ ] Update issue wording, README, changelog and architectural invariants.
- [ ] Complete native verification and independent review; address substantive findings.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] Tests cover real/IQ extrema, bounded reads, cancellation, conservative rebinning,
  viewport ends/deep zoom, same-process reopening and generation independence.
- [ ] Native real-GPU checks cover stable waveform pixels through zoom/pan/style
  changes, early navigation, minimap gestures, resizing and real/IQ captures.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after the gate passes

## Post-completion

Continue with #81, then #80, the remaining grid/ruler backlog, and #82.
