# Issue #31: Waveform panel

Tracks [Issue #31](https://github.com/o-kos/argand/issues/31).

## Overview

Replace the waveform placeholder with a linear time-domain envelope above the
spectrogram. Preserve transients, distinguish I and Q in one track, and remember
the user-adjusted panel split.

## Context

The DSP already computes a channel-separated `WaveformEnvelope` in the same
sample-reading pass as the spectrogram. The GUI currently does not request it.
Issues #29 and #30 are still open: progressive refinement and time navigation
are unavailable and their integration criteria cannot be completed in this increment.

## Decisions

- Start at 3 rem including the separator, following the current Issue #31 and #49 requirements.
- Use the spectrogram's exact horizontal plot geometry and request column count.
- Keep linear amplitude values and distinguish I and Q with labelled colours.
- Persist a user-adjusted panel proportion; keep the font-relative default until adjusted.
- Retain the latest waveform and spectrogram together while a replacement is computed.
- Compare envelope values with the shared DSP used by `aspec`; the CLI deliberately merges channel spans, so its pixels are not a two-colour GUI reference.
- Keep Issue #31 open until the #29/#30 integration criteria are implemented and verified.

## Rejected alternatives

- Independent waveform analysis would duplicate sample reads and risk time alignment drift.
- Implementing #29 and #30 implicitly here would expand this branch into two separate milestones.

## Implementation steps

- [ ] Request and expose the waveform envelope alongside each spectrogram.
- [ ] Draw aligned real and I/Q traces with a labelled linear scale.
- [ ] Implement and persist a bounded draggable panel separator.
- [ ] Test transient preservation, alignment, channel visibility and restored panel layout.
- [ ] Update the changelog and architectural status.
- [ ] Complete the local gate, release build and external review.
- [ ] Verify native rendering and dragging with a current release binary.
- [ ] ⚠️ Integrate progressive refinement after #29 and synchronized navigation after #30.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Native GPU checks for real and complex captures, narrow windows, themes and DPI
- [ ] External review using GPT-5.6 Sol with High reasoning effort

## Post-completion

Move this plan to `completed/` only when the remaining dependency criteria are fulfilled.
