# Issue #31: Waveform panel

Tracks [Issue #31](https://github.com/o-kos/argand/issues/31).

## Overview

Replace the waveform placeholder with a linear time-domain envelope above the
spectrogram. Preserve transients, merge I and Q in one track, and remember
the user-adjusted panel split.

## Context

The DSP already computes a channel-separated `WaveformEnvelope` in the same
sample-reading pass as the spectrogram. The GUI requests it at the spectrogram column count.
Issues #29 and #30 are still open: progressive refinement and time navigation
are unavailable and their integration criteria cannot be completed in this increment.

## Decisions

- Start at 3 rem including the separator, following the current Issue #31 and #49 requirements.
- Use the spectrogram's exact horizontal plot geometry and request column count.
- Use the same merged min/max trace and linear scaling as `aspec`, without channel or amplitude labels.
- Persist a user-adjusted panel proportion; keep the font-relative default until adjusted.
- Retain the latest waveform and spectrogram together while a replacement is computed.
- Share waveform pixel spans and amplitude scaling with `aspec`, preserving its CLI output.
- Keep Issue #31 open until the #29/#30 integration criteria are implemented and verified.

## Rejected alternatives

- Independent waveform analysis would duplicate sample reads and risk time alignment drift.
- Implementing #29 and #30 implicitly here would expand this branch into two separate milestones.

## Implementation steps

- [x] Request and expose the waveform envelope alongside each spectrogram.
- [x] Draw an aligned merged real/IQ trace without additional labels.
- [x] Implement and persist a bounded draggable panel separator.
- [x] Test transient preservation, alignment, merged channel extrema and restored panel layout.
- [x] Update the changelog and architectural status.
- [x] Complete the local gate, release build and external review.
- [x] Verify native rendering and dragging with a current release binary.
- [ ] ⚠️ Integrate progressive refinement after #29 and synchronized navigation after #30.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Native GPU checks for real and complex captures, narrow windows, themes and DPI
- [x] External review using GPT-5.6 Sol with High reasoning effort

## Post-completion

Move this plan to `completed/` only when the remaining dependency criteria are fulfilled.

## Validation evidence

- All 389 local tests pass, followed by a release build.
- The worker test checks every I/Q column against an independent sample scan,
  including a one-sample burst and the final capture sample.
- GPU-backed headless Wayland checks cover real and complex captures, dark and
  light themes, 300- and 640-pixel windows, and 100%, 125% and 200% display scale.
- Native multi-step pointer dragging saves the split on normal window close.
  After restarting, the waveform area
  is pixel-identical to the adjusted layout in both themes.
- Native Windows/macOS rendering remains unverified.

## External review follow-up

- Accepted: coincident I/Q fills collapsed to an unidentified blended band.
  Partition the channel spans into I-only, Q-only and explicit `Both` overlap
  bands, with matching legend colours and primitive-level regression tests.
- Accepted: splitter motion could start a full analysis for an intermediate
  height. Coalesce requests during the gesture and ask once from the final
  layout after mouse-up; retain the displayed pair throughout the drag.
- No findings declined. The focused follow-up review found both defects closed
  and no substantive regressions.
- Snap adjusted panel boundaries to device pixels, including fractional DPI.

- Final native checks confirm no analysis completes during a multi-step splitter
  gesture and exactly one completes after mouse-up, in both themes.
- Final native checks include identical I/Q samples and adjusted panel proportions
  restored at 125% and 200% DPI.

## Owner feedback: match the aspec waveform

The owner superseded the original separate-channel and labelled-scale criteria:
use one merged envelope, as `aspec` does, with no extra text. This also supersedes
the first review's overlap-colour remedy; the deferred splitter request fix remains.

- [x] Share channel merging, pixel rounding and adjacent-column joining through `WaveformEnvelope::pixel_spans`.
- [x] Share default and peak-relative amplitude scaling with `aspec`.
- [x] Remove channel colours, overlap bands, legend and amplitude caption; return their row to the trace.
- [x] Verify unchanged CLI PNG pixels in both orientations and every range mode.
- [x] Repeat the local gate, release build, native checks and focused external review.

The merged-waveform revision passes all 389 tests, formatting and strict Clippy,
followed by a release build. All 24 CLI PNG comparisons are pixel-identical:
real/IQ, horizontal/vertical, default/fixed/auto range, and 0/12 dB input gain.
GPU-backed Wayland checks confirm the unlabelled single trace in both themes,
at 100%, 125% and 200% DPI, with the splitter gesture and restart checks passing.
Focused external review found no substantive findings; none were declined.
