# Issue #131: Stock PopupMenu for the application menu

Resolves [#131](https://github.com/o-kos/argand/issues/131).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](../124-standard-ui-architecture.md), phase 5 (standard-control
replacement), inventory rows "Main cascading menu" and "Ruler context menu".
Predecessor [#130](130-standard-buttons.md), merged in PR #157.

## Overview

The File / View application menu is a hand-built cascade: a toolkit-neutral
navigation model (`app_menu.rs` `Menu`, `Effect`, `place`), `div` panels and rows,
our own keyboard handler, width measurement, placement and a full-window overlay
(`app_menu_ui.rs`). Replace it with the stock gpui-component `PopupMenu` and its
submenus, and delete the custom navigation, placement and rendering.

The owner decided on 2026-09-28 to accept the stock keyboard contract where it
differs from the accepted #91 behaviour, instead of keeping a custom cascade for one-level
Escape. That contract also leaves Enter and Space inert on a branch row, so only
Right enters File, View or Time scale format. Everything the stock menu does not provide and the Issue still requires is
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
  focuses an open submenu; Enter runs a row and does nothing on a submenu row,
  so it never opens a branch; Left in a submenu returns focus to the parent but
  the submenu stays drawn, because the parent's `selected_index` is private and
  still names the branch row.
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
  `appearance(false)`, `track_focus` on the menu's focus handle, anchored
  `TopLeft` so it hangs 4 pixels below the button) whose content is the menu
  entity, following `hints::pinned`. The button click and F10
  (`OpenApplicationMenu`, still bound in `Shell`) toggle it through the same Shell
  method, which interrupts the plot, closes the analysis hint and refreshes recent
  availability before building, as `toggle_application_menu` does now, and focuses
  the menu. Delete `application_menu_anchor` and its canvas if the Popover anchors
  to the trigger itself. The button keeps its `On` look and its suppressed hint
  while the menu is open.
- **Isolation:** while the menu is open a transparent occluding backdrop, as
  `hints::backdrop`, covers the window beneath the menu layers. An outside click
  therefore dismisses the menu (the stock capture-phase listener) and reaches
  nothing else: no pan, no title drag or maximize, no button activation, and a click
  on the application button closes the menu without reopening it. An outside wheel
  reaches nothing either and leaves the menu open, which is what the overlay this
  replaces did. The stock menu and its submenus already `occlude()` themselves.
- **Keys the stock menu lacks, through composition only.** The Popover content wraps
  the menu in a `div` with key context `ApplicationMenu`; submenus sit on its
  dispatch path because deferred draws keep their parent's dispatch node.
  - F10 and Tab in `ApplicationMenu` dismiss the menu (emit `DismissEvent`), as now.
  - Digits 1 to 9 in `ApplicationMenu` open the numbered recent row, only while the
    File submenu entity holds keyboard focus (entered with Right). Shell
    builds the File submenu itself so it keeps that entity to test. A digit with
    focus elsewhere does nothing. This narrows today's behaviour, where a digit also
    worked with File only hovered open, and the owner checks it natively.
  - Nothing else is added: Home, End and Space stop working in the menu,
    Escape closes the whole menu, and Enter or Space on File, View or Time
    scale format does not open that branch, because the stock `confirm` ignores
    a submenu row. The owner accepted all three on 2026-09-28, and only Right
    enters a branch.
- **The keyboard follows a branch that a hover takes away.** The stock menu draws
  a submenu only while its row is selected, and hovering another row moves that
  selection, so the focused submenu can leave the frame while its focus handle
  stays focused; GPUI then routes keys to the root node. While the menu is open
  the shell holds a `Context::on_focus_lost` subscription that focuses the open
  menu's own handle again, and drops it on dismissal, so a lost focus that is not
  a dismissal costs nothing and a dismissal already handled is left alone.
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

## Resolved: focus inside the controlled `Popover` (2026-09-28)

