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

## Owner feedback 1 (2026-09-29)

The first native check found the surface unusable, and the implementation had not
been run in a real window before it was shown. From here each change is checked in
the running application first: the owner allowed launching it on the second monitor
through XWayland with an isolated configuration and session, driven by XTEST and
captured per window.

- [x] **The Window list was cut off.** A frameless `Select` took the width of its
      text and its list inherited it. The value column now has a fixed width, every
      control fills it, and lists are wider than the column. Verified: Hann,
      Hamming, Blackman-Harris and Rectangular are shown whole.
- [x] **Names are title-cased.** Window and colour-scheme names show as Hann,
      Blackman-Harris, Rectangular, Oceanic, including the status summary; the CLI
      and configuration keep their lower-case names, which parse back unchanged.
- [x] **Defaults was not discoverable.** It is now an outlined `Reset to defaults`
      button in its own row below the values (owner's choice).
- [x] **Overlap and Range showed only the steppers and the unit.** `NumberInput`
      grows with `flex_1` and sat in a cell without a width, so its text collapsed to
      nothing. With the fixed value column the number shows (verified: 75 %, 40 dB).
- [x] **A click on the summary closed the hint and it came back.** Not the click: the
      summary is under the pointer when the backdrop that took the press disappears,
      so it is hovered again and the 500 ms hover opening fired. A hover within
      400 ms of a close no longer opens the hint; leaving and returning does. The
      summary also opens the hint on press rather than click. Verified both ways.
- [x] **A stepper click then a drag recalculated the spectrum.** Not reproduced as a
      second request: the debug log shows one `analysis settings requested` for the
      step and none for the drag. The step's analysis takes about 0.9 s and refines
      left to right, which is still running when the drag starts.
- [ ] **The picture jerks after a change.** Frames captured every few hundred
      milliseconds show the old picture replaced at once by a sparse preview
      (`refined_columns: 0`), then refined left to right. This is the existing
      progressive analysis every settings change has used; the owner checks it
      against `main` before deciding.

## Owner feedback 2 (2026-09-30)

- [x] **Values take their own width**, right-aligned in the panel, instead of one
      shared column width. Lists open 220 pixels wide whatever the value's width.
- [x] **Steppers act on the press.** Overlap and Range are now a standard `Input`
      between two standard `Button`s rather than `NumberInput`, whose steppers step
      on click only and offer no press hook.
- [x] **Steppers repeat while held** (owner-approved custom press handling): one step
      at the press, then every 80 ms after 400 ms, until the button is released or
      the pointer leaves it. A step that yields an unusable value stops the repeat.
      Verified in the running window: 14 steps in 1.5 s, none after release.
- [x] **Enter in Overlap or Range applies the number and keeps the hint.** Enter
      elsewhere in the hint still closes it keeping the values.
- [x] **The picture jerked after every change.** The owner's recording showed four
      states in a row: a 128-frame blocky preview, a denser preview with a brighter
      background, a left-to-right refinement brighter than the result, then the
      result. A settings change restarted the first-analysis sequence. Every analysis
      after a file's first is now a final-only replacement, as navigation already
      was: the shown picture stays, the status bar shows progress, and the new
      picture replaces it once (owner's choice). A test fails without the change.
      On the test bench the step's analysis took 0.85 s against 1.3 s for the
      file's first analysis, with no intermediate snapshot in the log.

## Owner feedback 3 (2026-09-30)

- [x] **Lists moved while the keyboard walked them.** A list was as wide as its
      current name and right-aligned, so every name of another length shifted it.
      Each list now has the fixed width of its widest name. A headless test walks
      the Window list with Down and Enter and compares the bounds; it fails on
      content-sized lists (95 px against 70 px).
- [x] **The low-level warning was off-centre and left an empty line when absent.**
      The advice and its button left the hint. While the hint is open and the range
      is warned, the range item's own hint, with its Ctrl+R keycap, is drawn beside
      the settings hint above the status bar. It cannot sit directly above the
      range item, because the settings hint covers that place.
- [x] **No reserved status line.** The pending state sits beside the title and an
      error takes the title's place, in one line of fixed height.

## Owner feedback 4 (2026-09-30)

- [x] **Values looked wide again.** The fixed-width lists of feedback 3 underlined
      their whole width. What moved under the keyboard was the popup, anchored to
      the left edge of a content-sized list whose shown value follows the walk. The
      list keeps the fixed width, so the popup stays, but shows its value at the
      right beside the chevron and is underlined only under that value.
- [x] **The balloon did not look like one.** It has a leader in the warning colour,
      down from its body and along the status bar to an arrowhead at the sign, and
      its body ends level with the settings hint.
- [x] **While the balloon is shown the item reads `110 dB ⚠`**, sign last, so the
      leader ends at the sign.
- [x] **Superseded by feedback 5**: the leader and the reordered status item are gone.
- [x] Checked in the running application, in a private nested X server that needs
      no focus from the desktop: the open hint with the balloon, the FFT list
      walked with Down, an unusable overlap and the pending state.

## Owner feedback 5 (2026-09-30)

- [x] **The status bar's range item is not to be touched.** It is back to what it
      was before this issue's feedback, with the hint open or closed.
- [x] **The warning lives in the hint's Range row.** The row reads `110 dB ⚠` in
      the warning colour.
- [x] **A balloon is a bubble with a pointer.** It stands to the right of the hint,
      centred on the Range row, edged in the warning colour, with the warning text
      and the Ctrl+R keycap, and its pointer's tip at the sign. Checked in the
      running application.

## Owner feedback 6 (2026-09-30)

- [x] **A click applies the recommendation.** The warned value is underlined like
      every editable value, in the warning colour, and a click on it or on the
      balloon does what Ctrl+R does: the balloon goes and the hint stays. The
      popover dismissed itself on any press outside its panel, which the balloon
      is, so the hint's backdrop now closes it instead (`overlay_closable(false)`).
- [x] **No rule above Reset to defaults.**
- [x] Owner's decision: the status bar's range item stays visible while the
      balloon is shown.
- [x] Checked in the running application: both clicks, Ctrl+R back, a press
      outside closing the hint with 40 dB kept, and reopening.

## Owner feedback 7 (2026-09-30)

- [x] **The close button needed two clicks while the hint was open.** The backdrop
      covered the title bar and took the first press. It now starts below the title
      bar; a press in the title bar closes the hint, keeping its values, and still
      reaches the control under it. Checked in the running application: a press on
      the title closes the hint, and one press on the close button ends the process.
- [ ] The colour of the stepper buttons is left for a colours issue (owner's call).

## Review round 1

Codex `gpt-6-sol` high reviewed the revision the owner first saw; its findings were
arbitrated against owner feedback 1.

- **Accepted: Ctrl+, and File → Settings opened an invisible hint without a
  document**, whose backdrop then covered the start page. Without a document
  `edit_analysis` does nothing and the Settings row is disabled; a test covers it.
- **Accepted differently: a click on the summary closed the open hint.** After owner
  feedback 1 the click is a toggle that closes the open hint and does not bring it
  back; that matches a button that opens a popover and the owner's complaint about it
  reappearing.
- **Accepted: Reset to defaults left a refused number in its field** when the
  settings already equalled the configuration. Applying equal settings now resyncs
  the fields; a test covers it.
- **Accepted: the hint moved when the advice appeared or went.** The status area
  keeps the advice's height; verified in the running window.
- **Accepted with the owner's agreement: holding a stepper repeats.** The standard
  `NumberInput` steps on click only, so the steppers are standard `Button`s with a
  press handler and a repeat task (owner feedback 2).
- **Accepted: tests claimed more than they proved.** The choice test is named for
  what it checks, the Escape test also checks the frequency view, and a preview is
  asserted not to be saved.
- **Accepted: stale documentation** in README (OK and session version 5), AGENTS.md
  (#108 as future work) and the inventory (a renamed test).

## Rejected alternatives

- Closing on pointer leave, from the Issue's description: the owner kept the #129
  lifecycle, which the owner already verified natively.
- Keeping the settings window beside the hint: two settings surfaces with different
  transactions is what the Issue removes.
- A hand-written inline editor (PR #106): standard controls only.

## Implementation steps

- [x] Generalize `PinnedHint` to hold any view; keep its lifecycle and tests.
- [x] `Editor` view: heading, Reset to defaults, rows with standard controls,
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
