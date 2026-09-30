# #124 control and interaction inventory

Closure of the inventory the [parent plan](../plans/completed/124-standard-ui-architecture.md) asked
for: every render and input entry point in `crates/app/src`, the parent inventory row it
belongs to, and a final disposition for every parent row.

Scope: the production application tree. The #126 compatibility fixture was removed in this
stage, so `crates/app/examples` holds only the `fft_scheduling` benchmark; the fixture's
history stays in [126-compatibility](126-compatibility/).

## Reproducing the audit

From the repository root. `PAT` names every render and input entry point plus the
test-module marker, so the `awk` pass can drop test scaffolding without a second
search:

```sh
PAT='impl Render for|impl gpui_kit::Render for|on_mouse_[a-z_]*|on_scroll_wheel|on_key_[a-z_]*|on_action|on_drag[a-z_]*|intercept_keystrokes|observe_keystrokes|on_click|on_hover|on_double_click|on_modifiers_changed|on_drop|on_release|observe_release|observe_window_[a-z_]*|on_mouse_event|\.context_menu\(|on_open_change|\.tooltip\(|#\[cfg\(test\)\]'

rg -n --no-heading "$PAT" crates/app/src | awk -F: '
  { if (test[$1]) next; if ($0 ~ /#\[cfg\(test\)\]/) { test[$1] = 1; next } print }'
```

That command returns 101 entry points at this revision. The frame's own render
method is not an `impl Render` block and is found separately:

```sh
rg -n 'pub fn render' crates/app/src
```

`\.tooltip\(` covers both the element method and `interactivity().tooltip(`,
which is why the control hints in I35 appear alongside the content hints in I19.
One production hit is deliberately absent: `hints.rs:471` builds the passive hint
inside `#[cfg(test)] mod tests` and is scaffolding, not an entry point.

Toolkit-neutral models (`navigation`, `frequency`, `panels`, `time_ruler`,
`numbers`, `session`, `settings`) name no toolkit type and are not entry points.

`file:line` below is the position after the fixture removal, which is the position these
commits leave behind.

## Render entry points

