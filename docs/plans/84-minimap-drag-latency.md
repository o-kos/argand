# Issue #84: Reduce minimap drag latency

Resolves #84.

## Overview

Keep the bright minimap viewport attached to the native cursor during continuous
motion while immediately moving the held spectrogram. Preserve full-capture
waveform content, exact final analysis and stopped-position grab accuracy.

## Context

The accepted #89 tree supplies both orientations and current navigation controls.
This branch is stacked on #89 while the accepted PR chain awaits integration.
The prior #84 measurements distinguished continuous-motion lag from rounding or
clamping: delaying FFT requests alone reduced tail latency but did not meet the
acceptance target. Minimap and spectrum currently share one GPUI canvas/frame.

## Decisions

- Measure the current release on a real GPU before changing scheduling or drawing
- Compare unchanged motion speed, output refresh/scale, viewport width and captures
- Keep native pointer behavior; do not hide, warp or predict it to mask delay
- Keep expensive work away from pointer handling and preserve cancellation

## Rejected alternatives

- Changing paint-call order cannot present part of one frame earlier
- Deferring all FFT requests until release was previously insufficient by itself

## Implementation steps

- [ ] Reproduce continuous-motion latency on the accepted baseline and record evidence
- [ ] Profile event handling, frame preparation and presentation; identify contributors
- [ ] Implement and measure the smallest correction meeting the continuous-motion target
- [ ] Verify steady grab offsets, reversals, bounds, outside release and file replacement
- [ ] Cover real/IQ captures, both orientations and normal/high display scales
- [ ] Document measurements, limitations and rendering invariants
- [ ] Complete independent review and address substantive findings
- [ ] Move this plan to `docs/plans/completed/` before owner review

## Validation

- [ ] Formatting, strict Clippy and full workspace tests
- [ ] Release rebuilt after the local gate
- [ ] Native GPU before/after distributions for the issue's approximately 46-pixel viewport
- [ ] No cursor escape on the stated interior centre-grab motion test
- [ ] No loss of final analysis, navigation controls or cancellation
