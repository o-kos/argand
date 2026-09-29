# Issue #108: Analysis settings in the FFT hint

Resolves [#108](https://github.com/o-kos/argand/issues/108).
Builds on the #124 architecture ([completed plan](completed/124-standard-ui-architecture.md)):
the main window's borderless `Root`, the `hints::PinnedHint` contract from #129 and
standard gpui-component controls.

## Overview

Analysis settings live in two places today: a read-only pinned hint over the status
bar's FFT summary, and a separate settings window with OK and Cancel. Make the hint
the one settings surface: the same popover, looking like the hint, whose values are
standard controls that preview live. Remove the settings window.

Implementation class: **A**, declared before implementation: settings transactions,
analysis previews, focus and overlay behaviour, and the removal of a window.

Roles, set by the owner on 2026-09-29: implementer **Claude in session**; reviewer
**Codex** `gpt-6-sol`, reasoning effort `high`.

Native interaction is verified on Linux only, as for #124 (owner, 2026-09-29).

## Decisions (owner, 2026-09-29)

The Issue's description and the contract agreed in its 2026-09-23 comment differ;
the owner settled each difference.

- **Lifecycle is the #129 contract, unchanged.** Hovering the summary for 500 ms or a
  right click opens the hint; moving the pointer away does not close it. A click
  outside closes it and keeps the values (the click is consumed). Enter keeps the
  values and closes; Escape reverts to the values at opening and closes. Ctrl+,
  (Cmd+, on macOS), File → Settings and a left click on the summary open it pinned
  with the keyboard in it; Ctrl+, again closes it and keeps the values. F10, Ctrl+O
  and opening a file close it keeping the values. While it is open the plot is frozen
  behind the backdrop.
- **Rows look like the hint.** A compact heading `Analysis settings` on the left and a
  **Defaults** button on the right, then label/value rows in the existing metadata
  style. Each editable value is a standard `Select` or `NumberInput` with
  `appearance(false)`, marked by a dashed underline; a click opens its list or edits
  its number. No OK or Cancel.
- **Rows:** FFT size, Window, Overlap (%), Aggregation, Range mode, Range (dB; a
  number only when the mode is `Below measured peak`, otherwise the effective value,
  read-only and not underlined), Colour scheme. The low-signal advice and its
  `Use recommended` button (Ctrl+R) stay under the rows.
- **Editing.** A choice previews as soon as it is confirmed in its list. A number
  previews on Enter, blur or a stepper step; Enter on a valid number also closes the
  hint keeping the values. An invalid number keeps the hint open with the error shown
  under the rows. Enter inside an open list chooses the item and keeps the hint.
  Escape closes an open list first. Defaults applies the `argand.toml` values and
  keeps the hint open.
- **Escape restores everything.** The opening settings, time view and frequency view
  are captured when the hint opens and restored on a reverting close, as the settings
  window's Cancel did.
- **The settings window goes.** `settings_editor.rs`'s window, `settings_window`,
  `settings_backup` and its view and frequency backups, `finish_settings`,
  `cancel_settings_window` and `PinnedHint::set_enabled` are removed. The
  standard-input test in `settings_editor.rs` moves with the controls it proves.
- **One surface, one entity.** `PinnedHint` holds any view instead of a `Tooltip`;
  the analysis hint builds a fresh `Editor` entity (`settings_editor.rs`) each time it opens, synced
  from the shell while open (pending pictures, effective range, recommendation),
  and its layout does not change between hover and keyboard use.
- The yellow range item in the status bar still applies the recommendation directly
  and never opens the hint.

Verified before planning with a headless probe on the locked stack: a `Select` and a
`NumberInput`, both `appearance(false)`, inside a controlled `Popover` with
`track_focus` (the `hints::pinned` shape) choose and type; Enter in an open list
does not confirm the popover; the first Escape closes only the list; Escape from the
input reaches the hint's `Cancel`. GPUI 0.3.6 supports nested deferred draws, which
the #126 Select-in-Popover panic lacked. `NumberInput` keeps its steppers without
appearance, and `border_dashed` exists.

Discovered during implementation:

- A single-line input does not consume Enter, so the popover's `Confirm` closed the
  hint and dropped the editor before the input's `PressEnter` reached it. The editor
  now takes `Confirm` itself, previews the numbers and lets `Confirm` through only
  when they are usable, so Enter applies, closes and keeps, and an unusable number
  keeps the hint open with its error.
- The editor reads the shell while it is built, so Shell opens the hint through
  `window.defer` rather than inside its own update.
- Settings previewed in the open hint are not saved until a keeping close, so a
  crash mid-edit leaves the last kept settings in the session.
- `Settings::edited_numbers` now backs the editor's number preview, instead of a
  second copy of the overlap rule.
- Headless focus loss needs an active test window: an inactive one reports no focus
  path, so `InputEvent::Blur` never fires there.

## Rejected alternatives

- Closing on pointer leave, from the Issue's description: the owner kept the #129
  lifecycle, which the owner already verified natively.
- Keeping the settings window beside the hint: two settings surfaces with different
  transactions is what the Issue removes.
- A hand-written inline editor (PR #106): standard controls only.

## Implementation steps

- [x] Generalize `PinnedHint` to hold any view; keep its lifecycle and tests.
- [x] `Editor` view: heading with Defaults, rows with standard controls,
      advice, error; live preview, Enter/Escape/Defaults behaviour.
- [x] Capture and restore settings, time view and frequency view on a reverting close.
- [x] Route Ctrl+, / File → Settings / summary click to the pinned hint with focus in
      it; remove the settings window and every piece of its state.
- [x] Headless tests: choose and preview, number Enter applies and closes, invalid
      number keeps the hint with the error, Escape restores settings and views,
      click outside keeps, Defaults, Ctrl+, toggles, opening a file closes, the range
      item never opens the hint, the plot stays frozen while the hint is open.
- [x] Update AGENTS.md, README, CHANGELOG and the #124 inventory rows that name the
      settings window.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Native check on Linux (owner): hover and keyboard opening, every row, lists
      and numbers, invalid input, Enter, Escape restore, click outside, Defaults,
      Ctrl+R, Tab / Shift+Tab, both themes, a file opened while the hint is open.
- [ ] `ci/full` on Linux, Windows and macOS.

## Post-completion

- None planned.
