# Issue #102: Apply range advice from the status bar

Resolves [#102](https://github.com/o-kos/argand/issues/102).

## Overview

Clicking the yellow status-bar dB value applies the same recommendation as
Ctrl+R / Cmd+R, without opening the settings window.

## Context

`settings_ui.rs` paints the range inside the analysis-settings button.
`UseRecommendedRange` already owns shortcut/action routing and the existing
recommendation path preserves settings-preview and availability checks.

## Decisions

- Dispatch the existing action from the warned range only.
- Stop click propagation so the containing button does not open settings.
- Preserve settings access from the rest of the summary and from an unwarned range.

## Rejected alternatives

- Do not duplicate recommendation calculations or settings application logic.

## Implementation steps

- [ ] Add the warned-range click target and preserve parent-button behavior.
- [ ] Update the changelog and interaction documentation.
- [ ] Verify click/shortcut behavior and settings access in a native window.
- [ ] Complete local checks and external review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after standard checks
- [ ] Native verification on a low-amplitude capture: warning click applies advice
  without opening settings, keyboard advice remains available, and the rest of
  the summary still opens settings. Confirm preview/cancel behavior.
- [ ] External review returns no substantive findings.

## Post-completion

Squash-merge after owner acceptance and successful full CI.
