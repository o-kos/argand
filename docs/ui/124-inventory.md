# #124 control and interaction inventory

Closure of the inventory the [parent plan](../plans/completed/124-standard-ui-architecture.md) asked
for: every render and input entry point in `crates/app/src`, the parent inventory row it
belongs to, and a final disposition for every parent row.

Scope: the production application tree. The #126 compatibility fixture was removed in this
stage, so `crates/app/examples` holds only the `fft_scheduling` benchmark; the fixture's
history stays in [126-compatibility](126-compatibility/).

## Current 0.9.1 reassessment (#226)

The dispositions below supersede the historical 0.6.6 audit that follows.
Source paths name the registry `gpui-component-0.9.1`, `gpui-base-0.9.1` and
`gpui-pre-0.3.12` graph; no registry source is modified.

| Row | Current disposition and exact-source evidence | Coverage / limits |
| --- | --- | --- |
| R1 frame | Replaced by standard Root/WindowBorder. Component `root.rs:711` registers overlay/frame presentation, `:734` wraps the surface. `window_border.rs:109` exposes paddings, `:199` and `:227` still use reported bounds. Owner approved the configured preview, including shadow inset 20 and radius 8. | `chrome_test.rs` checks local/global backdrop geometry for restored, fully/partly tiled and server-decorated windows. Runtime compositor resize/maximize/restore still needs native acceptance. |
| R1 captions, R2 title gestures | Retained separately from the frame. Component `title_bar.rs:250` keeps WindowControls private; Linux click handler `:234` zooms directly. No public replacement of that subtree. Existing shared Argand controls/gestures are preserved, as approved in the preview. | Existing caption/toolbar gesture and focus tests; native double-click/drag checks remain required. |
| R3, R6 buttons | Continue standard Button/ButtonGroup composition; no replacement control introduced. | Existing pressed/selected/disabled/focus tests. |
| R4 application menu | Standard PopupMenu in controlled Popover retained for application digits, focus restoration and outside-input isolation. `menu/popup_menu.rs:887` confirms through activate; `:923` selects a submenu. Owner approved Enter plus Right. | Menu keyboard/row/digit/outside-click/wheel tests; Space remains inert. |
| R5 splitter | Remains removed; no migration reason to reintroduce resizable UI. | Existing minimap layout checks. |
| R7 numeric editing | Existing shared NumericEditor retains standard Input/Button behavior and the approved press/repeat transaction. Base `number_input.rs:266` and `:277` still step on click. Base now exposes button decorators (`:188`, `:194`), but Component internally owns those decorators in `input/number_input.rs:151`, `:184`; no component-level press/repeat contract is supplied. Replacing the existing shared editor is unnecessary for this migration. | Existing repeat, commit, locale, disabled and both property-panel regressions; native consumers still require checks. |
| R8 ruler menu | Stock ContextMenu retained. `menu/context_menu.rs:513` uses a Weak subscription, fixing the old retention cycle. PlotView bookkeeping remains for pointer/focus restoration, not to compensate for a leak. | `shell/plot/plot_view_context_menu_test.rs` exercises real ruler/spectrum menus, keyboard isolation/recovery and popup release on window close. Native menu appearance and OS interaction are not established by these headless tests. |
| R9 hints/keycaps | Standard Tooltip/Popover/Kbd remain. Base `popover.rs:376` introduces PopoverContent with Escape handling, while Enter belongs to the trigger (`:343`) and PopoverContent explicitly blocks it with NoAction (`:27`). PinnedHint therefore explicitly binds/handles Confirm to preserve its approved Enter-close contract; child inputs/selects consume Confirm first. | Pinned lifecycle, nested selection, numeric commit and focus tests. |
| R10 ready-input observer | Retained: GPUI `window.rs:5704` asserts paint phase for on_mouse_event. The existing canvas is still required to register the non-consuming window observer. | Ready dismissal and input passthrough tests. |
| R11 symbolic keys | Retained plot-scoped interceptor; GPUI `app.rs:2705`, `:2720` still registers global observers/interceptors. | Exact-focus and multiwindow isolation tests. |
| R12 domain drawing | Retained domain canvases, not ordinary-control replacements. Toolkit axis primitives do not own RF semantics or the bounded render model. | Axis/navigation/render-model and texture-retirement regression tests. |
| R13 abandoned custom editor | Remains absent. | No new text editing implementation. |

