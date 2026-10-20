# Issue #219: Type a selection's edges, in place on its badges and in a dialog

Resolves #219. Part of phase 5 (selection) in `IMPLEMENTATION_PLAN.md`.

Class A (more than five code files: a new parser module, the badge layout, the plot's
gestures, a new dialog, the Edit menu and its binding). Implementer: Claude in the session,
chosen by the owner. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Owner feedback

- The badge input was wider than its value and grew while typing, and showed more decimals than
  the badge. It is now a stock `Input` with `appearance(false)` in the badge's type and padding,
  as wide as the badge with room for the caret, holding the badge's own text.
- The dialog's labels wrapped, and its list of labelled rows did not read as a selection. It is
  now a drawing of the selection laid out as its badges are on the plot, the fields standing
  where the badges would, each axis's check box named with its unit.
- The badge input's text fell below its frame, which was the badge's height while the input
  keeps a taller line of its own. The input is extra small and keeps its own height, centred on
  the badge, and the frame holds it.
- The dialog's fields were see-through, so the dimension lines crossed their text, and a refused
  value was not marked while its field held the keyboard, because the focus ring covered the
  border. The fields stand on the dialog's background with their text centred, and a refused
  field turns its focus ring off (`focus_bordered(false)`) to show the danger border. Checked by
  running the release build on a second monitor.
- Round of owner requests (2026-10-20). The dialog cannot be dragged with the stock `Dialog`;
  instead, as the owner chose, it opens where it covers least of the selection. Up and Down step
  a field by its last shown digit, ten with Shift. The check boxes became segments, Time, Band and
  Both, on stock buttons. Edge badges on the plot did not show which edge they name. Extension
  lines along the edges were too faint, and vanished where an edge is the plot's own; of three
  drawn variants the owner chose a small pointer in the badge's ink reaching the edge's corner. The dialog's type follows the app's small type. Checked in the
  release build on a second monitor with keyboard input sent to the window.
- The dialog itself did not suit (2026-10-20): wherever it opened it covered something useful, its
  segments did not belong in it, and OK and Cancel came from another world. At the owner's
  suggestion it is now a panel like the analysis settings, a `PinnedHint` under three title-bar
  segments right of Grid. The owner chose: a segment selects its kind and opens the panel, the lit
  one toggles it; the plot stays live under the panel, which follows it. With no check boxes and
  no segments in the panel, a value typed for an axis not selected selects it, which also keeps
  the panel usable from the keyboard alone.
- Next round (2026-10-20): it was unclear when the panel goes away, so it now closes as the
  analysis hint does, the plot frozen under it. Ctrl+E with nothing selected opens it on time and
  band from the visible range. The lit segment was dimmer than the orientation's, because only
  that one was marked `selected`. The pointers became symmetric triangles rather than a
  tooltip's tail. The selection tint does not show on a bright spectrogram; four variants were
  drawn on a real screenshot for the owner to choose from. The owner first chose D, an outline over a white veil, then C: the
  picture outside the selection is dimmed. Dimming alone left a selection on a dark picture
  unseen, so the tint inside stays as well. The pointers of two edges meeting at a corner merged
  into one odd shape, so each is now a plain triangle running from its edge into its badge.

## Review

- Round 1 (`gpt-6.1-sol` high): three P2 findings, all accepted. An open badge input read its
  text in the units the rulers showed at Enter, so a frequency zoom between typing and Enter
  changed the unit, and it now keeps the `Scale` it was written in. The dialog could apply or
  restore a selection in another document opened under it with Ctrl+O, and it now selects only
  in the document it opened on and closes when another one opens. Sample indexes were read
  through `f64`, losing precision past 2^53 and rounding past the end of such a capture, and
  whole indexes are now read as integers and clamped in integers. Each has a regression test.
  No findings declined.
- Round 2: the round 1 fixes confirmed, two P2 findings, both accepted. Ctrl+O over the dialog
  focused the plot and left the dialog open behind it when the chooser was cancelled, so
  Escape cleared the plot's selection; choosing a file now closes the dialog keeping what it
  applied, as it closes the analysis hint. Leaving the window closed a badge input without
  giving the keyboard to anything; a blur that hands focus to no other control now returns it
  to the plot. Each has a regression test. No findings declined.
- Round 3: the round 2 fixes confirmed, one P2 finding, accepted. Opened from the application
  menu with nothing selected, the dialog lost the keyboard to the plot, because the menu's
  dismissal arrives after its action and focuses the plot, so Escape reached the plot. The
  dialog now dismisses the menu before it opens, as Settings does, with a regression test that
  goes through the menu. No findings declined, and nothing else substantive remained.

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
- **Texts to type over.** A field shows its value as the badges show it, both written by one
  `Scale`: time to one pixel of the visible time, frequency to one row. A field whose text is
  unchanged is never parsed again, so that rounding moves no edge, and a typed value carries
  the precision it is typed with. (First written at the precision of one sample, changed at
  the owner's request so the input and the dialog show what the badge shows.)
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
- **The panel.** `selection_panel.rs` holds a `SelectionPanel` shown in a `PinnedHint` under
  the title bar's Time, Band and Both segments, without the analysis hint's backdrop, so the
  plot stays live and the panel follows it. It applies each read value at once; Enter keeps the
  values and closes once every changed field reads, Escape restores the opening selection.
  (It replaced a modal dialog, see "Owner feedback".)
- **Menu and key.** `EditSelection` is bound to Ctrl+E (Cmd+E) in the `Plot` context and sits in
  the Edit menu as Selection panel; both toggle the panel.

## Rejected alternatives

- Painting an editable text field into the canvas: hand-written text editing, which the
  conventions forbid.
- Laying transparent, occluding hit areas over the badges: they would take the crosshair,
  the wheel and middle-button pans as well; the plot's own press already decides gestures
  once.
- Applying only on OK: the owner chose live preview.
- A modal dialog, whatever its placement: it covered something useful wherever it stood, and its
  own buttons and segments did not belong to the app's surfaces.

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
- [x] `SelectionDialog` with live preview, OK and Cancel, Edit → Selection… and Ctrl+E, and
      headless tests.
- [x] Update `AGENTS.md` and `CHANGELOG.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Owner check in the release binary: edit each badge in both orientations and the three
      ruler modes, make a time, band and rectangle selection from the dialog with nothing
      selected, Cancel restores, a bad value is marked.

## Post-completion

None.
