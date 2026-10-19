# Issue #219: Type a selection's edges, in place on its badges and in a dialog

Resolves #219. Part of phase 5 (selection) in `IMPLEMENTATION_PLAN.md`.

Class A (more than five code files: a new parser module, the badge layout, the plot's
gestures, a new dialog, the Edit menu and its binding). Implementer: Claude in the session,
chosen by the owner. Reviewer: `gpt-6.1-sol` high, proposed by the class table and to be
agreed with the owner before the first round.

## Overview

A selection's edges can only be set with the mouse (#183). This adds two ways to type them:

1. a double click on a selection badge turns it into a standard input holding its value;
2. Edit → Selection… (Ctrl+E, Cmd+E on macOS) opens a dialog with every value of the
   selection, which also makes a new selection when nothing is selected.

The result is an ordinary selection, the same as one dragged out, never an edit.

## Owner decisions (2026-10-19)

- The dialog's shortcut is Ctrl+E (Cmd+E on macOS).
- Each axis in the dialog has a check box, Time and Band. An unchecked axis is not
  selected; its fields hold the visible range and are disabled until the box is checked.
- The dialog applies live: every accepted field (Enter or leaving it) and every check box
  moves the selection on the plot at once. OK keeps it, Cancel and Escape restore the
  selection the dialog opened with.
- The band part is offered for a capture whose reference frequency is unknown (0), in
  baseband frequencies, as the ruler and the badges already show them.

## Context

- `argand_core::Selection { time, band }`; `Shell::selection` and `Shell::band` keep it;
  `Shell::select` takes a gesture's selection and clamps its band to `Shell::capture_band`.
  `PlotIntent::Select` is the plot's way to it.
- `time_ruler::Mode::counters` formats a span's start, end and length: clock, seconds or
  `#index` (start, last sample, count), falling back to sample indexes where seconds cannot
  tell samples apart. `Shell::band_texts` formats a band in the frequency ruler's unit
  (`axes::frequency_unit`).
- `selection_badges.rs` places and paints the badges inside the plot canvas's paint phase.
  Nothing outside the canvas knows where they are, and they take no input.
- `numbers::parse` reads a localized number as the settings inputs do;
  `numbers::Numbers::canonical` turns locale digits, grouping and decimal mark into ASCII.
- gpui-component 0.6.6: a single-line `InputState` propagates Enter (as `Confirm` inside a
  dialog) and emits `InputEvent::PressEnter`; its Escape propagates; `InputEvent::Blur`
  reports leaving it. `Dialog` (`window.open_dialog`) binds Enter to `on_ok` and Escape to
  `on_cancel` in its own context; `ask_unsaved` already uses it through `Root`.

## Decisions

- **A toolkit-neutral entry module.** `selection_entry.rs` owns the six fields
  (`Field::{TimeStart, TimeEnd, TimeLength, BandLow, BandHigh, BandWidth}`), their texts
  and their parsing, with no GPUI type, so it is tested without a window. A `Scale`
  carries the ruler mode, sample rate, sample count, the frequency unit's divisor and
  decimals and the capture's band.
- **Texts to type over.** A field shows its value at the precision that tells every sample
  apart (`counters` with no pixel limit), so the text read back gives the same sample. A
  band shows its edges in the ruler's unit to the finer of one hertz and one row. A field
  whose text is unchanged is never parsed again, so an edge made by a drag keeps its exact
  value when another field is typed.
- **Parsing.** Every number is read through `numbers::parse` (locale digits, grouping and
  decimal mark). Time in clock mode takes one to three colon-separated fields
  (`h:mm:ss.fff`, `m:ss.fff` or seconds); seconds mode takes a number with an optional
  trailing `s`; a leading `#` names a sample index in every mode (the end field names the
  last sample, as the badges do, and the length counts samples), which is also what the
  clock and seconds badges show where seconds cannot tell samples apart. A plain number in
  samples mode is an index. Frequency is a number in the ruler's unit, an optional trailing
  unit name accepted.