Corner zoom pairs use standard div anchors in the current plot rectangle inside
the existing domain canvas. Their Button behavior, identities and styling remain
standard; no new widget or retained geometry owner is introduced. The plot's
font-measured rectangle is only available in prepaint, so a panel-level anchor
would include the minimap and axis gutters. Public AnyElement layout/prepaint/paint
APIs host the controls there, as supported by GPUI `element.rs:694`, `:705`, `:689`
and the component radar chart's element-label lifecycle (`chart/radar_chart.rs:469`,
`:482`). Deferred gesture geometry keeps the same corner policy but no longer
positions the controls. First-resize-frame regression compares actual painted
plot bounds with rendered Button bounds, without waiting for deferred convergence.
The existing Paste-here marker is prepared in the same frame-local overlay bundle
and painted before the standard controls, preserving its previous stacking order.
A render/input regression checks overlapping marker/control quads and click ownership;
this composition adds no retained entity, custom Element or replacement control.

### Frame observer composition

`FrameObserver` is a stateless Base RootPlugin, not a second root or a custom
control. The standard component plugin remains the sole frame and resize owner.
The observer's wrapper receives bubbled left presses only; stock WindowBorder's
handler (`window_border.rs:217`) starts resize without stopping propagation.
Deferred popups can consume input first. The callback resolves Shell through the
current Root and retains no strong entity or focus handle. Geometry has one shared
local-to-window transform, based on public toolkit paddings. No fixed 20-pixel
padding is duplicated in application code. The private four-pixel hit band and
one-pixel border are explicitly mirrored only to keep backdrops out of resizing.

`frame_press_bubbles_but_a_popup_above_the_edge_keeps_its_input` exercises real
GPUI dispatch for an uncovered edge, content and a blocking popup. Headless
TestWindow reports server decorations; this test does not establish native client
frame/compositor interactions. Those remain native acceptance scenarios.

## Historical 0.6.6 audit

The following source observations and line numbers preserve the earlier audit's
reasoning. They are not current 0.9.1 limitations or instructions to retain a
superseded frame.

## Reproducing the audit

From the repository root. `PAT` names every render and input entry point plus the
test-module marker, so the `awk` pass can drop test scaffolding without a second
search:

```sh
PAT='impl Render for|impl gpui_kit::Render for|on_mouse_[a-z_]*|capture_any_mouse_down|on_scroll_wheel|on_key_[a-z_]*|on_action|on_drag[a-z_]*|intercept_keystrokes|observe_keystrokes|on_click|on_hover|on_double_click|on_modifiers_changed|on_drop|on_release|observe_release|observe_window_[a-z_]*|on_mouse_event|\.context_menu\(|on_open_change|\.tooltip\(|#\[cfg\(test\)\]'

rg -n --no-heading "$PAT" crates/app/src | awk -F: '
  { if (test[$1]) next; if ($0 ~ /#\[cfg\(test\)\]/) { test[$1] = 1; next } print }'
```

That command returns 111 entry points at this revision, of which one is a
`capture_any_mouse_down` registration, named since #156, which the earlier
`on_mouse_` alternation missed. The frame's own render
method is not an `impl Render` block and is found separately:

```sh
rg -n 'pub fn render' crates/app/src
```

`\.tooltip\(` covers both the element method and `interactivity().tooltip(`,
which is why the control hints in I35 appear alongside the content hints in I19.
One production hit is deliberately absent: `shell/hints.rs:471` builds the passive hint
inside `#[cfg(test)] mod tests` and is scaffolding, not an entry point.

Toolkit-neutral models (`navigation`, `frequency`, `panels`, `time_ruler`,
`numbers`, `session`, `settings`) name no toolkit type and are not entry points.

