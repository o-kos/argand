# Issue #32: Live analysis settings

Resolves #32. Builds on the progressive analysis in #29; navigation (#30) is deferred.

## Overview

Provide compact in-window analysis controls for the effective transform and display
settings. Keep the current picture while replacement work runs or a combination is
invalid. Persist effective values in session state without writing `argand.toml`.

## Decisions

- Keep a toolkit-independent effective-settings model with validation and a distinction
  between transform changes and display-only changes.
- Re-shade the cached grid for colour/range edits; never re-read samples or restart the
  transform for these edits. Apply current display settings to subsequent snapshots too.
- Track the settings that produced the displayed analysis separately from requested
  settings so diagnostics can remain accurate while a replacement runs.
- Use two status-bar groups: a combined file summary with one metadata hint, and an
  analysis summary opening an interactive popup. Group transform and display controls.
  The file hint includes sample count, file bytes, and original sample extrema (I/Q separately).
- Highlight an excessive display range in yellow using the aspec recommendation policy;
  make the recommended range directly applicable in the popup.
- Include #61 and the range-advice portion of #62 in this owner-approved scope. Keep
  the remaining signal-level/FFT diagnostic panels in #62.
- Store valid effective choices in a new backwards-readable session version; preserve
  configuration defaults and comments.

## Implementation

- [x] Add validated effective settings and persistence.
- [x] Add cached re-shading with the same range resolution as aspec.
- [x] Integrate progressive transform replacement and retain old images on errors.
- [x] Add the combined file status/hint including exact counts, file bytes and sample extrema.
- [x] Add the interactive analysis popup and clickable recommended range.
- [x] Cover setting transitions, persistence and no-transform display updates.
- [x] Validate native interaction and timing in both themes.
- [x] Update architecture and user documentation.
- [ ] Complete independent review and move the plan before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after checks
- [x] Cached colour/range edits present in under 30 ms in the measured GPU case; active refinement retains one transform pass (see validation report)
- [x] FFT/window/overlap/reduction changes cancel obsolete requests and refine a preview
- [x] Invalid settings preserve the last picture and report the problem
- [x] Restart restores effective settings and leaves configuration bytes unchanged
- [x] Dark/light native UI checks, including narrow windows
- [ ] Independent review with GPT-5.6 Sol at High effort

## Evidence and timing boundary

See [native validation and timing](../performance/32-status-settings.md). Cached edits
presented in 25.6–27.3 ms on the measured system. During the interval before a
replacement transform produces its first preview, the old paired image retains its
own transform and style; edits target the incoming preview. Blocked I/O and a single
oversized FFT are outside the measured latency claim.

## Next

Continue with the remaining signal levels and FFT information (#62). Leave #30 deferred.
The retained overview introduced by #74 is the source for display-only changes; style
changes must no longer invalidate the transform generation, even during refinement.
