# UI architecture

Window composition, focus and overlay contracts. The [control inventory](../ui/124-inventory.md) records toolkit limitations and retained exceptions; [component selection policy](../../CONTRIBUTING.md#ui-component-selection) governs changes.

## UI architecture and ownership

Standard UI infrastructure (#124) is in place, and every control in it is either a toolkit component or a recorded exception. The graph runs on `gpui-kit` 0.9.1 with the default GPUI snapshot 0.3.12. `gpui_kit::open_window` installs the standard `Root`, strongly owning `Shell` as its child, and a per-document `PlotView` owns plot focus and gestures (see [Plot ownership (#128)](ui.md#plot-ownership-128)). Overlays own the input they cover (see [Overlay surfaces (#129)](ui.md#overlay-surfaces-129)). The corner zoom pairs, the title-bar toolbar, the orientation segments and the grid toggle are standard `Button`s and one `ButtonGroup` (see [Standard buttons (#130)](ui.md#standard-buttons-130)). The application menu is a single stock `PopupMenu` drawn by a controlled `Popover`, and the minimap splitter is gone (see [Waveform and axis layout](navigation-selection.md#waveform-and-axis-layout)). What remains custom is the Linux caption controls, the Linux title bar's gestures, the settings form's transaction over the standard inputs, the passive and pinned hint contracts, the ready-input observer canvas, the plot-scoped symbolic-key adapter, and the domain drawing; each one's standard alternative, verified limitation in the locked 0.9.1 sources, why composition cannot reach it and its covering tests are in [control inventory](../ui/124-inventory.md). The FFT hint is the analysis settings surface on that infrastructure (#108).

Window composition: standard `Root` owns theme font setup, the client frame and
component overlay hosting. It retains `Shell`; `Shell` keeps its scoped Tab
traversal. The owner approved the standard frame with Argand's window-border
color and `window_radius = 8` logical pixels. Linux shadow padding is the toolkit's
20 logical pixels rather than the former 12; at a fixed outer size the content is
16 pixels smaller per axis. There is no application resize implementation or
second frame. Linux caption controls and title gestures remain unchanged.

`chrome::overlay_area` derives backdrop coordinates from the toolkit's public
`window_paddings`, viewport size and the locked frame's one-pixel border and
four-pixel resize band. The local coordinate transform is shared with the
`FrameObserver` Root plugin. That plugin only observes bubbled left presses left
to the standard frame and closes overlays; it neither consumes the event nor
starts a resize. Popup handlers above an edge own their input before this wrapper.
The observer resolves the current window's shell through the standard Root and
captures no strong Shell entity. Global key/pointer observers use the same
Root-to-Shell ownership boundary. Dialogs, sheets and notifications are hosted
automatically by the toolkit plugin; Shell draws no manual dialog layer.

Linux title-bar controls paint their own rounded hover/press backgrounds; Windows and macOS retain toolkit controls. File uses the same small text size as popup menus. The status bar places File, FFT and Range in one compact group with shared gaps and separator insets, keeping equal external space around their standard control surfaces without changing other status readouts. It has a combined file group and the analysis settings hint (#108), with compact duration units in the bar and h:mm:ss.mmm in the file hint. The worker reports file bytes and the reader's optional `original_sample_units` affine conversion (including gain and codec bit depth); whole-current-capture Peak and RMS use centered stored amplitude, including current capture gain (IQ RMS counts pairs). A per-source statistics tree of at most 8192 32-byte nodes is accumulated only by the sequential minimap scan, independently of edit versions. Background queries merge complete cells and read partial piece boundaries with a bounded cache; current SourceFile encoding resolves computed-source units. Source stamps, retained source scratches and document/capture signatures protect publication. Pending or invalid metrics never fall back to channel extrema. Settings and validation live in toolkit-neutral `settings.rs`; `shell/settings_ui/mod.rs` renders the status and explicit information panels; `shell/settings_ui/editor.rs` owns the editor the Analysis settings panel shows: standard selects and inputs drawn with `appearance(false)` in the hint's look. Ctrl+, (Cmd+, on macOS), View → Analysis settings… and a left click on the FFT summary open the panel; Ctrl+, again closes it keeping the values. Tab/Shift+Tab traverse its values and Reset to defaults, Enter or the arrows open a focused list, the arrows step a focused number, and Escape closes an open list before reverting. Ctrl+R (Cmd+R on macOS) applies available range advice, restores the full range from an explicit range when no advice remains, and does nothing on the full or automatic range, from the main window or the hint. The shared `numeric_editor::NumericEditor` owns the standard `Input`, standard `Button` steppers, configured units/steps/bounds, locale commit formatting and Up/Down handling for both property panels. Unlike click-only `NumberInput` steppers, its steppers act at the press and repeat every 80 ms after 400 ms while held (the retained owner-approved press/repeat exception, `NumericEditor::start_repeat`). Numeric edits commit on Enter, blur or a step; Analysis previews the commit and File information retains a conversion draft. Enter in a number applies it and keeps the hint open, an unusable number shows its error, and Enter elsewhere closes the hint keeping the values, because the editor handles `Confirm` before the popover does. The editor observes its owner so recommendations, effective values and the pending state refresh while it remains open. `document.rs` supplies toolkit-neutral hint titles, current values and optional explanations; `shell/mod.rs` preserves heading typography, renders aligned file-hint values with muted labels, and uses smaller muted explanations of one sentence without terminal periods. File and Analysis property panels use a shared font-relative, viewport-bounded width and aligned label columns. The numeric input budget follows the same scaled label, padding and gap sizes so Reference frequency stays on one line without crowding its value. Other metadata hints measure their definite width with GPUI using the rendered font sizes and weights; keep text blocks non-shrinking so wrapped text contributes its full height. The ready-state timing hint describes the latest analysis including sample reads, transforms and CPU image preparation, excluding file opening and window drawing; it is absent for other states. The whole ready status and its timing hint are transient and disappear on a mouse button press, a key press, a wheel event or the cursor readout appearing. Pointer motion outside the readout area, activation and focus changes do not dismiss them. Each accepted ready update resets dismissal without discarding the elapsed time. Window-level capture listeners observe mouse presses and wheel events, and a retained keystroke subscription covers bound actions with direct dismissal in the two existing key interceptors. Dismissal never consumes input. The cursor readout shows frequency before time and places the level in its own status-bar field. No status group is shown before a file is open. Cursor coordinates share the Alt badge formatter, following the time-ruler mode, frequency-axis unit and device-pixel precision. The minimap uses full-capture time and sample extents.

The analysis settings hint selects Peak (MAX) or Mean power, including subsequent files; saved effective settings override configuration defaults after a restart. The current session persists transform/palette settings, the time-ruler format, grid visibility, orientation, scale-control visibility and interface theme. `session::VERSION` and `session::READABLE` define the written and supported versions; legacy dynamic range is ignored. Range and its mode reset to configuration defaults on file opening. Changes in the hint preview without altering saved session settings; a keeping close (click outside, Enter, Ctrl+, again, F10, Ctrl+O, opening a file) saves them, while Escape restores the settings, time view and frequency view the hint opened with (`HintOpening`). Reset to defaults, below the values, applies the configuration values and keeps the hint open. Window and colour-scheme names are title-cased in the interface (`editor::display_name`, which parses back unchanged), and each value looks its own width, right-aligned, with lists opening wider than any value they offer. Changes use the same cancellable worker and retain the old picture until the new one is complete (see [Time navigation (#30)](navigation-selection.md#time-navigation-30): every analysis after a file's first is a final-only replacement). Mean power accumulates squared, tone-calibrated spectral amplitudes in cached bin/frame cells and weights their overlap with display pixels before taking dB; it is not integrated band power. Temporal power sums use f64 and count every frame once, including preview frames. CLI `mean` retains its legacy dB average after frequency MAX. Neither reducer changes the waveform envelope, PSD, transform lattice, or normalization. The displayed settings are tracked separately from requested settings so pending work cannot mislabel the retained picture. The recommended range is the same measured value as the CLI; the GUI warning specifically requires a nonzero signal whose spectral peak is in the lower half of the absolute full-scale colour window. It is suppressed for peak-relative/auto modes; obsolete or failed deliveries never create a warning, while a failed re-analysis retains the presentation of the last accepted picture without an action. A narrower recommendation alone is not a warning. The transform summary is the settings button and owns the analysis hint. The range is a separate hinted status item with warned, corrected and full states. Its effective number, requested mode and recommendation are derived together when the accepted analysis snapshot enters the document, so the displayed state never combines parts of different updates. Warned and corrected states use the same default cursor, hover background and shared toggle action as Ctrl+R / Cmd+R; full and automatic ranges are informational only. The action retains the stricter requested-settings check. While the displayed and requested settings differ, or while re-analysis is failed, the level keeps the displayed appearance but offers no action, keycap or hover affordance.

## About Argand (#107)

A permanent standard ghost tag-and-version button at the right end of the title bar before
window controls and a direct About Argand application-menu item open one retained
`PinnedHint`, anchored beneath the right title-bar button toward the window interior
and styled like File information. There is no command shortcut
and hover only shows the build-identity tooltip. `shell/about_ui.rs` composes the
existing pinned helper, standard Button and keyboard-accessible Base Link. The
repository Link opens its URL through the standard control; neither Link activation
nor prose/empty-space clicks add a special panel dismissal. About uses the same
information-panel owner, native left trigger, repeated-entry toggle, outside
backdrop, Escape and window-deactivation policy as File/Analysis. Ordinary focus
changes add no About-specific dismissal. Opening another application panel/menu
closes About, and one shared generation invalidates deferred open requests.

`about.rs` reads the version, description, licence and repository from Cargo
compile-time metadata. `build.rs` captures the short HEAD hash, commit timestamp
in ISO 8601 with offset and an exact HEAD tag (`untagged` otherwise), with explicit
`unavailable` values when Git identity cannot be obtained. The displayed time is
the commit time, never the build time. No Git process runs at application startup.
Build-script inputs cover the application's sources and assets, its owned
workspace library sources, their manifests and the workspace manifest/lockfile.
Git-only changes do not invalidate the build. Metadata is a snapshot from the last
build-script execution; a cached binary retains that snapshot. This is not a
callback on every compiler invocation. A source archive without its own `.git`
never borrows an enclosing repository's identity.
The tooltip contains tag, hash and commit date; the panel adds package details
and the existing approved application artwork.

## Theme

The default `theme = "system"` follows GPUI's OS appearance at startup and through a retained main-window appearance subscription. Theme changes refresh all windows. Explicit `dark` / `light` stay fixed; interface theme does not change the spectral palette. View → Theme chooses System, Light or Dark (#142); the choice is kept in the session (version 11, `Session::theme`) and, while the session holds none, `argand.toml` decides. The menu never rewrites `argand.toml`.

Interface colours (#123, #140, #151) live in `theme.rs`, which derives Argand's light and dark `ThemeConfig` from the toolkit defaults and installs both once at start-up, so every later `Theme::change` applies them. Surfaces step up from the window to the title and status bars (`status_bar` token), the settings sheet (`theme::sheet`) and the popovers (hints, menus, lists), so each stands apart from what it lies over; the menu and list row under the pointer uses a raised `accent`. Do not set a surface colour at a call site that the theme can carry. The title bar is muted while the window is inactive on every platform (`chrome::TitlePalette`, also over the stock `TitleBar`), and the Linux caption icons brighten under the pointer and dim while held. Buttons whose ink must follow the pointer (the settings steppers and Reset to defaults) draw it through `pointer_ink`, because a custom button variant has one foreground for every state.

## Plot ownership (#128)

`shell/plot/plot_view.rs` defines `PlotView`, one entity per document created when the file
describes itself and dropped with it. It owns the plot focus handle and the
`Plot Horizontal`/`Plot Vertical` key context on its surface element. It also owns
pointer and readout position, pan and frequency-pan state, measured
geometry and badge metrics, and the ruler context menu. Focusable overlays are
siblings of the surface, never descendants, so they inherit no plot bindings.
Navigation actions and gestures become `PlotIntent`s that Shell handles in one
subscription. Shell validates views against the capture and keeps view ranges,
tick schemes, analysis requests, session writes and settings rollback. Session
commands (grid, scale controls, orientation, ruler mode) stay Shell handlers,
reached by bubbling from plot focus. While a drag is active, a tracker with its
own hitbox matching the plot's handles every move the plot's hitbox does not see:
beyond the plot and over a hint lying on it (#129). The plot's own listener
handles the rest, so no move is handled twice.

Shell remains the only texture owner. It rebuilds an immutable `PlotSnapshot` of
shared references in every render and gives it to the plot. `shell::retire` is the
only place images are released: it clears the plot's snapshot, then schedules the
two-callback release on the originating window. Backdrop and deep-preview code
return the images they drop rather than releasing them.

While a document is shown, the plot owns keyboard focus. Toolbar and status-bar
buttons are not tab stops, and their clicks never take focus. Menus and popups
take focus temporarily and return it through `Shell::focus_target`, which the
application menu also uses to resolve keycaps. Headless GPUI tests
(`gpui-kit` `test-support`, dev-only) cover key routing, focus isolation, window
isolation, plot replacement, retirement and drags beyond the plot.

## Overlay surfaces (#129)

Surfaces over the plot own the input they cover through toolkit layering; the
plot never learns which overlays are open or where. It decides its cursor,
readout and Alt guides from its own hitbox hover state, and its handlers are
hover-based, so any blocking layer above it wins.

- Passive hints: `shortcut_tooltip`, `metadata_tooltip` and `unit_tooltip`
  return their view through `hints::passive`, never a bare `Tooltip::build`. A
  passive hint (`.tooltip`) lives only while its trigger is hovered, so the
  pointer can be inside it only over the trigger, which keeps the pointer and its
  clicks. Blocking it would take clicks from its own trigger, because a GPUI
  tooltip opens 13 pixels from the pointer and can cover the trigger.
- The analysis hint is a pinned hint (`hints::PinnedHint`) and the only analysis
  settings surface (#108). It is a standard gpui-component `Popover` anchored above
  the summary, drawn in the `Tooltip` look (`appearance(false)`); a right click
  toggles it, and a left click, Ctrl+, and View → Analysis settings… open it with
  the keyboard in it. Hover only shows informative help and the registered shortcut;
  it never opens an information panel. Moving the
  pointer away does not close it. Only a press outside (taken by the backdrop), Enter (keeps
  the values), Escape (reverts to the settings and views at opening), Ctrl+, again,
  F10, Ctrl+O and opening a file close it. `PinnedHint` holds any view; Shell opens
  it through `window.defer`, because the editor reads the shell as it is built. While it is open the plot is frozen: a transparent deferred
  backdrop (`hints::backdrop`) covers what the frame's resize regions leave of the
  window (`chrome::overlay_area`, #170, #226), below the title bar, so the plot gets no pointer,
  readout, Alt guides, clicks or wheel, and the popover holds keyboard focus,
  so `Plot` bindings do not match. Beyond that cover the title bar and the resize edges
  stay live and close the overlay themselves, so a press inside a hint, list or
  submenu lying over them is never taken for an outside press: the title bar closes
  the hint in its capture phase (only when its own hitbox is hit), and
  the Root `FrameObserver` observes the standard frame's bubbled press, which
  closes the hint keeping its values and dismisses the application menu. Space, which the
  popover would treat as Enter, does nothing (`NoAction` in the `PinnedHint`
  key context). The keyboard (#169): the first frame puts it on FFT size; Tab and
  Shift+Tab go round the hint's own stops through the window's tab order
  (`Editor::traverse`, bounded by the editor's own focus scope, because the plot
  surface is a tab stop too): the five lists, the numbers and Reset to defaults, whose
  own focus place (`defaults_focus`) takes Enter. A focused list opens on Enter, Up
  or Down through the stock `Select` (gpui-base binds them in its `Select` context);
  the list's arrows walk it, Enter chooses and Escape closes only the list. Up and Down
  on a focused number step it (`PreviousValue` and `NextValue` in the editor's context).
  Enter elsewhere closes the hint keeping the values. A focused value's underline turns
  solid and two pixels thick in the toolbar accent, and Reset gets an accent frame. Values changed from
  it preview live. `PinnedHint` emits `Pinned::Opened` / `Pinned::Closed { revert }`;
  Shell interrupts the plot on opening and restores `hint_opening` on a reverting
  close. Each list is as wide as its widest name, so its popup keeps its place
  while the keyboard walks it, but it shows its value at the right beside the
  chevron (`Name::display_title`) with the dashed underline only under that value,
  so every value looks content-sized and right-aligned. The heading line shows the pending state
  beside the title, or an error in the title's place, so the hint keeps one height
  with no reserved empty line. A warned range is shown in the hint's own
  Range row and nowhere else: the row reads `110 dB ⚠` in the warning colour, and
  a balloon (`Editor::range_readout`) carries the range item's warning text, the
  Ctrl+R keycap and an × that hides it until the hint closes, edged in the warning
  colour, with a pointer (`balloon_pointer`, a canvas path) towards the Range row.
  The editor measures its own bounds and the value's each frame (`Drawn`), and
  `balloon_side` places the balloon from the last measurement (#171): right of the
  hint, its tip at the sign and its text wrapped into the room left before the window
  edge; left of the hint once that room is under 220 pixels; not at all without room
  on either side. The balloon is hidden while any list of the hint is open (#168),
  told by a `SelectState` answering another focus handle than its closed one. While
  the balloon is closed or hidden for room, the value carries the same text as an
  ordinary hover hint. It is
  deferred above `POPUP_PRIORITY`, because the hint's popover would otherwise be
  painted over it. While Ctrl+R can act, the value carries the dashed underline
  in the warning colour, and a click on it or on the balloon applies the
  recommended range, which removes the balloon and keeps the hint. The popover is
  `overlay_closable(false)` and the backdrop closes the hint on any press, keeping
  its values, so a press on the balloon outside the panel is not an outside press. The status bar's range item is the same with the hint open or
  closed.
- The pointer over the plot: `time-plot` uses `on_hover` with
  `HoverListenerMode::InputModalityIndependent`. GPUI's default mode ends hover
  on every key press until the mouse moves, which cleared the readout and the
  crosshair on the first key. `on_hover` also fires when a layout change moves
  the plot under a still pointer: a file opened from the start page, or a hint
  or menu closed by a key. `PlotView::pick_up_pointer` then takes the pointer
  up, but only while `PlotSnapshot::pointer_in_window` holds. GPUI keeps the
  last position and hit test after `MouseExited`, and `is_window_hovered` means
  "active" on macOS, so Shell follows window-level `MouseMove` and
  `MouseExited` itself (`pointer_presence`) and clears the plot's pointer when
  the mouse leaves. An orientation change keeps the pointer and the old
  geometry until the next frame measures the new one, so the cursor does not
  flash to an arrow and the readout does not blank; that layout filters the
  pointer through the new geometry (`plot_pointer`).
- The application sets `CursorHideMode::Never`: GPUI's default hides the mouse
  pointer on Tab and on every key bound to an action, and on the plot the
  pointer is the working tool.
- The corner zoom buttons suppress the crosshair, the Alt guides and the
  status-bar readout (`PlotGeometry::over_scale_buttons`).
- Menus: the application menu and the ruler `PopupMenu` `occlude()`, taking wheel,
  clicks and drags. The application menu additionally draws a backdrop at a lower
  priority than its popover while it is open, so an outside click dismisses the
  menu and reaches nothing beneath it, and an outside wheel reaches nothing and
  leaves the menu open.
- Gestures: `PlotView::interrupt` ends drags, the ruler menu and the pointer.
  Shell calls it before the application menu, the settings hint and the file
  chooser open. Opening the ruler menu and deactivating the window call
  `PlotView::end_gestures`, which keeps the pointer.
- Tests cover the pinned hint's lifecycle and isolation, a passive hint's
  trigger click, a drag crossing a blocking layer, a menu-like occluding layer
  and interruption. The toolkit context-menu retention cycle (#144) is fixed in 0.9.1; native
  compositor behavior remains distinct from headless input dispatch.

## Application menu and toolbar (#91, #99)

File contains Open file, availability-filtered recent rows directly in the menu,
and Settings (`EditAnalysis`). Empty recent lists leave one separator between
Open and Settings, and a long name truncates in the middle rather than wrapping. The first ten recent rows carry digit keycaps, 1 to 9 and then 0, and pressing
that digit while the File branch holds the keyboard activates the row. View
appears with and without a file and
contains grid, scale
controls, orientation, Fit time, Fit frequency (both disabled without a file), Time scale format and Theme; pan/zoom
commands remain available through keys and gestures rather than menu rows.
With a document open, the toolbar's BB toggle beside Grid and Ctrl+B in the plot
invoke the same Baseband frequencies action as View. The pressed state reflects
the saved presentation preference; the tooltip names the next action and shortcut.
The menu is the toolkit's own `PopupMenu`, drawn by a controlled `Popover` on the
application button, whose tracked focus handle is the menu's own so that the
popover's open leaves the keyboard inside the menu.
`shell/plot/plot_ui.rs` draws translucent `[+|-]` zoom pairs over the spectrogram itself,
eight logical pixels clear of the picture's edges (`corner_zones`). Each pair
runs along the ruler of the axis it zooms (#150): a row in the bottom-left
corner beside the bottom ruler, a column in the top-right corner beside the
right ruler. Horizontal mode puts time bottom-left and frequency top-right;
vertical mode swaps them. Each pair is one framed container with
rounded corners (22-pixel squares, one-pixel divider) whose halves are
gpui-component `Button`s dispatching the registered zoom actions with shortcut
tooltips. Each half rounds the two corners facing away from its pair to the
frame's radius less its border, because the frame clips content to a rectangle
and a square hovered half would paint outside the rounded frame. View → Show scale controls
(`ToggleScaleUi`, Ctrl+U, session version 10, default on) hides or shows
every pair; pairs vanish when either spectrum side is too small. Pair zones use
an arrow cursor and exclude pan, drag and wheel navigation.

The application icon and Argand label form one button that opens the cascading File / View menu; F10 opens the same menu
from the shell, and both go through one open and dismiss pair that returns focus
to `Shell::focus_target`. `app_menu.rs` owns the toolkit-neutral recent order and
its digits (`file_items`). `shell/app_menu_ui.rs` builds the stock `PopupMenu` entities
and their rows, and the popover only draws them. The popover's own gesture opens
the menu on the button's press and reports it through `on_open_change`; the button
carries no handler of its own. Escape closes the whole chain, outside clicks close
it, Home, End and Space do nothing in the menu, and Enter or Right on File, View
or Time scale format opens that branch; Space remains inert. The owner approved
this standard submenu activation during #226. F10, Tab and the File rows'
digits are bound in an `ApplicationMenu` key context that wraps the menu, and the
digits answer only while the File submenu entity holds the keyboard. The stock
menu draws a submenu only while its row is selected, and a hover moves that
selection, so while the menu is open the shell holds a `Context::on_focus_lost`
subscription that focuses the open menu's handle again whenever a branch holding
the keyboard leaves the frame, and drops it on dismissal. While the
menu is open a backdrop drawn below the popover's priority covers the window
beneath it, so an outside click dismisses the menu and reaches no plot gesture, no
title drag and no control, while an outside wheel reaches nothing and leaves the
menu open. It covers only `chrome::overlay_area`, so the frame's resize edges keep
working, and a press on them also dismisses the menu through the Root frame observer (#170, #226). The independent ruler context menu
retains the stock PopupMenu. Toolbar buttons share the grid and orientation actions and persistence.
Orientation is a two-segment control in one frame whose selected segment is the
mode in force and whose tooltip names its own mode, and the segments and the grid
toggle appear only while a document is shown (#130). The segments carry the
Lucide `panel-top` and `panel-left` artwork, which the toolkit's default bundle
serves for the second and `assets.rs` embeds for the first with `icon_assets!`.
`assets.rs` adds embedded application artwork to the toolkit icons.
Title-bar content centers the shrinking filename region on the full window with
symmetric margins that include the toolbar and native controls;
interactive controls consume drag and double-click gestures.

The centered client title contains only the filename. Set the native window title
explicitly at startup and on each file opening: `Argand` or `filename – Argand`,
so application switchers and window managers receive the same identity.

## Standard buttons (#130)

The corner zoom pairs use standard div anchors in the current frame's measured
plot rectangle. The existing canvas lays out and prepaints their standard elements
before painting them above its marks; deferred gesture geometry never positions
the buttons. Their visibility and gesture zones share the same corner constants.

The corner zoom halves and the toolbar are gpui-component `Button`s, and each
keeps the toolkit's own pressed, disabled and click behaviour. A control that
lives over the picture or in the title bar is `tab_stop(false)` and carries a stable
`id`. The zoom halves, Grid and the orientation segments hand keyboard focus back
to the plot on a click; the application button opens its menu, which takes focus.
A custom variant paints no border, so these controls carry none:
`toolbar_style` takes a `ToolbarState` and gives one control its whole surface.
`Off` is the resting surface with the accent hover and pressed shades on top,
and tints the glyph while hovered. `On`, which is a Grid that is shown and the
application button while its menu is open, is the accent at 0.30 with 0.40 under
the pointer and 0.52 while held, and an accent glyph. The button carries that
0.30 surface itself (`on_surface`), because a custom variant paints its resting
colour at a fifth of its opacity. `Selected`, which is the
orientation segment in force, is the accent at 0.30 in every state with an accent
glyph, because a selected button is painted with its variant's active colour; it
is inert, because clicking the mode in force is not an action. The application
button and the grid toggle are not `selected`, because the toolkit stops
repainting a selected button's hover and pressed surfaces. The two orientation
segments share one frame on the group, a one-pixel border in the opaque blend
of `foreground.opacity(0.24)` over the title bar, with a six-pixel radius, and
each segment rounds its own outer corners to the radius the frame leaves inside, since GPUI clips content
to rectangles. Their divider is painted
inside the second segment rather than a border, because a border takes the colour
of the states its button passes through. The status-bar FFT item is a `Button`,
and the range item is one only while it has an action and a `div` otherwise.
Both follow the pointer through the button's own hover background and a hover
group on their text, with no hovered flag in the shell, and the FFT summary stays
lit while its pinned hint is open, which keeps the pointer off the window. The
yellow range progression and the accepted colours are unchanged. The audit
dispositions are recorded in the inventory table of
[completed UI architecture plan](../plans/completed/124-standard-ui-architecture.md), one line per control, with the
audit itself, every render and input entry point and each retained row's
verified limitation in the locked 0.9.1 sources in [control inventory](../ui/124-inventory.md).

## Recent files and opening

The start page and the inline recent rows in File share `recent.rs`'s toolkit-neutral availability snapshot, displaying only verified regular files in recent order. The shared snapshot excludes the currently loaded path, recorded only after a successful open and normalized with the same function as session history. Filtering preserves saved entries and opening hints, and the previous file returns in recent order when another file opens. Availability refreshes on startup (including a file argument), successful file opening, window activation and File-menu opening. Each refresh considers at most ten saved entries and coalesces already pending paths. Distinct new paths are checked independently even when old paths are blocked; there is no global worker cap that could let old network mounts starve the current history. Results identify paths, so reordered history cannot misapply a result or restore old opening hints. The menu takes the latest completed snapshot when opened and keeps its entries stable until dismissal; pending availability changes appear on reopening. Both numbered start-page links and Alt+1 through Alt+9, then Alt+0 for the tenth (#216, `app_menu::digit`), use the same filtered order. These shortcuts apply only while the start page is visible. The shell owns the initial focus handle; File returns focus through `Shell::focus_target`, which is the plot while one is shown and the shell handle otherwise. The result receiver lives with the shell, across file openings; blocked filesystem checks are never joined on the UI thread or at shutdown. Failed checks do not delete history. Recent rows and the chooser size their hover backgrounds to the text, bounded by the list width; the chooser follows the list content height, with `or` aligned to the heading and chooser text aligned to the filename column. The complete start-page group is content-sized up to the viewport-aware width limit and centered horizontally and vertically in the available content area; only the recent list shrinks and scrolls in short windows. Empty history keeps the chooser centered. Ctrl+O (Cmd+O on macOS), File > Open and the start-page chooser dispatch one `ChooseFile` action. Its binding and application-level handler also cover popup focus; the handler dismisses the tracked title-bar popup and restores focus through `Shell::focus_target` before opening the chooser. All shortcut hints size to their content with a viewport-aware maximum and use `shortcut_tooltip`, which resolves the registered action binding and places it at the right edge in the same framed keycap used by analysis hints. A recent row's own hint is two lines, the file name over its containing directory in the smaller muted type; a menu row adds no hint. The application's menu action rows are element rows that draw `shortcuts::keycap` at the right edge, resolved from `Shell::focus_target`, because the stock menu's own keycaps are borderless and transparent; they keep the stock action, check and keyboard navigation. Recent rows are named by `recent::labels` on the start page and in File alike (#216): the file name alone where no other shown row shares it, otherwise with the nearest parent folders up to the first that tells the rows apart, the folders every one of them shares at that depth and those beyond it elided with `…`, and a path told apart only at its top shown from its root. A file name or shown folder longer than 20 characters keeps its first and last characters around `…`, 8 each at first, growing until every two different names or folders at one depth stay different. The start page hint names the bare file over its whole folder. Rows truncate in the middle so the extension stays visible. `shell/shortcuts.rs` applies shared, slightly stronger text and border contrast to `Kbd` and measures that same element for menu widths. Keep the original Edit settings geometry and typography: a text-colour override replaces the entire built-in text refinement, so the helper explicitly preserves its size, alignment, line height and whitespace defaults. Reserve its full width and keep platform notation from registered bindings. Keep every metadata check off the UI thread, including paths that look local but may be network mounts.

## File information and conversion (#189)

The upper metadata section includes a Reference frequency numeric editor (#185),
using the existing NumericEditor and whole Hz from 0 through u32::MAX. It rounds
on commit/step, leaving draft text and cursor untouched during typing. Numeric
input accepts edited space grouping in the integer part and normalizes it on
commit; decimal punctuation grouping remains strict. It includes zero and remains
physical when View > Baseband frequencies is checked. Valid Enter/blur/step
commits are undoable metadata edits; invalid text leaves the document unchanged
and shows an inline error. Escape from a changed reference draft restores its
last committed value; otherwise it follows the panel's existing dismissal.
Save/Save As writes the committed reference. Conversion's separate Carrier
frequency field retains its domain-conversion meaning and draft transaction.
Convert also accepts a committed reference change relative to the source file;
Save/rebind establishes the new baseline and Undo can restore the original.
The baseband View preference persists with other session display preferences.

`processing/conversion.rs` converts a whole immutable edited-capture snapshot into a new WAVE/RF64;
selection is ignored. `shell/conversion_ui.rs` composes the file information status Button,
PinnedHint panel, Input, Select and Convert Button. File > File information… and F11 open
the same panel. File and Analysis summaries share `settings_ui::status_control`;
both property editors share `settings_editor` row, numeric, Select and outlined-action
presentation over standard controls. Numeric initialization and Enter/blur formatting share
one event handler, while each editor keeps its own preview/draft transaction. One `hints::Panel` owner and shared trigger/popover composition route File,
Analysis and About through the same opening/closing lifecycle; choosing a new
owner closes the previous panel, and stale close events cannot clear it. On a real/IQ domain change, target sample rate and conversion carrier defaults are derived from the source: IQ to real doubles Fs with carrier FsIQ/2; real to IQ halves Fs with carrier FsReal/4. Subsequent numeric edits remain explicit; changing target Fs does not move the carrier. Drafts do not mutate the current document; Escape discards them.

`shell/property_advice.rs` shares measured callout placement, dismissal, standard close
Button and hover reopening of the same balloon between Range and automatic sample-format changes; stock hover fallback is used only without side room. Format
advice marks the selected value without an inline row, hides while a Select is open,
and resets dismissal on a new automatic change; explicit choices clear the warning.

The shared NumericEditor uses stock disabled controls and skips disabled numeric handles in its existing scoped Tab traversal.

See [conversion DSP and reference semantics](processing.md#capture-conversion-189)
and [conversion publication and saving](editing-io.md#conversion-publication-and-saving-189).

## Sample type consumers (#192)

The canonical twelve `SAMPLE_TYPE_TOKENS` include `rl_i24` and `iq_i24`. File
information and conversion share the existing standard sample-format Select and
show `Real · Int 24-bit` and `I/Q · Int 24-bit` for WAVE. FLAC filtering retains
only U8/I16 and its existing domain-preserving fallback. The document format hint
identifies signed 24-bit integers; raw opening hints, configuration/session tokens,
CLI help and reports use the same parsed core sample type. No new widget or input
handling is introduced. See the current [control inventory](../ui/124-inventory.md).

## Shared information-panel and tooltip policy (#246)

File information, Analysis settings and About Argand open explicitly through their
standard buttons, menu entries or existing shortcuts, taking normal Popover focus.
No panel hover timers, hover previews or additional toolbar Tab stops exist.
The standard Popover owns native left-button trigger activation and Confirm;
there is no separate right-button activation or per-button open callback. Repeating
any own trigger/menu/registered shortcut closes its panel; another trigger switches
the owner. A shared content backdrop and bubbling status-bar outside press handler
close panels while native trigger propagation keeps neighbor switches live. Internal
empty clicks and non-left presses do not add About-specific dismissal. File draft
and Analysis preview/keep/revert remain content transactions, not separate lifecycle
controllers.
File > File information… and F11 open the same properties/conversion panel; the
Convert button inside performs conversion. View > Analysis settings… uses Ctrl+,
(Cmd+, on macOS). Status-button tooltips describe the corresponding fields and
show the same registered shortcut. About retains its logo/branding and uses
About Argand as its accessible name and tooltip title.

`Shell::external_tooltips_allowed` supplies one per-window eligibility check to
external toolbar, plot, status and recent-file tooltip registrations. An open
information/selection panel or application menu removes those registrations;
deactivation also disables them and closes panels, invalidating deferred opens.
A suppression epoch is acknowledged from the existing pointer-observer canvas
after a frame has retired the native registrations, including deferred controls.
After that epoch is cleared, the active owning window may register fresh native
builders; an inactive window waits for activation. Fresh registration starts no
tooltip timer, so the first subsequent pointer motion can show normal help without
resurrecting old help under a resting pointer. Stale acknowledgements cannot
release a newer suppression.
The native GPUI tooltip lifecycle drops active/pending state when its builder is
removed on the next render. Re-enabling registration alone must not reveal stale
help under a resting pointer. Internal field help and automatic Range/format advice
retain their existing behavior. Standard Popover focus, nested Select dismissal,
Analysis keep/revert and File drafts remain the existing contracts.

File groups Open, optional standard Recent files submenu, Save commands and final
File information; Analysis settings is a separate final View group. Recent files
contains at most ten single-line rows and uses native submenu sizing, as do the
fixed command branches. Menus do not mount a scroll layer. Empty Recent is omitted
and one separator remains between Open and Save. The frozen recent snapshot and
1–9/0 shortcuts apply while File or Recent files holds focus. Permanent commands
remain drawn and unavailable commands are disabled. Analysis remains
document-dependent because its standard Popover is anchored to the document status
summary. At the supported 640×400 minimum, File information and all ten Recent rows
must remain pointer- and keyboard-reachable without unnecessary scrollbars.

Without a document, View's grid/scale/orientation/time-format commands remain
visible but disabled; unavailable Time scale format is a plain disabled item
without an expandable child. Theme remains available. Edit keycap labels resolve the
registered Plot bindings even for disabled empty-window commands, while native
menu dispatch keeps the actual focus target and existing command guards.

Caption Close targets the retained Shell action path without changing actual
focus. The menu backdrop reserves Linux's existing caption controls and Windows'
standard Close plus supported minimize/maximize controls, each at the toolkit's
TITLE_BAR_HEIGHT width. macOS adds no app-painted caption reservation. Toolbar
and drag surfaces remain blocked. Quit Argand and Ctrl+Q (Cmd+Q on macOS)
use the same window close request, including conversion cancellation and unsaved
Save/Discard/Cancel protection. The overlay shortcut fallback is registered for
its owning window, not an application-wide unconditional quit.

Before close confirmation, the existing close transaction dismisses nested
Select, menus and information panels with keep semantics so deferred overlays
cannot cover the unsaved dialog. Conversion/replacement guards remain in force.