`file:line` below is the position after the fixture removal, which is the position these
commits leave behind.

## Render entry points

| # | Location | What it renders | Parent row |
| --- | --- | --- | --- |
| E00 | `shell/mod.rs:1421` | The Argand frame's content: title bar, panels, status bar | R1, R6, R12 |
| E01 | `shell/plot/plot_view.rs:291` | The plot surface, its rulers, its overlays and the corner zoom pairs | R3, R8, R9, R12 |
| E02 | `shell/hints.rs:128` | The pinned hint's open state; the hint body is a stock `Popover` | R9 |
| E03 | `shell/settings_ui/editor.rs:373` | The analysis settings editor the pinned FFT hint shows (#108) | R7 |
| E04 | `shell/app_menu_ui.rs:815` | `NoTooltip`, the empty view that stands in for the pressed application button's hint | R4 |
| E05 | `chrome.rs:135` | `Frame::render`: insets, rounded corners, border, shadow, resize regions | R1 |

## Input entry points

### Window frame and title bar

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I01 | `chrome.rs:54` | Linux caption control press: `prevent_default` and `stop_propagation` so a control never starts a window move | R1 | Retained |
| I02 | `chrome.rs:58` | Linux minimize, zoom and close clicks | R1 | Retained |
| I03 | `chrome.rs:178` | Per-edge resize hitbox press, which calls `start_window_resize` | R1 | Retained |
| I04 | `shell/mod.rs:1101` | Bare-title double click zooms the window | R2 | Retained |
| I05 | `shell/mod.rs:1102`, `shell/mod.rs:1103`, `shell/mod.rs:1107`, `shell/mod.rs:1111` | Title drag: arm on press, disarm on release or a press outside, start the move on the first move | R2 | Retained |
| I06 | `shell/mod.rs:1122` | Title right click opens the platform window menu | R2 | Retained |
| I07 | `shell/app_menu_ui.rs:405`, `shell/app_menu_ui.rs:816` to `:819`, `shell/app_menu_ui.rs:827`, `:828`, `:830` | Each toolbar control consumes its own left press and double click through `title_control`, the container keeps its right-press stop, and `keeps_focus` prevents the focusing default in the capture phase for a press that is not the left one, so a control never reaches the title gestures and never takes the keyboard from a right or middle press while the gaps between them do carry the drag | R2, R4 | Retained (#158, #156) |

### Plot gestures and keyboard

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I08 | `shell/plot/plot_view.rs:310` | Wheel over the plot, routed to the dominant axis | R12 | Retained |
| I09 | `shell/plot/plot_view.rs:311`, `shell/plot/plot_view.rs:313`, `shell/plot/plot_view.rs:314` | Press, release and release-outside begin and end a pan | R12 | Retained |
| I10 | `shell/plot/plot_view.rs:312` | Pointer move updates the readout and continues a drag | R12 | Retained |
| I11 | `shell/plot/plot_view.rs:317` | Hover, in input-modality-independent mode, takes and clears the pointer | R12 | Retained |
| I12 | `shell/plot/plot_view.rs:352` | The drag tracker's window-level move listener, which follows a drag beyond the plot's hitbox | R12 | Retained |
| I13 | `shell/plot/plot_view.rs:219` to `shell/plot/plot_view.rs:286` | Sixteen navigation actions on the plot's own key context | R11, R12 | Retained |
| I14 | `shell/plot/plot_view.rs:372` | The symbolic-key interceptor, gated by window id and the plot's exact focus handle | R11 | Retained |
| I15 | `shell/mod.rs:1451` | Modifier change repaints, which the Alt guides read while painting | R9, R12 | Retained |

### Menus, hints and keycaps

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I16 | `shell/plot/navigation_ui.rs:917` | `.context_menu` on the time ruler, which builds a stock `PopupMenu` | R8 | Retained |
| I17 | `shell/hints.rs:157` | The pinned hint's `Popover` open change, which is how the hint opens and closes | R9 | Retained |
| I18 | `shell/hints.rs:173` | `Cancel` inside the pinned hint's own key context, which closes it and asks for a revert | R9 | Retained |
| I19 | `shell/settings_ui/mod.rs:283`, `shell/settings_ui/mod.rs:328`, `shell/settings_ui/mod.rs:410`, `shell/plot/plot_ui.rs:289` | Passive `Tooltip` metadata, range, shortcut and unit hints over content | R9 | Retained |
| I20 | `shell/settings_ui/mod.rs:433` | The analysis summary's hover, which starts the pinned hint's open delay | R9 | Retained |
| I35 | `shell/app_menu_ui.rs:383` | The application button's hint, which yields to `NoTooltip` while the menu is open | R4, R9 | Retained |
| I36 | `shell/app_menu_ui.rs:496`, `shell/app_menu_ui.rs:528` | The orientation segment's and the grid toggle's framed shortcut hints | R6, R9 | Retained |
| I37 | `shell/plot/plot_ui.rs:568` | A zoom half's framed shortcut hint, naming the axis and its binding | R3, R9 | Retained |
| I38 | `shell/mod.rs:1298`, `shell/mod.rs:1328` | The start-page recent rows' and the chooser's hints, the rows carrying their digit binding | R6, R9 | Retained |

### Toolbar, status bar and start page

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I21 | `shell/app_menu_ui.rs:448` | The orientation `ButtonGroup` reports its selected segment | R6 | Replaced (#130) |
| I22 | `shell/app_menu_ui.rs:524` | The grid `Button` dispatches `ToggleGrid` | R6 | Replaced (#130) |
| I23 | `shell/plot/plot_ui.rs:564` | A zoom half dispatches its registered zoom action and hands focus back to the plot | R3 | Replaced (#130) |
| I24 | `shell/settings_ui/mod.rs:377` | The actionable range item dispatches `UseRecommendedRange`, wrapped by `keeps_focus` so no press takes the keyboard from the plot; the informational item has no handler | R6 | Retained (#156) |
| I25 | `shell/settings_ui/mod.rs:428` | The analysis summary opens the settings hint with the keyboard in it (#108), wrapped by `keeps_focus` so no press takes the keyboard from the plot | R6 | Retained (#156) |
| I26 | `shell/settings_ui/editor.rs` (`range_readout`) | The hint's recommendation and edit buttons are gone (#108); the warned Range value and its balloon apply the recommendation on a click | R9 | Removed |
| I27 | `shell/mod.rs:1287` | A recent row opens its capture | R6 | Retained |
| I28 | `shell/mod.rs:1327` | The start page's chooser dispatches `ChooseFile` | R6 | Retained |
| I29 | `shell/app_menu_ui.rs:109` | A recent row inside the File branch opens its capture | R4 | Replaced (#131) |
| I30 | `shell/app_menu_ui.rs:357` | The application `Popover` follows the menu's own open state | R4 | Replaced (#131) |
| I31 | `shell/app_menu_ui.rs:622`, `shell/app_menu_ui.rs:626` | The `ApplicationMenu` key context carries F10, Tab and the File rows' digits | R4 | Replaced (#131) |

### Settings hint (#108, which replaced the settings window)

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| I32 | `shell/settings_ui/editor.rs` (`range_readout`) | The recommendation button left the hint; its Range row marks a warned range and explains it in a balloon with a pointer | R7 | Removed |
| I33 | `shell/settings_ui/editor.rs:401`, `shell/settings_ui/editor.rs:405` | `UseRecommendedRange`, and `Confirm` taken before the popover so Enter keeps an unusable number open | R7 | Retained |
| I34 | `shell/settings_ui/editor.rs:319` | Defaults | R7 | Retained |
| I39 | `shell/settings_ui/editor.rs` (`traverse`, `arrow`, `defaults`) | Tab and Shift+Tab bounded to the hint's focus scope, Up and Down stepping a focused number, and Enter on Reset to defaults' own focus place (#169) | R7 | Retained |
| I40 | `chrome.rs` (`Frame::render`), `shell/mod.rs` (`edge_pressed`, the title bar's capture press) | Backdrops cover only the frame's free area; the resize edges and the title bar close the overlay themselves and keep their press (#170) | R4, R9 | Retained |
| I39 | `shell/settings_ui/editor.rs:547`, `shell/settings_ui/editor.rs:568`, `shell/settings_ui/editor.rs:579` | The standard select's confirm, the input's Enter and blur, and the number steppers preview a value | R7 | Retained |

### Infrastructure observers and adapters

These are recorded separately from ordinary controls, as the parent plan requires.

| # | Location | What it does | Parent row | Disposition |
| --- | --- | --- | --- | --- |
| A01 | `shell/mod.rs:405` | Window bounds, which drive session geometry | A05 | Retained |
| A02 | `shell/mod.rs:406` | Window activation, which refreshes the recent list and ends the plot's gestures | A05 | Retained |
| A03 | `shell/mod.rs:416` | Window appearance, which re-syncs the interface theme | A05 | Retained |
| A04 | `shell/mod.rs:419` | `App::observe_keystrokes`, which dismisses the ready status in whichever window the key arrived | R10 | Retained |
| A05 | `shell/mod.rs:507`, `shell/mod.rs:513` | `Window::on_mouse_event` for move and exit, which follow whether the mouse is in the window | R10 | Retained |
| A06 | `shell/mod.rs:540`, `shell/mod.rs:546` | `Window::on_mouse_event` for press and wheel, which dismiss the ready status | R10 | Retained |
| A07 | `shell/mod.rs:975`, `shell/mod.rs:984`, `shell/mod.rs:993` | App-level `ChooseFile`, `UseRecommendedRange` and `EditAnalysis`, deferred so popup focus releases first | A01 | Retained |
| A08 | `shell/mod.rs:1452`, `shell/mod.rs:1453`, `shell/mod.rs:1454`, `shell/mod.rs:1455` | The shell's own action handlers, reached by bubbling from the plot and by global dispatch | R4, R6 | Retained |
| A09 | `shell/mod.rs:1459`, `shell/mod.rs:1464` | Tab traversal, held while a pinned hint holds the keyboard | R9 | Retained |
| A10 | `shell/mod.rs:1471` | A capture dropped on the window opens | A05 | Retained |
| A12 | `shell/plot/navigation_ui.rs:600` to `shell/plot/navigation_ui.rs:619` | Six session commands, which change the session rather than the view | R6, R12 | Retained |

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
(`chrome_test.rs`, included at `chrome.rs:234`). Native rows R1 to R3 in the Linux matrix.

### R2 Title-bar drag and double-click handlers

**Retained** for Linux, where `shell/mod.rs:1089` builds the bar; Windows and macOS already use
the stock `TitleBar` (`shell/mod.rs:1131`).

The standard alternative is gpui-component's `TitleBar`, and the production bar is its
composition: the same `TITLE_BAR_HEIGHT`, the same `title_bar_border` and `title_bar`
tokens, the same double-click zoom and the same press/release/move drag sequence
(`gpui-component-0.6.6/src/title_bar.rs:344` to `:371`).

Verified limitation: `TitleBar` cannot be composed without its private caption controls
(`title_bar.rs:248`, `:400`), which on Linux call `zoom_window()` on every click
(`:232`) and take their hover and active colours from the global secondary tokens
(`:168` to `:191`) with no per-control style hook. The production bar additionally carries
a filename region centred on the full window. Each control in the toolbar consumes its own
left press and double click through `title_control` (`shell/app_menu_ui.rs:767` to `:771`), and
the container keeps only its right-press stop, so the gaps between controls stay bare
title-bar pixels that carry the drag and the double click (#158).

Covering test: `the_toolbar_gaps_carry_the_title_drag_and_its_controls_do_not`, which mutation checks
against both the occluding container and a consumption-free wrapper.
Native rows R3 and O1.

When a status editor's stock Select is open, title-bar Capture dispatches the
standard Cancel synchronously through its saved trigger before closing the hint.
The locked Select's outside Capture handler otherwise consumes that first press
(`select.rs:404`, `:625`). Closing the standard Select first lets the original
press reach the existing caption control without replay or an alternate close
path. `caption_close_with_open_sample_list_reaches_unsaved_workflow` covers
one-click unsaved protection and press/release outside cancellation; removing the
Cancel composition fails the one-click assertion.

### R3 Zoom halves and `pressed_zoom`

**Replaced (#130).** Each half is a gpui-component `Button` in a shared frame
(`shell/plot/plot_ui.rs:525` to `:577`); the component owns press, disabled and click, and
`pressed_zoom`, `release_press` and the `ToggleScaleUi` repair are gone.

Covering tests: `a_zoom_half_zooms_once_and_leaves_the_keyboard_on_the_plot` and
`hiding_the_pairs_during_a_press_leaves_nothing_behind`. Private split geometry and
stock-button outside-release smoke checks were retired in #222.

### R4 Main cascading menu

**Replaced (#131).** One stock `PopupMenu` drawn by a controlled `Popover`
(`shell/app_menu_ui.rs:350`). Escape now closes the whole chain, which the owner accepted in
place of the one-level behaviour of the former custom menu, because
`PopupMenu::dismiss` in the locked source dismisses the entire parent chain
(`gpui-component-0.6.6/src/menu/popup_menu.rs:1052` to `:1081`) and there is no supported
option to stop it. `NoTooltip` (`shell/app_menu_ui.rs:815`) is the one remaining adapter: an empty
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
`a_long_recent_name_keeps_the_file_branch_within_its_width` (`shell/app_menu_ui.rs:1185` to
`:1526`).

### R5 Waveform/spectrum splitter

**Removed (#132).** The owner decided the minimap is not resizable: it has a fixed height,
or width in vertical orientation, from `[panels].minimap_size`. The drag strip, its state,
the fraction intent and `Session::waveform_fraction` are gone, and the standard
`ResizablePanelGroup` is not used. The locked `ResizablePanel` publishes its callback on
mouse up only (`gpui-base-0.6.6/src/resizable/panel.rs:473` to `:486`), which could not have
answered the continuous observation the removed handle provided, so nothing was adopted in
its place.

Covering tests: `unusable_sizes_are_refused`, `the_minimap_size_reads_either_unit`
and `an_unusable_minimap_size_falls_back_alone`. Simple rem/pixel conversion and
session-version literal checks were retired in #222; configuration behavior is unchanged.

### R6 Toolbar, status range/FFT, start/recent buttons

**Toolbar replaced, the rest retained (#130).** The toolbar, the orientation segments, the
grid toggle and the zoom halves are gpui-component `Button` and `ButtonGroup`; the range
item, the FFT summary, the start-page chooser and the recent rows were already standard
`Button`s and stay. The custom variant paints no border, so `toolbar_style` supplies the
surfaces (`shell/app_menu_ui.rs:773`).

**No locked-toolkit limitation is claimed for the status items.** They are standard
`Button`s with a custom variant, and the audit found no custom press, hover or focus
machinery in them. The accepted warning and neutral colours are painted by that variant.

Covering tests: `only_the_other_orientation_segment_switches`,
`confirming_numeric_edits_validates_both_fields_without_losing_either` and
`displayed_value_and_state_advance_as_one_analysis_snapshot`. Private toolbar
composition snapshots were retired in #222.

### R7 Settings form

**Retained, and already standard; moved into the FFT hint by #108.** The settings are
gpui-component `Select` and `Input` drawn with `appearance(false)` in the pinned hint's
popover (`shell/settings_ui/editor.rs`), plus standard `Button`s for the steppers and Reset to
defaults. The separate `Root`-backed window with Reset, Cancel and OK is gone. There is
no custom editing, focus or validation machinery; the numeric policy is a small
application-value adapter on the standard `InputEvent`, and the editor takes `Confirm`
before the popover so that Enter in a number applies it and keeps the hint. Their signs and the Reset to defaults label follow the pointer through `pointer_ink` (a
group hover and group active over the button), because a custom button variant has one
foreground for every state (#123). One piece of
press handling is custom and owner-approved: a stepper steps at the press and repeats
while held (`Editor::start_repeat`), until release, the pointer leaving the button or a
list choice. The standard alternative is `NumberInput`, whose steppers in the locked
0.6.6 sources step on click only and expose no press hook, so composition cannot reach
a hold-to-repeat; the steppers are not tab stops, the keyboard steps through the input's
own arrows, and the repeat stops on an unusable value, on Reset to defaults, on the
recommendation and once its stepper is no longer shown.

Range and sample-format warnings share `shell/property_advice.rs`: measured side placement,
a standard close Button, the retained triangular pointer drawing, and ordinary Tooltip
fallback only when neither side has a readable width. After dismissal, a fresh marker
entry reopens the same anchored balloon when room permits. The format warning
is informational; Range retains its recommendation action and shortcut. Both disappear
while a property Select is open. This extracts existing approved composition rather than
introducing a replacement tooltip, select, button or input. The standard Tooltip cannot
supply the persistent adjacent callout and dismiss-to-marker lifecycle; the same existing
composition supplies it to both property editors. Keyboard focus remains with standard
controls; the close Button is not a Tab stop. The format-advice GPUI regression covers
automatic display, Close, same-balloon hover reopening, open-list suppression, explicit clearing and
constrained-viewport placement alongside the existing Range regressions.

For the rest of the form the standard alternative is the same component, so no limitation is claimed. Composition
would not help: the form's contract is a preview-and-keep transaction over a signal
document, not a control.

Covering tests: `settings_editor::hint_tests` (choosing previews and keeps the hint, Enter in a number
applies it and keeps the hint, a keeping close saves, a held stepper stops when the pointer
leaves it, a list stays put while the keyboard walks it, an unusable number keeps the hint with its error, Escape restores
settings and views, Defaults). The toolkit-only stock-control fixture was retired;
the actual editor form and event workflows remain the product boundary. `confirming_numeric_edits_validates_both_fields_without_losing_either` and
`saved_choices_round_trip_and_invalid_values_fall_back` in `settings.rs` cover the
transaction. Native row S1.

### R8 Ruler context menu

**Retained, and already standard.** `PlotView::time_context_menu` and
`PlotView::spectrum_context_menu` in `shell/plot/navigation_ui.rs` open stock `PopupMenu`s
through the toolkit's `.context_menu`. GPUI Component 0.9.1
`menu/context_menu.rs:513` captures a Weak reference in its dismiss subscription,
removing the retention cycle recorded in #144. PlotView's weak menu handle and
`DismissEvent` subscription still clear menu state and restore the pointer; this
is application lifecycle tracking, not a replacement control.

Covering tests: `an_overlay_on_the_plot_hides_its_pointer_until_the_first_move_back`,
`an_overlay_on_the_plot_takes_its_wheel_clicks_and_drags`,
`a_drag_keeps_tracking_over_a_hint_on_the_plot` and
`a_new_orientation_keeps_a_resting_pointer` in `shell/plot/plot_view.rs` drive an occluding
layer over the plot and read the pointer and intents. The UI integration tests
in `shell/plot/plot_view_context_menu_test.rs` additionally open the production menus with
real right-click events. They check ruler-menu focus and isolation of Ctrl+G,
time/frequency zoom and Home, Escape focus restoration and command recovery,
reopening, and release of time/spectrum popup entities after closing their
windows without dismissing them. Lifetime assertions use WeakEntity directly;
they do not rely on the optional GPUI leak detector. Native appearance, pointer
transitions and OS interaction remain outside this automated evidence.

### R9 Metadata, analysis and shortcut hints, and keycaps

**Retained, and already standard.** Passive hints are the stock `Tooltip`; the pinned hint
is the stock `Popover`; keycaps are `Kbd` with a shared text refinement and a measured
width (`shell/shortcuts.rs`). `shell/hints.rs` centralizes the surface contracts, not the controls.

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
  (`shell/hints.rs:31`). This is a binding, not a control.
- `HitboxBehavior::BlockMouse`, which `.occlude()` sets, makes every hitbox behind it
  report `should_handle_scroll() == false`
  (`gpui-pre-0.3.6/src/window.rs:879` to `:898`). A passive hint must not take the wheel
  from the plot, so `hints::passive` (`shell/hints.rs:38`) uses a plain `.tooltip` and never
  occludes, while the pinned hint's backdrop (`shell/hints.rs:184` to `:196`) does occlude,
  because while it is open the plot must get no wheel at all.

Covering tests: `a_short_hover_opens_nothing`, `leaving_the_trigger_keeps_the_hint_open`,
`an_open_hint_keeps_the_plot_from_input`,
`a_click_outside_closes_the_hint_and_goes_no_further`,
`enter_closes_the_hint_and_keeps_the_values`, `space_leaves_the_hint_open`,
`escape_closes_the_hint_and_reverts`,
`a_passive_hint_leaves_its_trigger_the_click` (`shell/hints.rs:337` to `:481`);
`the_shell_follows_the_mouse_into_and_out_of_the_window`,
`edit_analysis_opens_the_hint_and_closes_it_keeping_the_values`. Duplicate
settings-model close-policy checks were retired in #222; the real form and event
workflows retain transaction coverage. Native row P2.

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
navigates; `key_presses_keep_the_pointer_over_the_plot` (`shell/plot/plot_view.rs:883`) shows a key
press does not clear the plot. Native row O1.

### R11 Global symbolic-key interceptor

**Retained as a scoped adapter.** `navigation_ui::init` binds key bindings but registers
no interceptor. `shell/plot/plot_view.rs:366` to `:387` registers one interceptor per `PlotView`,
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
`a_replaced_plot_stops_intercepting` (`shell/plot/plot_view.rs:538`, `:649`, `:663`, `:686`).
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

Covering tests: `axes/axes_test.rs`, `navigation_test.rs`, `spectrogram_test.rs`,
`minimap_test.rs`, `document_test.rs`, the plot-view gesture tests
(`shell/plot/plot_view.rs:779` to `:1014`) and `retirement_clears_the_snapshot_first`
(`shell/plot/plot_view.rs:708`). Native rows F1, G1 and the orientation rows.

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
is a module of the binary again. The later toolkit-only stock-control fixture was
also retired in #222; current verification belongs to the actual settings editor
form/event workflows. The evidence the fixture gathered stays in
[126-compatibility](126-compatibility/) as history.

Advice dismissal and hover now share the same anchored balloon when side space is
available. Standard GPUI hover handlers cover the existing anchor, body, pointer,
close Button and three-pixel connecting gap; deferred reconciliation combines
those occluding hit regions without timers or input replay. Only a new anchor
entry reopens dismissed advice. Transient advice hides after leaving the union,
and open Selects clear transient state. Stock Tooltip fallback remains only where
neither side can fit the balloon. Rendered pointer coverage asserts the balloon
ID, unchanged geometry, usable Close Button, gap crossing and suppression.

Editors explicitly deactivate advice while its warning/readout is absent, preserving
dismissal but clearing transient hover and obsolete anchor bounds. Reactivation
distinguishes an outside pointer from an anchor reappearing beneath it. The Range
regression applies the reopened recommendation, leaves while fixed, restores the
warned range with Ctrl+R and verifies that only a fresh entry reopens the balloon.

## Sample format options (#192)

File information and whole-capture conversion retain `shell/conversion_ui.rs`'s shared
standard Select. WAVE offers the canonical twelve core sample types, including
Real / I/Q Int 24-bit; FLAC keeps the existing U8/I16 filter and fallback. Document
format hints identify signed 24-bit integers. Raw opening hints and CLI/configuration
parsing accept `rl_i24` / `iq_i24`; CLI help and reports expose these same tokens.
No control composition or pointer/keyboard handler changes. Native interaction
acceptance must still exercise the new rows; headless Select regressions establish
only automated behavior, not desktop interaction.
