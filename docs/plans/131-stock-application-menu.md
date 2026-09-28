# Issue #131: Stock PopupMenu for the application menu

Resolves [#131](https://github.com/o-kos/argand/issues/131).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 5 (standard-control
replacement), inventory rows "Main cascading menu" and "Ruler context menu".
Predecessor [#130](completed/130-standard-buttons.md), merged in PR #157.

## Overview

The File / View application menu is a hand-built cascade: a toolkit-neutral
navigation model (`app_menu.rs` `Menu`, `Effect`, `place`), `div` panels and rows,
our own keyboard handler, width measurement, placement and a full-window overlay
(`app_menu_ui.rs`). Replace it with the stock gpui-component `PopupMenu` and its
submenus, and delete the custom navigation, placement and rendering.

The owner decided on 2026-09-28 to accept the stock keyboard contract where it
differs from the accepted #91 behaviour, instead of keeping a custom cascade for one-level
Escape. Everything the stock menu does not provide and the Issue still requires is
added through supported composition around it, never by reimplementing rows or
navigation.

Boundaries: menu contents, their order, labels, checked states and the actions
they dispatch do not change. No new actions, no #108 editor, no dependency change,
no splitter (#132). The ruler context menu stays the stock `PopupMenu` it already
is. The toolbar buttons from #130 keep their look and behaviour.

Implementation class: **A**, declared before implementation: focus routing and
overlay isolation across `app_menu.rs`, `app_menu_ui.rs`, `shell.rs` and probably
`settings_ui.rs` / `plot_view.rs` tests, with a deletion well above 400 lines.

Roles for this Issue, set by the owner on 2026-09-28: implementer **Kilo Code** with
`openrouter/stealth/space-bunny-alpha`, variant `xhigh`; reviewer **Codex**
`gpt-6-sol`, reasoning effort `high`, for every review round of this PR. Claude writes
this plan, arbitrates review disagreements, runs the gate and talks to the owner.

## Context

Verified against the locked `gpui-component` 0.6.6 (`src/menu/popup_menu.rs`,
`dropdown_menu.rs`, `popover.rs`):

- `PopupMenu` binds, in key context `PopupMenu`, only `enter` (Confirm), `escape`
  (Cancel), `up`, `down`, `left`, `right`. Up/Down wrap over enabled rows; Right
  focuses an open submenu; Left in a submenu returns focus to the parent but the
  submenu stays drawn, because the parent's `selected_index` is private and still
  names the branch row.
- `Cancel` calls the private `dismiss`, which emits `DismissEvent`, restores focus to
  `previous_focus_handle` or `action_context`, and recursively dismisses every
  parent. Escape therefore closes the whole chain. There is no public way to close
  one level while keeping its parent open and selected.
- Hover selects a row and opens its submenu; leaving a command row clears the
  selection; leaving a submenu row keeps it. Submenus are drawn `deferred` one
  priority above their parent and anchored beside the parent row, flipping left when
  `max_w + origin.x` exceeds the window width and snapping to the window with a
  4-pixel margin.
- `PopupMenuItem` supports `checked`, `disabled`, an `action`, an `on_click`
  handler and `PopupMenuItem::element` rows with a custom render. Confirm on an item
  calls its handler, or focuses `action_context` and dispatches its action, then
  dismisses. Shortcut hints are stock `Kbd`s resolved through `action_context`, the
  trigger and the previous focus.
- `scrollable(true)` caps the height at `max_h` (default half the window, at most
  450 pixels) and scrolls; `max_w` defaults to 500 pixels.
- Outside clicks dismiss through `on_mouse_down_out`, a capture-phase listener that
  does not consume the click, so without a covering layer the click also reaches
  the plot or the title bar beneath.
- An external `cx.emit(DismissEvent)` on the menu entity is the public way to close
  it from outside (`PlotView` already does this for the ruler menu).
- `Popover` with `open(bool)`, `on_open_change`, `appearance(false)` and a `Button`
  trigger is the controlled composition `hints::pinned` already uses, and
  `hints::backdrop` is the transparent occluding layer that keeps the pointer from
  the plot while a surface is open.

Current code:

- `app_menu.rs`: `Item`, `Kind`, `Menu` (selection stack, hover, step, edge, back,
  enter, `activate_numbered`), `Effect`, `file_items` (recent ordering) and `place`.
- `app_menu_ui.rs`: `Shell::application_items` / `application_view_items` build the
  rows; `toggle_application_menu` / `dismiss_application_menu` / `menu_effect` /
  `application_menu_key` drive them; `application_menu_overlay` / `_panel` / `_row`
  and `menu_width` render them; `toolbar` measures the button bounds into
  `application_menu_anchor`. The toolbar controls from #130 live in the same file.
- `shell.rs` holds `application_menu` and `application_menu_anchor`, binds F10 to
  `OpenApplicationMenu` in `Shell`, closes the menu from `open` and elsewhere, and
  renders the overlay. `settings_ui.rs` closes it before the settings window opens.

## Decisions

- **The menu is one stock `PopupMenu` entity** built fresh each time it opens, with
  two stock submenus, File and View, and View's own "Time scale format" submenu.
  Shell keeps `Option<Entity<PopupMenu>>` and a `DismissEvent` subscription that
  clears it, returns focus through `Shell::focus_target`, resets
  `title_drag_pending` and notifies. The menu's `action_context` is
  `Shell::focus_target`, so an action row first returns focus to the plot (or the
  shell without a document) and then dispatches from there, as today, and the
  keycaps resolve against the plot's bindings.
- **Rows:** Open file (`ChooseFile`), the recent files, Settings (`EditAnalysis`) in
  File, with separators placed exactly as `file_items` places them now; View rows,
  checks and separators as `application_view_items` lists them. Action rows use the
  stock `PopupMenuItem::new(..).action(..).checked(..)`. Recent rows are
  `PopupMenuItem::element` rows that render the muted localized digit
  (`numbers::number`) before an ellipsized label for the first nine and the label
  alone after, with an `on_click` that opens the origin. That handler runs inside
  the menu's own update and the menu dismisses itself right after it, so it must not
  touch the menu entity: defer the open with `window.defer` (or equivalent) so
  `Shell::open` runs after the menu is dismissed. Cap the width with `max_w(420.)`
  and make File `scrollable(true)`.
- **Open and close:** the application button stays the #130 `Button` and becomes the
  trigger of a controlled `Popover` (`open(menu.is_some())`, `on_open_change`,
  `appearance(false)`, anchored below the button's outer bottom-left edge) whose
  content is the menu entity, following `hints::pinned`. The button click and F10
  (`OpenApplicationMenu`, still bound in `Shell`) toggle it through the same Shell
  method, which interrupts the plot, closes the analysis hint and refreshes recent
  availability before building, as `toggle_application_menu` does now, and focuses
  the menu. Delete `application_menu_anchor` and its canvas if the Popover anchors
  to the trigger itself. The button keeps its `On` look and its suppressed hint
  while the menu is open.
- **Isolation:** while the menu is open a transparent occluding backdrop, as
  `hints::backdrop`, covers the window beneath the menu layers. An outside click or
  wheel therefore dismisses the menu (the stock capture-phase listener) and reaches
  nothing else: no pan, no title drag or maximize, no button activation, and a click
  on the application button closes the menu without reopening it. The stock menu
  and its submenus already `occlude()` themselves.
- **Keys the stock menu lacks, through composition only.** The Popover content wraps
  the menu in a `div` with key context `ApplicationMenu`; submenus sit on its
  dispatch path because deferred draws keep their parent's dispatch node.
  - F10 and Tab in `ApplicationMenu` dismiss the menu (emit `DismissEvent`), as now.
  - Digits 1 to 9 in `ApplicationMenu` open the numbered recent row, only while the
    File submenu entity holds keyboard focus (entered with Right or Enter). Shell
    builds the File submenu itself so it keeps that entity to test. A digit with
    focus elsewhere does nothing. This narrows today's behaviour, where a digit also
    worked with File only hovered open, and the owner checks it natively.
  - Nothing else is added: Home, End and Space stop working in the menu, and
    Escape closes the whole menu, as the owner accepted.
- **Recent ordering stays toolkit-neutral and tested.** Keep `file_items` (or an
  equivalent pure function) with its test; delete `Menu`, `Effect`, `Kind`-driven
  navigation, `place`, their tests, `ApplicationMenu`, `application_menu_key`,
  `menu_effect`, the overlay / panel / row renderers, `menu_width`, `ROW`,
  `SEPARATOR`, `PADDING` and everything else only they use.
- **Keycaps become the stock menu keycaps**, the borderless `Kbd` the ruler context
  menu already shows, so both menus look alike. `shortcuts::keycap` and
  `shortcuts::width` stay for hints and anything else that still uses them; delete
  only what loses its last caller.
- **Ruler context menu unchanged**, including its retained entity and explicit
  DismissEvent tracking (#144). Re-verify that it still satisfies the #129
  isolation cases after this change.

## Rejected alternatives

- Keep the custom cascade for one-level Escape: the owner chose the stock contract.
- Close one level through `rebuild` or synthesized SelectLeft/SelectUp dispatches:
  rebuild drops the parent selection and recreates submenu entities, and
  synthesized selection is a private-state workaround, not composition.
- Build each level as a separate top-level `PopupMenu` cascaded by Shell: Confirm
  on a branch row dismisses its own menu, and Shell cannot see the stock selection
  for Right, so it needs custom navigation again.
- `Button::dropdown_menu`: its open state is internal keyed state, so F10 cannot
  open it and Shell cannot close it when a file opens or the settings window opens.
- Digits bound in the generic `PopupMenu` context: they would reach the ruler
  context menu too.
- Stock outside dismissal without a backdrop: the dismissing click would start a
  pan or a title drag, which the #129 isolation gate forbids.

## Implementation steps

- [ ] Build the application menu as stock `PopupMenu` with File, View and Time scale
      format submenus, the same rows, checks, separators, actions and recent order;
      recent rows as element rows with digits and a deferred open.
- [ ] Host it in a controlled `Popover` on the application button with the backdrop;
      route the button, F10, `Shell::open`, the settings window and every existing
      dismissal path through one Shell open/dismiss pair with focus return.
- [ ] Add the `ApplicationMenu` key context for F10 / Tab dismissal and File-focused
      digit activation.
- [ ] Delete the custom navigation model, placement, overlay, renderers, key handler
      and width measurement, and every test and helper only they used; keep the
      recent-ordering function and its test.
- [ ] Headless GPUI tests (`gpui-kit` `test-support`), replacing the deleted ones
      where they covered behaviour that remains:
  - the button and F10 open the menu with keyboard focus in it; a second F10, Tab,
    Escape and an outside click close it and return focus to `focus_target`; the
    next key reaches the plot;
  - an outside click on the plot while the menu is open starts no pan and changes
    no view; a wheel over the plot changes no view;
  - a click on the application button while the menu is open closes it and it stays
    closed;
  - Enter on a View row dispatches its action exactly once (for example Show grid
    toggles once) and closes the menu;
  - a recent row click and its digit (with File focused) open that recent entry;
    a digit with File not focused does nothing; recent rows follow the filtered
    order with digits only on the first nine;
  - opening a file and opening the settings window close the menu;
  - the ruler context menu's existing tests still pass unchanged.
  Where a case cannot be driven headlessly, say so here and leave it to the native
  matrix.
- [ ] Update the parent inventory rows (main cascading menu: replaced, with the
      accepted Escape/Home/End/Space change; ruler context menu: retained,
      re-verified) and tick phase 5's menu item for the menu half.
- [ ] Update AGENTS.md ("Application menu and toolbar", "Overlay surfaces" menu
      bullet, `app_menu.rs` ownership sentence, one-level Escape) and add
      `CHANGELOG.md` `[Unreleased]` entries for the user-visible changes.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Native matrix on Linux (owner), recorded in the PR as
      `case | revision | platform/backend, theme, orientation | input | expected |
      observed | pass/fail/not exercised`:
  - F1: after closing the menu each plot navigation key acts once;
  - F3: Escape from File, from View and from Time scale format closes the menu and
    the next key reaches the plot; Left returns to the parent level;
  - keyboard: Down/Up wrap, Right/Enter enter a submenu, Enter runs a row, F10 and
    Tab close, digits with File focused open recent files;
  - hover: moving between File and View switches submenus; checked rows show their
    marks; keycaps present on every row that has a binding;
  - P1: click, wheel and drag inside the menu and outside it over the plot, the
    rulers and the minimap change no view and start no drag;
  - P3: open the menu during a drag, and via F10 during a drag: the drag ends;
  - R3: the application button opens and closes the menu without moving or
    maximizing the window; an outside click on the bare title closes the menu and
    does not move the window;
  - narrow and short windows: submenus stay on screen and File scrolls when it
    does not fit; the parent row remains reachable;
  - both themes, both orientations, start page (File only) and with a document;
  - ruler context menu: open, choose, Escape, outside click, Alt guides after
    dismissal, as in #129.
- [ ] Windows and macOS: `ci/full`; native interaction there is not exercised unless
      the owner runs it, and is reported as such.

## Post-completion

- Continue with #132 (splitter) on the same standard-control basis.
