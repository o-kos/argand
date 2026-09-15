# Issue #102: Apply range advice from the status bar

Resolves [#102](https://github.com/o-kos/argand/issues/102) and
[#108](https://github.com/o-kos/argand/issues/108).

## Overview

Clicking the yellow status-bar dB value applies the same recommendation as
Ctrl+R / Cmd+R, without opening the settings popup. The same status control
opens the complete analysis editor as a transient popup instead of a separate
application window.

## Context

`settings_ui.rs` paints the range inside the analysis-settings button.
`UseRecommendedRange` already owns shortcut/action routing and the existing
recommendation path preserves settings-preview and availability checks.
The existing editor already owns live preview, validation and transactional
OK/Cancel behavior, but currently renders in an independent normal window.

## Decisions

- Dispatch the existing action from the warned range only.
- Stop click propagation so the containing button does not open settings.
- Keep the retained range warning visible until the replacement display arrives,
  so applying advice changes the warning and value as one visible update.
- Preserve settings access from the rest of the summary and from an unwarned range.
- Render the existing editor in a compact transient popup surface opened from
  the status control, without normal window chrome. Place it above the control
  where the window backend accepts client positioning; otherwise retain the
  compositor's transient placement.
- Treat clicking or focusing outside the popup as Cancel, restoring its opening
  settings and view. Keep explicit OK, Cancel, Reset and keyboard behavior.

## Rejected alternatives

- Do not duplicate recommendation calculations or settings application logic.
- Do not keep a modeless normal window or add a modal overlay: owner feedback
  selected the status popup as the editing surface.
- Do not wrap the main shell in `gpui_component::Root`: that would add a second
  frame. Standard inputs require their own Root-backed surface, while Wayland
  does not expose absolute placement for such a surface.

## Implementation steps

- [x] Add the warned-range click target and preserve parent-button behavior.
- [x] Move the analysis editor into the status popup and preserve transactional behavior.
- [x] Update the changelog and interaction documentation.
- [ ] Verify click/shortcut behavior and popup editing in a native window.
- [x] Complete local checks and external review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after standard checks
- [ ] Native verification on a low-amplitude capture: warning click applies advice
  without opening settings, keyboard advice remains available, and the rest of
  the summary opens the editor popup. Confirm preview/OK/Cancel, click-out,
  focus-loss and dropdown behavior.
- [x] External review returns no substantive findings.

## Post-completion

Squash-merge after owner acceptance and successful full CI.