The first implementation run stopped on a stock menu inside a controlled `Popover`
that never ran its own key actions. The cause is the composition, not the toolkit.
When a `Popover` opens it focuses its `tracked_focus_handle`, or its own handle when
none is set (`gpui-base` 0.6.6 `popover.rs`, `PopoverState` open path). Without
`track_focus`, that open focuses the popover's handle after Shell focused the menu,
so key actions dispatch from a node above the menu. The menu's ancestors see them,
and its own `on_action` handlers never run.

- **Give the `Popover` `.track_focus(&menu.focus_handle(cx))`**, as `hints::pinned`
  does with its own handle. A headless probe on this branch confirmed it. With
  `track_focus`, Down then Enter runs the first row once, and Down, Down, Right,
  Enter runs a row in a submenu. Without it, neither does. `Button::dropdown_menu`
  and a menu drawn directly in the tree both work.
- **Placement.** The stock `Popover` with `Anchor::TopLeft` puts the content below
  the trigger. In the same probe the menu's top was 4 pixels under the trigger's
  bottom edge (`render_popover_content` adds `top_1`), with its left edge at the
  trigger's left, inside the window margin. Accept that stock offset rather than
  measuring the button into `application_menu_anchor`.
- **Headless F10.** Shell's key bindings are registered inside `run`'s closure,
  which no test reaches. Move them into a function that `run` and the tests both
  call, so F10 can be exercised headlessly.

## Implementation steps

- [x] Build the application menu as stock `PopupMenu` with File, View and Time scale
      format submenus, the same rows, checks, separators, actions and recent order;
      recent rows as element rows with digits and a deferred open.
- [x] Host it in a controlled `Popover` on the application button with the backdrop;
      route the button, F10, `Shell::open`, the settings window and every existing
      dismissal path through one Shell open/dismiss pair with focus return.
- [x] Add the `ApplicationMenu` key context for F10 / Tab dismissal and File-focused
      digit activation.
- [x] Delete the custom navigation model, placement, overlay, renderers, key handler
      and width measurement, and every test and helper only they used; keep the
      recent-ordering function and its test.
- [x] Headless GPUI tests (`gpui-kit` `test-support`), replacing the deleted ones
      where they covered behaviour that remains:
  - the button and F10 open the menu with keyboard focus in it; a second F10, Tab,
    Escape and an outside click close it and return focus to `focus_target`; the
    next key reaches the plot;
  - an outside click on the plot while the menu is open starts no pan and changes
    no view; a wheel over the plot changes no view and leaves the menu open;
  - a click on the application button while the menu is open closes it and it stays
    closed;
  - F10, Down, Right into File, then a pointer hover onto the View row: the keys
    still close the menu, hand the keyboard back and move the selection;
  - Enter on a View row dispatches its action exactly once (for example Show grid
    toggles once) and closes the menu; Enter on the branch row itself runs nothing
    and leaves the keyboard in the menu, where Right enters it;
  - a recent row click and its digit (with File focused) open that recent entry;
    a digit with File not focused does nothing; a digit opens the row that was
    drawn even when a probe changes the list while the menu is open; recent rows
    follow the filtered order with digits only on the first nine;
  - the settings window opens from the menu's own Settings row, not from a
    direct call;
  - the ruler context menu's existing tests still pass unchanged.
  Where a case cannot be driven headlessly, say so here and leave it to the native
  matrix. Three cases came out in part. "The next key reaches the plot" is checked
  as the keyboard reaching the window again, because a headless `Shell` has no
  described document and therefore no `PlotView`; the plot's own key routing is
  unchanged and already covered there. The pan and wheel cases are checked where
  they are observable without a plot: the view is unchanged, a control under the
  menu does not answer the dismissing click, and a press on the title bar starts
  no window drag. Starting a pan on the plot itself needs a described document
  that a headless window cannot build, and stays a native-matrix case; a real
  pointer click on a stock menu row is covered by the recent-row click test, which
  finds the row by the index the toolkit records it under.
- [x] Update the parent inventory rows (main cascading menu: replaced, with the
      accepted Escape/Home/End/Space change; ruler context menu: retained,
      re-verified) and tick phase 5's menu item for the menu half.
      Phase 5's menu item is `[x]`, the Escape change is recorded in the row and in
      `AGENTS.md`, and the ruler row records that this path was left alone.
