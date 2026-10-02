# Issues #171, #168, #170, #169: The settings hint's balloon, window edges and keyboard

Resolves #171, #168, #170 and #169.

## Overview

Four defects the owner found in the analysis settings hint after #108 and #167:

- **#171** The range warning balloon runs past the window's right edge in a narrow window.
- **#168** An open list of the hint is drawn beneath the balloon.
- **#170** While the settings hint or the application menu is open, the window cannot be
  resized by its edges.
- **#169** The hint cannot be operated from the keyboard: after Ctrl+, nothing is focused,
  Tab moves nowhere and Enter closes the hint.

Class A (more than roughly 400 lines over overlays, focus and the frame). Implementer:
Claude in session. Reviewer: Codex `gpt-6.1-sol` high.

## Context

- The hint is a `Popover` holding `settings_editor::Editor`; a new editor is built on every
  opening. The balloon is drawn by `Editor::range_readout`, absolutely placed to the right of
  the Range value at a fixed width, deferred at `POPUP_PRIORITY + 1`. Select lists are
  deferred at `POPUP_PRIORITY`.
- `SelectState` in gpui-component 0.6.6 keeps its open flag private. Its `Focusable` answers
  the list's handle while open and its own handle while closed, so an editor that kept the
  closed handle can tell an open list from a closed one.
- The closed `Select` has no key bindings at all and no public way to open its list. Its focus
  handle is not a tab stop; `InputState`'s is.
- `Shell` swallows `FocusNext` and `FocusPrevious` while the hint is open, because the plot
  surface is a tab stop and window-wide traversal would leave the hint.
- `hints::backdrop` covers the window below the title bar, and the application menu backdrop
  covers the whole window. Both lie above `chrome::Frame`'s resize regions, which extend from
  the window edge through the shadow inset and 6 pixels into the visible frame.

## Decisions

- **#171 placement (owner).** The balloon stands right of the hint and wraps its text into the
  room left before the window's edge. When that room is under about 180 pixels it stands left
  of the hint instead, centred on the Range row, with its pointer turned right towards the row.
  The editor measures its own bounds and the Range value's bounds each frame and places the
  balloon from the last measurement, so it is not drawn before the first one.
- **Balloon close (owner).** An × on the balloon hides it until the hint closes. The value keeps
  its warning colour, its click and an ordinary hover hint with the same text.
- **#168.** The balloon is hidden while any list of the hint is open.
- **#170.** `Frame` reports the rectangle its resize regions leave free. Both backdrops cover
  only that rectangle, and a capture-phase observer closes the hint or the menu on a press
  outside it, so the press that starts a resize also closes the overlay; the hint keeps its
  values.
- **#169 keyboard** (see the corrections below: the stock list keys are kept).
  - Ctrl+, and File → Settings put the keyboard on FFT size.
  - Tab and Shift+Tab cycle through the hint's own stops in order: FFT size, Window, Overlap,
    Aggregation, Range mode, Range (while it is a number), Colour scheme, Reset to defaults.
    The editor handles the traversal itself, so it never leaves the hint.
  - Up and Down on a focused list take the previous or next value with a live preview, through
    `SelectState::set_selected_index`; the list does not open. An open list keeps its own arrows.
  - Up and Down on a focused number step it; digits type into it as before.
  - Enter on Reset to defaults resets, on a number applies it, and elsewhere closes the hint
    keeping the values. Escape still reverts.
  - The focused value is marked: its underline turns solid in the accent colour, and Reset to
    defaults gets an accent ring.

## Corrections during implementation

- **The stock `Select` does take the keyboard.** gpui-base 0.6.6's `Select` root binds
  Up, Down, Enter and Escape in its `Select` context and is a tab stop; the owner's choice
  of arrows that change a closed list's value rested on my wrong reading that it had no
  key handling. With the keyboard on a list, the stock behaviour opens it on Enter, Up or
  Down, its arrows walk it and Enter chooses, which is what #169 asked for, so the hint
  keeps the stock behaviour and adds nothing for lists. Reported to the owner.
- Nothing had the keyboard after Ctrl+, apart from the popover itself, which is why Tab
  seemed to do nothing and Enter closed the hint.
- **Tab traversal** comes from the window's tab order, bounded to the hint: `Shell` binds
  Tab to its own `FocusNext`, which it swallowed while the hint was open, so the editor
  takes `FocusNext` and `FocusPrevious` first and skips stops outside its focus scope.
- **No balloon without room on either side**: in a window too narrow for both, a balloon
  wrapped into a sliver ran off the panel; the value's hover hint carries the words instead.

## Rejected alternatives

- Opening a list from the keyboard by a synthetic click on the closed field: it depends on
  private toolkit structure; the owner chose arrows on the closed field.
- Shifting the whole hint left to make room for the balloon: the hint would no longer stand over
  the FFT summary.
- Letting window-wide tab traversal run with the hint open: the plot surface is a tab stop.

## Implementation steps

- [x] #170: `Frame` free rectangle, both backdrops cover it, presses outside close the overlay.
- [x] #171 and #168: measured placement, wrapping, left side, × and hidden while a list is open.
- [x] #169: initial focus, traversal, arrows on lists and numbers, Enter on Reset, focus marks.
- [x] Headless tests for each of the above.
- [x] Screenshots in the running application: the balloon at four window widths (right,
      wrapped, hidden), an open list without the balloon, the × and the hover hint after it,
      every Tab stop with its mark, a list chosen with Enter and Down, a number stepped with
      Up, and Enter on Reset to defaults.
- ⚠️ A resize from each edge with the hint and the menu open: the nested X server the
  bench runs in has no window manager, so a resize cannot start there. Headless tests
  cover the free area and a press on an edge; the owner checks it natively.
- ➕ The left placement is covered by a unit test; no window on the bench puts the hint
  far enough right to need it.
- [x] Update `AGENTS.md`, `README.md`, `CHANGELOG.md` and `docs/ui/124-inventory.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The native checks above on Linux.

## Post-completion

- Close #171, #168, #170 and #169 through the merge.
