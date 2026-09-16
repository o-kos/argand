# Issue #102: Apply range advice from the status bar

Resolves [#102](https://github.com/o-kos/argand/issues/102) and
[#108](https://github.com/o-kos/argand/issues/108).

## Overview

Clicking the yellow status-bar dB value applies the same recommendation as
Ctrl+R / Cmd+R. Hovering the analysis summary reveals the same information as
an interactive hint; only its dashed-underlined values advertise editing, and
each value opens its own compact choice list or numeric stepper.

## Context

`settings_ui.rs` paints the range inside the analysis-settings button.
`UseRecommendedRange` already owns shortcut/action routing and the existing
recommendation path preserves settings-preview and availability checks.
The first implementation reused the dialog editor and rendered it in an
independent top-level window. A second attempt moved that form into a popover
but retained dialog-only state and OK/Cancel actions.

## Decisions

- Dispatch the existing action from the warned range only.
- Stop click propagation so the containing button does not open settings.
- Keep the retained range warning visible until the replacement display arrives,
  so applying advice changes the warning and value as one visible update.
- Show one retained editor in one anchored Popover on hover, without requiring
  a click. Ctrl+, / Cmd+, or the first edit pins that same entity; pinning must
  not reparent it or change its geometry, position or typography.
- Keep the hint free of any separate window or `Root` dependency.
- Use compact inline choice lists and numeric editing controls because the
  standard select and input components require a `Root`.
- Render ordinary rows like metadata and mark only editable values with a
  dashed underline. Reveal a local choice list or numeric stepper after click.
- Keep a compact `Analysis settings` heading inside the hint and align the
  lightweight reset command to the right.
- Keep the hover-to-pinned transition and numeric value width geometrically
  stable. Do not add a separate pending row; validation errors reuse the
  existing footer. Repeat stepper changes while a button remains pressed.
- Apply valid edits as previews and pin the hint after its first interaction so
  analysis updates cannot dismiss it. Enter accepts and persists the complete
  edit; Escape restores every opening value. Click-out and focus-out accept.
- Keep reset as a lightweight menu command and remove OK, Cancel and
  shell-owned transactional backup state.
- Keep the analysis status text muted on hover, matching the adjacent file
  status item; only the yellow range warning changes its colour.

## Rejected alternatives

- Do not duplicate recommendation calculations or settings application logic.
- Do not keep a modeless normal window, transient top-level window or modal
  overlay: owner feedback selected an interactive status hint as the editing
  surface.
- Do not move the old dialog form into the popover. A different container does
  not change a dialog interaction into an inspector interaction.
- Do not wrap the main shell in `gpui_component::Root`: that would add a second
  frame. Reusing Root-dependent controls is not worth changing the requested
  interaction model.

## Implementation steps

- [x] Add the warned-range click target and preserve parent-button behavior.
- [x] Replace the analysis dialog with an interactive hover hint and local controls.
- [x] Update the changelog and interaction documentation.
- [x] Verify hover, click/shortcut behavior and local editing in a native window.
- [x] Complete local checks and external review.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after standard checks
- [x] Native verification on a low-amplitude capture: warning click applies advice
  without opening settings, keyboard advice remains available, and hovering the
  summary opens the interactive hint. Confirm dashed editable values, local
  controls, retained previews, Enter acceptance, Escape rollback, reset,
  click-out, focus-loss, choice-list behavior, stable stepper geometry and
  press-and-hold repetition. Confirm the compact heading, right-aligned reset
  command and stable status-bar colour.
- [x] External review returns no substantive findings.

## Post-completion

Squash-merge after owner acceptance and successful full CI.
