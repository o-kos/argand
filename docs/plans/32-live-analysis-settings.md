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
- Use a compact side panel opened from the window controls; keep controls grouped by
  transform and display. Reuse existing popup controls and typography.
- Store valid effective choices in a new backwards-readable session version; preserve
  configuration defaults and comments.

## Implementation

- [ ] Add validated effective settings and persistence.
- [ ] Add cached re-shading with the same range resolution as aspec.
- [ ] Integrate progressive transform replacement and retain old images on errors.
- [ ] Add the settings panel and recommended range display.
- [ ] Cover setting transitions, persistence and no-transform display updates.
- [ ] Validate native interaction and timing in both themes.
- [ ] Update architecture and user documentation.
- [ ] Complete independent review and move the plan before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after checks
- [ ] Colour and range edits update in under 30 ms without new transform requests
- [ ] FFT/window/overlap/reduction changes cancel obsolete requests and refine a preview
- [ ] Invalid settings preserve the last picture and report the problem
- [ ] Restart restores effective settings and leaves configuration bytes unchanged
- [ ] Dark/light native UI checks, including narrow windows
- [ ] Independent review with GPT-5.6 Sol at High effort

## Next

Continue with signal levels and FFT information (#62). Leave #30 deferred.
