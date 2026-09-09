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
  analysis summary with an interactive hover hint and a separate settings window.
  Group transform and display controls.
  The file hint includes sample count, file bytes, and original sample extrema (I/Q separately).
- Highlight low-level signals in the lower half of the absolute scale in yellow;
  offer the aspec recommended range in the hover hint and settings window.
- Include #61 and the range-advice portion of #62 in this owner-approved scope. Keep
  the remaining signal-level/FFT diagnostic panels in #62.
- Store valid effective choices in a new backwards-readable session version; preserve
  configuration defaults and comments.

## Implementation

- [x] Add validated effective settings and persistence.
- [x] Add cached re-shading with the same range resolution as aspec.
- [x] Integrate progressive transform replacement and retain old images on errors.
- [x] Add the combined file status/hint including exact counts, file bytes and sample extrema.
- [x] Add the analysis hover hint, settings window and clickable recommended range.
- [x] Cover setting transitions, persistence and no-transform display updates.
- [x] Validate native interaction and timing in both themes.
- [x] Update architecture and user documentation.
- [x] Complete independent review and move the plan before final review.

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
- [x] Independent review with GPT-5.6 Sol at High effort

## Evidence and timing boundary

See [native validation and timing](../../performance/32-status-settings.md). Cached edits
presented in 24.5–27.7 ms on the measured system. During the interval before a
replacement transform produces its first preview, the old paired image retains its
own transform and style; edits target the incoming preview. Blocked I/O and a single
oversized FFT are outside the measured latency claim.

## Independent review

Three rounds completed; the final round had no substantive findings. Accepted fixes
cover sparse-preview refresh and cancellable delivery, whole-sample overlap aliases,
configuration FFT limits, and a queued style snapshot superseded by resize. The last
case has a deterministic regression test that failed before the fix.

The proposal to re-shade the superseded transform before its replacement preview was
declined: the retained paired picture stays associated with its own transform/style
and new style choices target the incoming preview. The reviewer challenged and
accepted this rationale under the explicit timing boundary above.

## Next

Continue with the remaining signal levels and FFT information (#62). Leave #30 deferred.
The retained overview introduced by #74 is the source for display-only changes; style
changes must no longer invalidate the transform generation, even during refinement.

## Owner feedback: status and keyboard interaction

- [x] Replace the file hint with aligned labels and values.
- [x] Show FFT details on hover, keep normal status text muted, and brighten it on hover.
- [x] Warn about a low-level signal in the lower half of the absolute scale; keep the aspec recommended value.
- [x] Replace expanding button groups with standard selects and numeric inputs in a dedicated settings window.
- [x] Verify opening, every control, recommendation and closing using only the keyboard.
- [x] Repeat native dark/light checks, local gate, release build and independent review.

The revised controls and keyboard interaction are documented in
[owner-feedback validation](../../performance/32-status-settings-ux.md). Independent
review identified a rejected select value that did not reflect effective settings;
restoring the selects on rejection fixes it. No findings were declined in this revision,
and the final round was clean.