- [x] Update AGENTS.md ("Application menu and toolbar", "Overlay surfaces" menu
      bullet, `app_menu.rs` ownership sentence, one-level Escape) and add
      `CHANGELOG.md` `[Unreleased]` entries for the user-visible changes.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Native matrix on Linux (owner), recorded in the PR as
      `case | revision | platform/backend, theme, orientation | input | expected |
      observed | pass/fail/not exercised`:
  - F1: after closing the menu each plot navigation key acts once;
  - F3: Escape from File, from View and from Time scale format closes the menu and
    the next key reaches the plot; Left returns to the parent level;
  - keyboard: Down/Up wrap, Right enters a branch, Enter runs a row and does not
    open one, F10 and Tab close, digits with File focused open recent files;
  - hover: moving between File and View switches submenus; checked rows show their
    marks; keycaps present on every row that has a binding;
  - hover: crossing from a focused branch to another branch, and back, keeps the
    keyboard on the menu, which the headless test drives through one direction;
  - P1: click, wheel and drag inside the menu and outside it over the plot, the
    rulers and the minimap change no view and start no drag; an outside wheel
    leaves the menu open, and a click on Settings works by pointer, which a
    headless test reaches only through the keyboard;
  - the application button's own look while the menu is open, which the headless
    tests cannot observe: the popover must not make it read as selected;
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

## Review round 1

The arbiter's five decisions, all applied on this branch.

- **Accepted, a digit could open the wrong capture.** The File rows were drawn from
  the `recent_entries` snapshot taken when the menu opened, but a digit resolved its
  index in `recent_files.shortcut`, which the asynchronous availability probes keep
  changing. The shell now keeps the captures its numbered rows name, in
  `application_file_rows` beside the File entity, resolves digits against that list
  and clears it on dismissal, and a new test re-checks the first capture as gone
  while the menu stands open and asserts that digit 1 still opens the row that was
  drawn.
- **Accepted, the popover was selecting the application button.**
  `Popover::trigger` calls `selected(selected || is_open)`, and a selected button
  loses its hover and pressed surfaces and paints the variant's active colour, which
  #130 forbids for this control. The trigger is now a `Trigger` wrapper that
  implements `Selectable` as a no-op and renders the wrapped `Button` unchanged, so
  the button's own `ToolbarState::On` remains the only open-menu look and the
  button is not reimplemented. A unit test asserts the wrapper never reports itself
  selected; whether the painted button changes is not observable headlessly and is
  in the native matrix.
- **Declined in code, accepted as a documentation fix.** An outside wheel does not
  dismiss the menu, and that is the behaviour this Issue inherited: the overlay it
  replaces consumed the wheel and left the menu open. The plan's Isolation decision,
  both `AGENTS.md` passages and the `CHANGELOG.md` entry now say that an outside
  click dismisses the menu and reaches nothing else while an outside wheel reaches
  nothing and leaves the menu open, and the existing test asserting that is kept.
- **Accepted in part.** A real simulated click on a drawn recent row opens that
  capture, with the row found by the index the toolkit records it under, and the
  settings window opens from the menu's own Settings row instead of a direct call.
  Pan prevention on a real `PlotView` stays native, because a headless window cannot
  build a described document; the plan's test list says so where the case is listed.
- **Accepted, three comments ran to two lines.** The `space` binding, the recent
  row's deferred open and the application button's press are one line each now, and
  no other comment in the branch spans a line.

Round 2 adds two items. The owner accepted the stock behaviour on 2026-09-28:
`PopupMenu::confirm` does nothing on a `Submenu` row, so Enter and Space never open
File, View or Time scale format and only Right enters a branch. It is recorded
beside the Escape and Home, End and Space change in the plan's Overview, Context
and Decisions, in the plan's keyboard line for the native matrix and its test
list, in the parent inventory row, in `AGENTS.md` and in the `CHANGELOG.md`
entry, and the plan's digit decision now says the File branch is entered with
Right rather than with Right or Enter. A headless test presses F10, Down twice to
the View branch and Enter, and finds the menu open with the keyboard still in it
and no row run, then presses Right and Enter and finds Show grid toggled. The
second item is a comment correction, accepted: the recent row's deferred open is
deferred because the toolkit dismisses the menu as the click handler returns, in
the same `confirm`, and the comment now says that.