- **Applying.** Times are rounded to whole samples. A start or end moves that edge; the
  edges are taken in either order, as dragging one past the other does; equal edges are
  refused. A length moves the end, keeping the start. A band's low or high edge moves that
  edge, its width moves the high edge, keeping the low one. Values are held to the
  capture: time to `0..=len`, frequency to the capture's band, a length or width as far as
  the capture reaches. A text that does not parse, or gives no extent, marks its field
  with the input's error state and changes nothing.
- **Badges take a left press.** The badge layout is split from painting
  (`SelectionBadges::layout`), and the canvas's prepaint hands each shown badge's field
  and rectangle to the `PlotView`, as it hands over the plot geometry. A left press on a
  badge (without Space) starts no selection gesture; a double click there opens the
  in-place input. Other buttons, the wheel, Space+drag and the crosshair are unchanged.
  The pointer is an arrow over a badge.
- **In-place input.** The `PlotView` owns a `BadgeEntry` (field, `InputState`) drawn as a
  sibling of the plot surface, so the `Plot` bindings never match while it holds focus,
  over the badge's rectangle and at least wide enough for its text, inside the plot.
  The badge under it is not painted. Enter applies (a parse failure keeps it open and
  marked), Escape (an outer `BadgeEntry` key context, since the input propagates it) and
  blur close it unchanged; focus returns to the plot. Any change of the selection from
  elsewhere, a gesture or a file opening closes it. The plot emits the parsed result as
  `PlotIntent::Select`, so it takes the shell's ordinary path.
- **The dialog.** `selection_dialog.rs` holds a `SelectionDialog` entity: two standard
  `Checkbox`es and six standard `Input`s in a grid with each axis's unit after it, in a
  stock `Dialog` with OK and Cancel. It opens on the time start (or the first enabled
  field) and remembers the selection it opened with. `on_ok` first applies the field
  holding focus, and keeps the dialog open if it does not parse; `on_cancel` restores the
  opening selection. Each applied value goes through `Shell::select` and every field is
  refreshed from the result except one holding an error. The dialog is offered whenever a
  capture is described, and is disabled while another dialog is open.
- **Menu and key.** `EditSelection` is bound to Ctrl+E (Cmd+E) in the `Plot` context and
  sits in the Edit menu after Delete, as `Selection…`.

## Rejected alternatives

- Painting an editable text field into the canvas: hand-written text editing, which the
  conventions forbid.
- Laying transparent, occluding hit areas over the badges: they would take the crosshair,
  the wheel and middle-button pans as well; the plot's own press already decides gestures
  once.
- Formatting the inputs at the badges' pixel precision: the text read back would move an
  edge the user did not touch.
- Applying only on OK: the owner chose live preview.

## Custom control audit

No new custom control. The in-place editor and the dialog fields are the stock `Input`,
the boxes the stock `Checkbox`, the dialog the stock `Dialog`. Positioning an `Input` over
a domain canvas is composition, as the zoom pairs over the spectrogram are.

## Implementation steps

- [x] `selection_entry.rs` with tests: field texts, time parsing in the three modes and with
      `#`, frequency parsing, applying each field with clamping, swapping and refusals, a
      locale with a comma decimal mark.
- [x] Split badge layout from painting, tag badges with their fields, hand their
      rectangles to the `PlotView`; left press on a badge starts nothing, arrow cursor.
- [x] In-place `BadgeEntry` on double click with Enter, Escape and blur, and headless tests.
- [ ] `SelectionDialog` with live preview, OK and Cancel, Edit → Selection… and Ctrl+E, and
      headless tests.
- [ ] Update `AGENTS.md` and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Owner check in the release binary: edit each badge in both orientations and the three
      ruler modes, make a time, band and rectangle selection from the dialog with nothing
      selected, Cancel restores, a bad value is marked.

## Post-completion

None.