| # | Location | What it renders | Parent row |
| --- | --- | --- | --- |
| E00 | `shell.rs:1421` | The Argand frame's content: title bar, panels, status bar | R1, R6, R12 |
| E01 | `plot_view.rs:291` | The plot surface, its rulers, its overlays and the corner zoom pairs | R3, R8, R9, R12 |
| E02 | `hints.rs:128` | The pinned hint's open state; the hint body is a stock `Popover` | R9 |
| E03 | `settings_editor.rs:373` | The analysis settings editor the pinned FFT hint shows (#108) | R7 |
| E04 | `app_menu_ui.rs:815` | `NoTooltip`, the empty view that stands in for the pressed application button's hint | R4 |
| E05 | `chrome.rs:135` | `Frame::render`: insets, rounded corners, border, shadow, resize regions | R1 |

## Input entry points

### Window frame and title bar

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I01 | `chrome.rs:54` | Linux caption control press: `prevent_default` and `stop_propagation` so a control never starts a window move | R1 | Retained |
| I02 | `chrome.rs:58` | Linux minimize, zoom and close clicks | R1 | Retained |
| I03 | `chrome.rs:178` | Per-edge resize hitbox press, which calls `start_window_resize` | R1 | Retained |
| I04 | `shell.rs:1101` | Bare-title double click zooms the window | R2 | Retained |
| I05 | `shell.rs:1102`, `shell.rs:1103`, `shell.rs:1107`, `shell.rs:1111` | Title drag: arm on press, disarm on release or a press outside, start the move on the first move | R2 | Retained |
| I06 | `shell.rs:1122` | Title right click opens the platform window menu | R2 | Retained |
| I07 | `app_menu_ui.rs:406`, `app_menu_ui.rs:410`, `app_menu_ui.rs:411` | The toolbar hitbox consumes left press, right press and double click so a control never reaches the title gestures | R2, R4 | Retained |

### Plot gestures and keyboard

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I08 | `plot_view.rs:310` | Wheel over the plot, routed to the dominant axis | R12 | Retained |
| I09 | `plot_view.rs:311`, `plot_view.rs:313`, `plot_view.rs:314` | Press, release and release-outside begin and end a pan | R12 | Retained |
| I10 | `plot_view.rs:312` | Pointer move updates the readout and continues a drag | R12 | Retained |
| I11 | `plot_view.rs:317` | Hover, in input-modality-independent mode, takes and clears the pointer | R12 | Retained |
| I12 | `plot_view.rs:352` | The drag tracker's window-level move listener, which follows a drag beyond the plot's hitbox | R12 | Retained |
| I13 | `plot_view.rs:219` to `plot_view.rs:286` | Sixteen navigation actions on the plot's own key context | R11, R12 | Retained |
| I14 | `plot_view.rs:372` | The symbolic-key interceptor, gated by window id and the plot's exact focus handle | R11 | Retained |
| I15 | `shell.rs:1451` | Modifier change repaints, which the Alt guides read while painting | R9, R12 | Retained |

### Menus, hints and keycaps

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I16 | `navigation_ui.rs:917` | `.context_menu` on the time ruler, which builds a stock `PopupMenu` | R8 | Retained |
| I17 | `hints.rs:157` | The pinned hint's `Popover` open change, which is how the hint opens and closes | R9 | Retained |
| I18 | `hints.rs:173` | `Cancel` inside the pinned hint's own key context, which closes it and asks for a revert | R9 | Retained |
| I19 | `settings_ui.rs:283`, `settings_ui.rs:328`, `settings_ui.rs:410`, `plot_ui.rs:289` | Passive `Tooltip` metadata, range, shortcut and unit hints over content | R9 | Retained |
| I20 | `settings_ui.rs:433` | The analysis summary's hover, which starts the pinned hint's open delay | R9 | Retained |
| I35 | `app_menu_ui.rs:383` | The application button's hint, which yields to `NoTooltip` while the menu is open | R4, R9 | Retained |
| I36 | `app_menu_ui.rs:496`, `app_menu_ui.rs:528` | The orientation segment's and the grid toggle's framed shortcut hints | R6, R9 | Retained |
| I37 | `plot_ui.rs:568` | A zoom half's framed shortcut hint, naming the axis and its binding | R3, R9 | Retained |
| I38 | `shell.rs:1298`, `shell.rs:1328` | The start-page recent rows' and the chooser's hints, the rows carrying their digit binding | R6, R9 | Retained |

### Toolbar, status bar and start page

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I21 | `app_menu_ui.rs:448` | The orientation `ButtonGroup` reports its selected segment | R6 | Replaced (#130) |
| I22 | `app_menu_ui.rs:524` | The grid `Button` dispatches `ToggleGrid` | R6 | Replaced (#130) |
| I23 | `plot_ui.rs:564` | A zoom half dispatches its registered zoom action and hands focus back to the plot | R3 | Replaced (#130) |
| I24 | `settings_ui.rs:382` | The actionable range item dispatches `UseRecommendedRange`; the informational item has no handler | R6 | Retained |
| I25 | `settings_ui.rs:431` | The analysis summary opens the settings hint with the keyboard in it (#108) | R6 | Retained |
| I26 | `settings_editor.rs` (`range_readout`) | The hint's recommendation and edit buttons are gone (#108); the warned Range value and its balloon apply the recommendation on a click | R9 | Removed |
| I27 | `shell.rs:1287` | A recent row opens its capture | R6 | Retained |
| I28 | `shell.rs:1327` | The start page's chooser dispatches `ChooseFile` | R6 | Retained |
| I29 | `app_menu_ui.rs:109` | A recent row inside the File branch opens its capture | R4 | Replaced (#131) |
| I30 | `app_menu_ui.rs:357` | The application `Popover` follows the menu's own open state | R4 | Replaced (#131) |
| I31 | `app_menu_ui.rs:622`, `app_menu_ui.rs:626` | The `ApplicationMenu` key context carries F10, Tab and the File rows' digits | R4 | Replaced (#131) |

### Settings hint (#108, which replaced the settings window)

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I32 | `settings_editor.rs` (`range_readout`) | The recommendation button left the hint; its Range row marks a warned range and explains it in a balloon with a pointer | R7 | Removed |
| I33 | `settings_editor.rs:401`, `settings_editor.rs:405` | `UseRecommendedRange`, and `Confirm` taken before the popover so Enter keeps an unusable number open | R7 | Retained |
| I34 | `settings_editor.rs:319` | Defaults | R7 | Retained |
| I39 | `settings_editor.rs:547`, `settings_editor.rs:568`, `settings_editor.rs:579` | The standard select's confirm, the input's Enter and blur, and the number steppers preview a value | R7 | Retained |

### Infrastructure observers and adapters

These are recorded separately from ordinary controls, as the parent plan requires.

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| A01 | `shell.rs:405` | Window bounds, which drive session geometry | A05 | Retained |
| A02 | `shell.rs:406` | Window activation, which refreshes the recent list and ends the plot's gestures | A05 | Retained |
| A03 | `shell.rs:416` | Window appearance, which re-syncs the interface theme | A05 | Retained |
| A04 | `shell.rs:419` | `App::observe_keystrokes`, which dismisses the ready status in whichever window the key arrived | R10 | Retained |
| A05 | `shell.rs:507`, `shell.rs:513` | `Window::on_mouse_event` for move and exit, which follow whether the mouse is in the window | R10 | Retained |
| A06 | `shell.rs:540`, `shell.rs:546` | `Window::on_mouse_event` for press and wheel, which dismiss the ready status | R10 | Retained |
| A07 | `shell.rs:975`, `shell.rs:984`, `shell.rs:993` | App-level `ChooseFile`, `UseRecommendedRange` and `EditAnalysis`, deferred so popup focus releases first | A01 | Retained |
| A08 | `shell.rs:1452`, `shell.rs:1453`, `shell.rs:1454`, `shell.rs:1455` | The shell's own action handlers, reached by bubbling from the plot and by global dispatch | R4, R6 | Retained |
| A09 | `shell.rs:1459`, `shell.rs:1464` | Tab traversal, held while a pinned hint holds the keyboard | R9 | Retained |
| A10 | `shell.rs:1471` | A capture dropped on the window opens | A05 | Retained |
| A12 | `navigation_ui.rs:600` to `navigation_ui.rs:619` | Six session commands, which change the session rather than the view | R6, R12 | Retained |

## Final disposition of every parent inventory row

The parent table keeps one line per control. This section gives each row an outcome.

### R1 Custom frame and Linux window controls

**Retained.** `chrome.rs` stays the only owner of decoration insets, resizing, the border,
the shadow and the Linux controls. The stock alternative is gpui-component's
`Root::bordered(false)`, which the production main window now uses for its focus, input and
overlay infrastructure, and `TitleBar`, which the production title bar already uses on
Windows and macOS.

Verified limitations in the locked sources, which are why the frame and its controls stay:

- `gpui-component-0.6.6/src/window_border.rs:150` and `:195` take the resize geometry from
  `window.window_bounds().get_bounds().size`, the *reported* outer rectangle, not from
  `window.viewport_size()`. The production frame measures `Frame::for_window` against
  `window.viewport_size()` (`chrome.rs:88`), which is what the owner accepted and what the
  #126 checkpoint found displaced in the stock frame.
- Not a limitation: `window_border.rs:188` to `:203` starts no resize when every edge is
  tiled, and the Wayland backend reports a maximized or fullscreen window as tiled on
  every edge (`gpui-pre-linux-0.3.6/src/linux/wayland/window.rs:1240` to `:1241`). The
  production frame keeps its own guard for expanded windows (`chrome.rs:94` to `:103`,
  `expanded_windows_have_no_resize_regions_or_corners`), which is equivalent there.
- `WindowControls` (`gpui-component-0.6.6/src/title_bar.rs:248`) and `ControlIcon` (`:111`)
  are private types, and `TitleBar::render` appends `WindowControls` unconditionally
  (`:400`). There is no supported way to replace or restyle them. The Linux branch of
  `ControlIcon::render` (`:232`) calls `window.zoom_window()` on every click of
  `Maximize` and `Restore`, so a double click on that button toggles twice, which is the
  #126 finding the retained controls answer. The bare title's double click is not the
  issue: `TitleBar` zooms on it (`:344` to `:346`), as the production bar does.

Composition is insufficient because the retained behaviour lives in the geometry the stock
border computes and in a control subtree the stock bar will not surrender; there is no
builder hook that reaches either.

Covering tests: `resize_regions_leave_all_content_to_its_own_cursor`,
`resize_grips_are_reachable_inside_the_visible_frame`,
`expanded_windows_have_no_resize_regions_or_corners`,
`only_free_tiled_edges_can_resize_or_round`, `native_decorations_do_not_get_a_second_frame`
(`chrome_tests.rs`, included at `chrome.rs:234`). Native rows R1 to R3 in the Linux matrix.

### R2 Title-bar drag and double-click handlers

**Retained** for Linux, where `shell.rs:1089` builds the bar; Windows and macOS already use
the stock `TitleBar` (`shell.rs:1131`).

The standard alternative is gpui-component's `TitleBar`, and the production bar is its
composition: the same `TITLE_BAR_HEIGHT`, the same `title_bar_border` and `title_bar`
tokens, the same double-click zoom and the same press/release/move drag sequence
(`gpui-component-0.6.6/src/title_bar.rs:344` to `:371`).

Verified limitation: `TitleBar` cannot be composed without its private caption controls
(`title_bar.rs:248`, `:400`), which on Linux call `zoom_window()` on every click
(`:232`) and take their hover and active colours from the global secondary tokens
(`:168` to `:191`) with no per-control style hook. The production bar additionally carries
a filename region centred on the full window, and the toolbar inside the bar must consume
drag and double click, which `app_menu_ui.rs:406` to `:411` does by stopping propagation on
its own hitbox.

Covering tests: `the_document_controls_join_the_toolbar`,
`the_title_reserves_exactly_what_the_toolbar_draws` (`app_menu_ui.rs:894`, `:906`).
Native rows R3 and O1.

### R3 Zoom halves and `pressed_zoom`

**Replaced (#130).** Each half is a gpui-component `Button` in a shared frame
(`plot_ui.rs:525` to `:577`); the component owns press, disabled and click, and
`pressed_zoom`, `release_press` and the `ToggleScaleUi` repair are gone.

Covering tests: `the_halves_split_their_pair_in_two`,
`a_zoom_half_zooms_once_and_leaves_the_keyboard_on_the_plot`,
`a_release_outside_a_zoom_half_changes_nothing`,
`hiding_the_pairs_during_a_press_leaves_nothing_behind` (`plot_view.rs:1119`, `:1156`,
`:1179`, `:1209`), plus `the_document_controls_join_the_toolbar`.

### R4 Main cascading menu

**Replaced (#131).** One stock `PopupMenu` drawn by a controlled `Popover`
(`app_menu_ui.rs:350`). Escape now closes the whole chain, which the owner accepted in
place of the one-level behaviour of the former custom menu, because
`PopupMenu::dismiss` in the locked source dismisses the entire parent chain
(`gpui-component-0.6.6/src/menu/popup_menu.rs:1052` to `:1081`) and there is no supported
option to stop it. `NoTooltip` (`app_menu_ui.rs:815`) is the one remaining adapter: an empty
view that stands in for the trigger's hint while the menu holds it, because the trigger
`Button` is a stock component whose tooltip cannot be suppressed.

Covering tests: `the_button_and_the_key_open_the_menu_with_the_keyboard_in_it`,
`a_click_on_the_button_closes_the_menu_and_leaves_it_closed`,
`the_key_escape_and_an_outside_click_each_close_the_menu`,
`an_outside_click_and_wheel_reach_nothing_under_the_menu`,
`the_backdrop_keeps_the_window_beneath_the_menu`,
`enter_on_a_view_row_dispatches_its_action_once`,
`a_digit_answers_only_in_the_file_branch`, `a_recent_row_opens_its_capture`,
`the_settings_hint_opens_from_the_menu_itself`,
`a_click_on_a_recent_row_opens_that_capture`,
`a_digit_opens_the_row_that_was_drawn`,
`enter_runs_no_branch_where_the_stock_menu_ignores_one`,
`the_keys_reach_the_menu_after_a_hover_drops_the_focused_branch`,
`an_action_row_draws_the_shared_keycap_when_its_binding_resolves`,
`a_long_recent_name_keeps_the_file_branch_within_its_width` (`app_menu_ui.rs:1185` to
`:1526`).

### R5 Waveform/spectrum splitter

**Removed (#132).** The owner decided the minimap is not resizable: it has a fixed height,
or width in vertical orientation, from `[panels].minimap_size`. The drag strip, its state,
the fraction intent and `Session::waveform_fraction` are gone, and the standard
`ResizablePanelGroup` is not used. The locked `ResizablePanel` publishes its callback on
mouse up only (`gpui-base-0.6.6/src/resizable/panel.rs:473` to `:486`), which could not have
answered the continuous observation the removed handle provided, so nothing was adopted in
its place.

Covering tests: `a_rem_size_follows_the_font_and_not_the_window`,
`a_pixel_size_ignores_the_font`, `sizes_parse_with_either_unit_and_any_spacing`,
`unusable_sizes_are_refused` (`panels.rs:83` to `:136`) for the fixed size,
`the_minimap_size_reads_either_unit` and `an_unusable_minimap_size_falls_back_alone`
(`config_tests.rs:373`, `:387`) for its configuration, and
`the_version_goes_up_when_the_layout_gains_something` (`session_tests.rs:679`) for the
session that no longer carries the fraction.

### R6 Toolbar, status range/FFT, start/recent buttons

**Toolbar replaced, the rest retained (#130).** The toolbar, the orientation segments, the
grid toggle and the zoom halves are gpui-component `Button` and `ButtonGroup`; the range
item, the FFT summary, the start-page chooser and the recent rows were already standard
`Button`s and stay. The custom variant paints no border, so `toolbar_style` supplies the
surfaces (`app_menu_ui.rs:773`).

**No locked-toolkit limitation is claimed for the status items.** They are standard
`Button`s with a custom variant, and the audit found no custom press, hover or focus
machinery in them. The accepted warning and neutral colours are painted by that variant.

Covering tests: `the_document_controls_join_the_toolbar`,
`the_title_reserves_exactly_what_the_toolbar_draws`, `the_segment_group_carries_its_own_frame`,
`only_the_other_orientation_segment_switches` (`app_menu_ui.rs:894` to `:970`); the
`settings.rs` cases `confirming_numeric_edits_validates_both_fields_without_losing_either`
and `displayed_value_and_state_advance_as_one_analysis_snapshot` (`settings.rs:213`, `:408`)
for the range item's action and its labels.

### R7 Settings form

**Retained, and already standard; moved into the FFT hint by #108.** The settings are
gpui-component `Select` and `Input` drawn with `appearance(false)` in the pinned hint's
popover (`settings_editor.rs`), plus standard `Button`s for the steppers and Reset to
defaults. The separate `Root`-backed window with Reset, Cancel and OK is gone. There is
no custom editing, focus or validation machinery; the numeric policy is a small
application-value adapter on the standard `InputEvent`, and the editor takes `Confirm`
before the popover so that Enter in a number applies it and keeps the hint. One piece of
press handling is custom and owner-approved: a stepper steps at the press and repeats
while held (`Editor::start_repeat`), until release, the pointer leaving the button or a
list choice. The standard alternative is `NumberInput`, whose steppers in the locked
0.6.6 sources step on click only and expose no press hook, so composition cannot reach
a hold-to-repeat; the steppers are not tab stops, the keyboard steps through the input's
own arrows, and the repeat stops on an unusable value.

The standard alternative is the same component, so no limitation is claimed. Composition
would not help: the form's contract is a preview-and-keep transaction over a signal
document, not a control.

Covering tests: `settings_editor::hint_tests` (choosing previews and keeps the hint, Enter
applies, closes and saves, an unusable number keeps the hint with its error, Escape restores
settings and views, Defaults) and `settings_editor::standard_input_tests` (standard
in-window typing, choosing and one-level Escape in a window whose root is a standard
`Root`). `confirming_numeric_edits_validates_both_fields_without_losing_either` and
`saved_choices_round_trip_and_invalid_values_fall_back` in `settings.rs` cover the
transaction. Native row S1.

### R8 Ruler context menu

**Retained, and already standard.** `navigation_ui.rs:917` opens a stock `PopupMenu`
through the toolkit's `.context_menu`. The only custom part is bookkeeping: the popup menu
entity is retained by a toolkit reference cycle, so `PlotView` tracks it and matches its
`DismissEvent` to clear the menu and restore the pointer (`plot_view.rs:140`). That is
lifecycle tracking, not a control, and it cannot be composed away.

Covering tests: `an_overlay_on_the_plot_hides_its_pointer_until_the_first_move_back`,
`an_overlay_on_the_plot_takes_its_wheel_clicks_and_drags`,
`a_drag_keeps_tracking_over_a_hint_on_the_plot` and
`a_new_orientation_keeps_a_resting_pointer` (`plot_view.rs:820`, `:849`, `:1017`, `:942`)
drive an occluding layer over the plot and read the pointer and the intents, which is the
same contract the menu's own hitbox has to keep. The popup entity's own retention and
dismissal cannot be opened in a headless test (toolkit reference cycle, #144), so its
dismissal is a native row: F3 and P1.

### R9 Metadata, analysis and shortcut hints, and keycaps

**Retained, and already standard.** Passive hints are the stock `Tooltip`; the pinned hint
is the stock `Popover`; keycaps are `Kbd` with a shared text refinement and a measured
width (`shortcuts.rs`). `hints.rs` centralizes the surface contracts, not the controls.

Every passive hint registration in the production tree is I19 for the hints over content
and I35 to I38 for the hints attached to a control: the application button, the
orientation segment, the grid toggle, a zoom half, a start-page recent row and the
chooser. Five of the six go through `shortcut_tooltip`, so they carry the same framed
keycap as the status bar, and the application button's is the one `NoTooltip` stands in
for.

Verified limitations, both already met in code:

- The popover's own key context binds `space` to `Confirm`
  (`gpui-base-0.6.6/src/popover.rs:21`). A pinned hint must not close on Space, so
  `hints::init` binds `space` to `NoAction` in the `PinnedHint` context
  (`hints.rs:31`). This is a binding, not a control.
- `HitboxBehavior::BlockMouse`, which `.occlude()` sets, makes every hitbox behind it
  report `should_handle_scroll() == false`
  (`gpui-pre-0.3.6/src/window.rs:879` to `:898`). A passive hint must not take the wheel
  from the plot, so `hints::passive` (`hints.rs:38`) uses a plain `.tooltip` and never
  occludes, while the pinned hint's backdrop (`hints.rs:184` to `:196`) does occlude,
  because while it is open the plot must get no wheel at all.

Covering tests: `a_short_hover_opens_nothing`, `leaving_the_trigger_keeps_the_hint_open`,
`an_open_hint_keeps_the_plot_from_input`,
`a_click_outside_closes_the_hint_and_goes_no_further`,
`enter_closes_the_hint_and_keeps_the_values`, `space_leaves_the_hint_open`,
`escape_closes_the_hint_and_reverts`,
`a_passive_hint_leaves_its_trigger_the_click` (`hints.rs:337` to `:481`);
`the_shell_follows_the_mouse_into_and_out_of_the_window`,
`edit_analysis_opens_the_hint_and_closes_it_keeping_the_values`,
`escape_restores_the_settings_the_hint_opened_with`,
`other_closes_keep_what_changed` (`shell.rs:1777` to `:1831`). Native row P2.

### R10 Ready-input capture canvas

**Retained** as the minimal passive registration adapter the parent plan allows. It draws
nothing; it exists only to call `Window::on_mouse_event` from a paint callback.

Verified limitation: `Window::on_mouse_event` is the only public window-level mouse
registration, and it is paint-phase only. It calls `self.invalidator.debug_assert_paint()`
and pushes into `self.next_frame.mouse_listeners`
(`gpui-pre-0.3.6/src/window.rs:5254` to `:5267`), so it must be registered while the frame
is drawn. The element extension methods that could replace it are bound to one element's
own hitbox and cannot see an event a target has already consumed, which is the property the
ready-status dismissal needs. The key half needs no canvas: `App::observe_keystrokes`
(`gpui-pre-0.3.6/src/app.rs:2295`) is the supported app-level hook and fires after every
other mechanism has resolved.

Covering tests: `a_disabled_hint_opens_neither_on_hover_nor_on_a_right_click` and
`an_open_hint_keeps_the_plot_from_input` show the observer never consumes and never
navigates; `key_presses_keep_the_pointer_over_the_plot` (`plot_view.rs:883`) shows a key
press does not clear the plot. Native row O1.

### R11 Global symbolic-key interceptor

**Retained as a scoped adapter.** `navigation_ui::init` binds key bindings but registers
no interceptor. `plot_view.rs:366` to `:387` registers one interceptor per `PlotView`,
keeps it in a field so dropping the subscription drops with the plot, and returns before
recognizing anything unless the event's window is the plot's own window and
`focus.is_focused(window)` holds.

Verified limitation: `App::intercept_keystrokes` is the only hook that runs before binding
match, and it is application-global. The locked source states this directly
(`gpui-pre-0.3.6/src/app.rs:2317` to `:2321`: "fires _before_ all other action and event
mechanisms have resolved ... `cx.stop_propagation` calls within interceptors will prevent
action dispatch"), and there is no per-element or per-window variant. The Shift loss the
adapter repairs is a platform keystroke-translation behaviour that a later hook cannot see.
Gating on window and focus is therefore the whole of what the parent plan asks for, and
there is no supported registration that would make it narrower.

Covering tests: `every_navigation_key_emits_exactly_one_intent`,
`plot_keys_need_the_plot_itself_focused`, `another_window_never_reaches_the_plot`,
`a_replaced_plot_stops_intercepting` (`plot_view.rs:538`, `:649`, `:663`, `:686`).
Native row F2.

### R12 Spectrogram, waveform, axes, minimap and cursor badges

**Retained domain drawing.** The GPUI canvas, image and path primitives are the standard
foundation here, not an alternative being bypassed. The audit found no ordinary control
inside this area: the only element handlers are the plot's own gestures (I08 to I15) and
the six session commands (A12), and the only retained `Button`s over it are the corner
zoom pairs of R3.

Verified limitation: gpui-component's plot module is a painter, not a calibrated axis. Its
`PlotAxis` takes a fixed `x` or `y` in pixels and a caller-supplied iterator of
`AxisText { text, tick, color }` (`gpui-component-0.6.6/src/plot/axis.rs:54` to `:180`), and
its `Grid` takes pre-computed pixel vectors (`src/plot/grid.rs:5` to `:36`). Nothing there
derives a label ladder in physical units, applies the edge-mark policy, formats through the
locale, or reports the gutter it needs, which `argand_core::axis` and the measured
`LabelMeasure` do. `Shape` supplies line, area and arc paths, not a two-sided
`-Fs/2` to `+Fs/2` domain, a per-frame min/max envelope, or a full-capture minimap.

Covering tests: `axes_tests.rs`, `navigation_tests.rs`, `spectrogram_tests.rs`,
`minimap_tests.rs`, `document_tests.rs`, the plot-view gesture tests
(`plot_view.rs:779` to `:1014`) and `retirement_clears_the_snapshot_first`
(`plot_view.rs:708`). Native rows F1, G1 and the orientation rows.

### R13 Rejected numeric editor in PR #106

**Not ported, and now out of reach.** The branch is not merged, the editor was rejected,
and #108 restarts on the standard `NumberInput` and `Input` that R7 already uses. Nothing
in this tree carries it.

## Locked-toolkit observations

Paths are in `gpui-kit 0.6.6`'s locked graph, located by Cargo, not instructions to modify
the registry. Every limitation claimed above is one of these.

| Observation | Source | Consequence |
| --- | --- | --- |
| The window's top-level entity must be `Root` | `gpui-component-0.6.6/src/root.rs:160` to `:163`, `:176` to `:181` | Standard in-window inputs need a real `Root`, which is why the headless tests open one |
| `Root` always composes `window_border` unless told otherwise | `src/root.rs:143`, `:604` to `:605` | Production asks for `bordered(false)`; a second frame would double the border |
| The stock border measures resize from `window_bounds()` and has no expanded guard | `src/window_border.rs:150`, `:195`, `:426` to `:441` | R1 is retained |
| `WindowControls` and `ControlIcon` are private and always appended by `TitleBar` | `src/title_bar.rs:248`, `:400`; Linux click at `:232` | R1 and R2 are retained |
| The popover's key context binds `space` to `Confirm` | `gpui-base-0.6.6/src/popover.rs:21` | `hints::init` binds `NoAction` for a pinned hint |
| `PopupMenu::dismiss` closes the whole parent chain | `gpui-component-0.6.6/src/menu/popup_menu.rs:1052` to `:1081` | R4's Escape closes the whole chain, accepted by the owner in #131 |
| `Select::escape` stops at its own list when it is open and propagates when it is not | `src/select.rs:404` to `:415` | The one-level Escape the new headless test proves |
| `Select::new` derives its element id from the state entity, and the list inside is a deferred popup | `src/select.rs:642`; the deferred popup at `:602` to `:634` | The headless test names its select and drives the list with the toolkit's own keys |
| `occlude()` blocks the wheel as well as the pointer | `gpui-pre-0.3.6/src/window.rs:879` to `:898`; `src/elements/div.rs:1244` | R9's passive hints never occlude |
| `Window::on_mouse_event` is paint-phase only | `gpui-pre-0.3.6/src/window.rs:5254` to `:5267` | R10 keeps its canvas |
| `App::observe_keystrokes` fires after, `App::intercept_keystrokes` before, and both for every window | `src/app.rs:2295`, `:2321` | R10 uses the observer; R11 keeps a focus-gated interceptor |
| `ResizablePanel` publishes `on_resize` on mouse up only | `gpui-base-0.6.6/src/resizable/panel.rs:473` to `:486` | R5 was removed rather than adopted |
| `PlotAxis` and `Grid` take pre-computed pixels and caller-supplied labels | `gpui-component-0.6.6/src/plot/axis.rs:54` to `:180`; `src/plot/grid.rs:5` to `:36` | R12 is retained domain drawing |

## Removed verification surface

`crates/app/examples/ui_compatibility.rs` and `crates/app/examples/ui_compatibility/` were
the #126 and #137 probe. They are gone, together with the library face
`crates/app/src/lib.rs` that existed only to let the example share `chrome.rs`; `chrome.rs`
is a module of the binary again. What replaces them is
`settings_editor::standard_input_tests`, a headless window whose root is a standard `Root`
and whose body is a standard `NumberInput` and `Select`. The evidence the fixture gathered
stays in [126-compatibility](126-compatibility/) as history.