Round 3 is the last round and adds two accepted items. The major one is the
keyboard, which the stock menu can lose: it draws a submenu only while its row is
selected, a hover moves that selection, and the handle that still holds the focus
is no longer in the frame, so GPUI routes keys to the root node and F10, Tab,
Escape, the arrows and the digits stop working. The fix is the supported hook, a
`Context::on_focus_lost` subscription the shell takes while the menu is open and
drops on dismissal, which focuses the open menu's own handle again; a focus lost
because the menu closed is left to the dismissal, which has already handled it. A
headless test opens the menu, walks into File, hovers the pointer onto the rendered
View row and draws, and finds that F10, Tab and Escape each still close the menu
and return focus, and that Down still moves the selection in the menu that is
drawn; with the subscription removed that test fails on the first key, which is
what makes it evidence. The native matrix keeps the same crossing in both
directions and from the Time scale format branch. The second item is a nit,
accepted: dismissal now clears the DismissEvent subscription and the focus-lost
subscription with the entity and the File rows, so no subscription outlives the
menu it belongs to.

The owner approved a targeted check of the round-3 fix instead of a fourth round.
The same reviewer examined only that diff, the focus-lost listener, its behaviour
when the menu closes, a file opens or the settings window opens, and whether the
test exercises the lost-focus path, and found no substantive issue.

## Owner feedback 1 (2026-09-28)

The owner's first native check raised three items, decided with the owner.

- [x] **Framed keycaps in the menu.** The stock `PopupMenu` paints its own keycaps
      and forces them borderless and transparent (`render_key_binding`), so they
      differ from the framed `shortcuts::keycap` the hints use. Action rows become
      `PopupMenuItem::element` rows that render the label and, when the action has
      a binding resolved from `Shell::focus_target`, `shortcuts::keycap` at the
      right edge, keeping `.action(..)` and `.checked(..)` so the stock menu still
      dispatches, checks, selects and navigates them. This supersedes the earlier
      decision to accept the stock keycaps.
- [x] **Recent labels are the file name alone**, in the menu and on the start page.
      `session::recent_labels` stops appending ` - <full directory>` to names that
      occur twice; duplicates show the same name. On the start page the row shows
      the name, and its hint shows the name with the containing directory under it
      in a smaller, muted font, beside the existing Alt+digit keycap. The menu shows
      the name only and adds no hint. This is pre-existing behaviour that the owner
      asked to change in this PR, because both lists share the helper.
- [x] **Long names truncate.** A recent row in the menu wrapped or overflowed
      instead of ending in an ellipsis, because its label had `text_ellipsis`
      without `overflow_hidden` and `whitespace_nowrap`. Recent labels in the menu
      and on the start page truncate in the middle (`text_ellipsis_middle`), so the
      extension stays visible, and every flex level between the stock row and the
      label must allow shrinking (`overflow_hidden` / `min_w_0`) so the menu keeps
      its `max_w`.
- [x] Headless tests: an action row still dispatches once and shows its binding
      through `shortcuts::keycap`; two recent captures with the same file name in
      different directories both label as the bare name; a long recent name keeps
      the File submenu within its maximum width. Update the `recent_labels` tests
      in `recent_tests.rs` and `session_tests.rs` to the new rule. What the tests
      can and cannot see: the keycap test binds an action into a key context a
      headless focus target can reach, because without a plot the row's own
      binding is in the Plot context and resolves to nothing, and it proves the
      row grows by the keycap rather than what the keycap says. The bare-name rule
      is asserted in `session_tests.rs`, where the rule lives, because a row's
      text is not observable headlessly. The width test guards the menu's maximum
      and its one-line rows; the ellipsis itself is a native check.
- [x] Update AGENTS.md and the CHANGELOG entries that mention the stock keycaps or
      the directory suffix.

## Post-completion

- Continue with #132 (splitter) on the same standard-control basis.
