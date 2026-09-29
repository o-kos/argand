# Issue #124: Standard UI infrastructure and focused plot ownership

Resolves [#124](https://github.com/o-kos/argand/issues/124).

## Overview

Migrate to GPUI Kit's aligned GPUI 0.3.x stack, adopt its borderless `Root` around
the existing Argand frame, give the plot a dedicated focused entity, and replace
custom ordinary controls with toolkit components wherever their required behavior
can be preserved. This supersedes the initial stock-frame direction after #126
demonstrated functional frame and composition blockers in the locked stack.

This initial revision contains planning and policy only. No UI migration, native
interaction verification or dependency change has been performed by this revision.

**Where the plan stands.** The architecture is implemented. Phases 0 through 5 are
complete, phase 6 with [#133](133-ui-integration.md), which closed the
inventory, removed the #126 prototype, brought this document and `AGENTS.md` from
planned to implemented, and hands #108 its updated constraints. The owner's
decision of 2026-09-29 replaces the three-platform native matrix with Linux-only
evidence, gathered stage by stage and mapped in
[ui/124-integration/linux-results.md](../../ui/124-integration/linux-results.md).
The implementing and reviewing roles are those the repository's role policy
names per Pull Request, not the planning revision's; #133 used Codex
`gpt-6-sol` high as its reviewer.

Implementation class: **A**, declared before implementation: the refactor spans
more than five code files / roughly 400 lines and touches GUI interaction and
texture-lifecycle ownership boundaries. Implementer: `gpt-5.6-sol` high; reviewer:
`gpt-5.6-terra` high, fixed for the implementation review rounds. The initial
documentation-only revision is reviewed by `gpt-5.6-luna` high under the repository's
documentation-review rule; this does not downgrade the future implementation.

## Context

- Locked dependencies at planning time: GPUI 0.2.2, gpui-component 0.5.1.
- `Root::read/update` expect the window's top-level entity to be `Root`; wrapping
  only an input subtree does not satisfy that contract. `Root::render` always
  calls `window_border()` in this version; there is no exposed border-off option.
- GPUI Kit 0.6.x is the application facade for its aligned GPUI 0.3.x family. Its
  `Root::bordered(false)` separates Root's focus/input/overlay infrastructure from
  the Linux CSD wrapper. #137 owns this dependency/API migration before #127 uses
  borderless Root in production.
- `chrome.rs` owns the current client frame, resize regions and Linux controls.
  Its viewport-based resize behavior, expanded/tiled cases and rounded corners
  must be compared with the standard frame, not discarded without verification.
- `shell.rs` attaches `Plot` key contexts to the shell, stores plot gestures and
  pointer state, and assumes `window.root::<Shell>()` / `WindowHandle<Shell>` in
  input observation and action dispatch. These assumptions must migrate too.
- `plot_ui.rs` measures and paints the plot through a canvas; `spectrogram.rs`
  paints retained textures. This rendering strategy is valid and remains in use.
- `navigation_ui.rs` mixes toolkit-free navigation with shell interaction routing
  and a symbolic-key interceptor. Preserve needed keyboard normalization without
  allowing it to bypass an active editing surface or consume its text commands.
- `app_menu_ui.rs` already isolates its menu through focus, occlusion and stopped
  propagation. Keep that protection. The stock `PopupMenu::dismiss` recursively
  dismisses parents, unlike the accepted one-level Escape behavior (#91).
- `settings_editor.rs` already uses standard NumberInput and Select in a separate
  Root-backed window. Do not replace these with custom editors.
- #108 / PR #106 is blocked on this infrastructure. This decision supersedes its
  no-Root constraint, not its remaining product acceptance criteria. Do not reuse
  the rejected hand-written numeric editor or silently merge that branch.
- #122 documents hint/cursor isolation and explicitly preserves click/wheel
  behavior. #123 concerns hint contrast, which is not part of this refactor.

## Decisions

### Ownership

```text
Application window
  Root (border disabled; focus/input/overlay infrastructure)
    Argand frame (sole inset/resize/title-bar owner)
      Shell: document/analysis and GPU-resource ownership, title/status, window commands
        PlotView: plot focus, pointer/readout state, hitboxes and gesture lifecycle
          existing canvas paints Shell-supplied snapshots, axes, waveform and minimap
          standard zoom controls
    toolkit overlays: menus, hints and editing popovers
```

This is an ownership sketch, not a requirement to draw every overlay as a normal
child; use the toolkit's supported overlay placement and focus mechanisms.
Do not create a generic widget framework or one entity per drawn tick/primitive.

- Keep analysis workers, generations, view requests and accepted snapshots under
  document/application coordination. Plot interactions submit typed view intents;
  they do not duplicate analysis state or mutate the worker directly.
- Shell remains the sole upload/coalescing/retirement owner for foreground,
  backdrop and deep-preview textures. PlotView receives immutable render snapshots
  with shared image references; it never uploads, retires or caches another copy.
  Before scheduling retirement of an old image, Shell synchronously replaces or
  clears PlotView's old snapshot and invalidates its rendering, so a later frame
  cannot paint that image again. Preserve the existing two `on_next_frame` callbacks
  with the originating window. Removing/replacing PlotView cancels its gestures and
  drops its snapshot; Shell keeps resource ownership and performs any retirement.
  Window teardown keeps current window-bound cleanup, with no callbacks targeting
  a replacement window. Test replacement while old frames are still in flight.
- UI-only gesture/focus state moves to PlotView. Session persistence remains
  centralized; do not persist hover, focus or pressed state.
- Exactly one Argand frame owns decoration insets and resizing. Root supplies the
  standard focus, input and overlay infrastructure with its border disabled.
- Migrate through the crates.io `gpui-kit` facade; do not depend on `gpui-pre`
  directly or independently select GPUI/component/assets versions. If the aligned
  stack cannot meet functional requirements, stop with a reproducible limitation
  and seek an owner decision. No silent fork, cache edit or second frame.

### Root-to-Shell bridge

Create `Entity<Shell>` inside the window construction closure and give it to Root
as the child view; Root owns that child strongly. The window handle becomes
`WindowHandle<Root>`. Window/application callbacks retain `WeakEntity<Shell>` and
the originating typed window handle, never a second strong owner or a process-wide
Shell registry. Enter the captured window through its handle and update the weak
Shell there; a closed window or dropped Shell makes the callback a no-op.

Replace `window.root::<Shell>()` in ready-status observation with an explicitly
captured weak owner for that window. Register observers/actions once, not during
render; preserve intentional global commands and editor-local action precedence.
Test commands from plot, menu and settings-window focus, then after window closure,
to catch wrong-window dispatch, leaked ownership and duplicate handling.

### Event contracts

| Surface | Pointer / wheel | Keyboard and lifecycle |
| --- | --- | --- |
| Plot | Only its exposed hitboxes start navigation; established drags finish or cancel outside | Plot focus owns navigation; focus loss, document replacement and overlay activation end incompatible gestures |
| Main/context menu | Covered content cannot receive plot gestures; preserve existing outside-dismiss behavior | Menu focus owns navigation/Escape; restore previous valid focus on dismissal |
| Interactive popover / nested select | Controls receive gestures, not the plot behind; outside action is explicit | Own focus scope; text editing/clipboard/undo must not dispatch plot actions; close nested popup before its editor where applicable |
| Passive hint | Own arrow cursor; suppress covered plot readout/Alt guides; preserve #122 click/wheel compatibility | Showing a hint does not steal keyboard focus; leaving restores plot feedback without a second mouse move |
| Separate native window | Window-local input; no accidental forwarding to the main plot | Preserve existing window/editor transactions and native activation behavior |
| Ready-status observer | May observe capture-phase input without consuming it | May dismiss transient status; must never navigate or override the focused control |

Use toolkit hitboxes, occlusion, focus scopes and supported propagation controls;
do not add a plot-side list of open tooltip rectangles. `occlude()` blocks wheel
as well as other mouse interaction, so it is not automatically appropriate for
passive hints with the #122 compatibility requirement. Verify that distinction in
the prototype. Root adoption alone is not proof of input isolation.

Window/application commands must have explicit scope. Preserve intentional global
Open/Settings actions, but do not promote plot navigation to application-global
handlers to compensate for lost focus. Toolbar/menu commands should target the
active document deliberately, without bypassing editing-surface rules.

### Custom-control inventory and target disposition

Every retained entry needs its own verified limitation, supported alternatives,
maintenance/interaction responsibilities, tests and owner decision. A prior
implementation or the word "custom" is not a justification.

| Existing implementation | Standard candidate / target | Verification or exception decision |
| --- | --- | --- |
| Custom frame and Linux window controls (`chrome.rs`) | Borderless Root plus the existing Argand frame | **Retained**, the sole frame. The locked `window_border` measures resize from the reported `window_bounds()` rather than the viewport, and `TitleBar` always appends private window controls whose Linux maximize button zooms on every click, so a double click on it toggles twice; the production frame measures `viewport_size()` and owns its controls. [R1](../../ui/124-inventory.md#r1-custom-frame-and-linux-window-controls) |
| Title-bar drag and double-click handlers (`shell.rs`) | Existing Argand title-bar behavior | **Retained** for Linux, the composition of the stock `TitleBar`, which cannot be used because its private caption controls are unconditional. Controls still do not start a window move, and the accepted maximize double-click is unchanged. [R2](../../ui/124-inventory.md#r2-title-bar-drag-and-double-click-handlers) |
| Zoom halves and `pressed_zoom` (`plot_ui.rs`, shell routing) | Button with shared group styling | **Replaced (#130).** Each half is a `Button` in the shared frame: the component owns press, disabled and click, and `pressed_zoom`, `release_press` and the ToggleScaleUi repair are gone. Geometry, translucency, the enabled rule and the tooltips are unchanged; the halves are `tab_stop(false)`, keep their ids and hand focus back to the plot. The frame and its border stay a `div` because it is layout, not a control |
| Main cascading menu (`app_menu.rs`, `app_menu_ui.rs`) | PopupMenu and supported composition | **Replaced (#131).** One stock `PopupMenu` with the File, View and Time scale format submenus, drawn by a controlled `Popover` on the application button whose tracked focus handle is the menu's own, because the popover's open focuses that handle and would otherwise take the keyboard from the menu. Rows are stock items with stock checks and actions, and the recent captures are element rows that render their digit and name; the order and the first nine digits stay in the toolkit-neutral `app_menu::file_items`. Escape closes the whole chain, which the owner accepted in place of one level, Home, End and Space do nothing in the menu, because the stock key context has no bindings for them, and Enter or Space on File, View or Time scale format does not open that branch, because the stock `confirm` ignores a submenu row, so only Right enters one. F10, Tab and the File rows' digits come from an `ApplicationMenu` key context that wraps the menu, and the digits answer only while the File submenu entity holds the keyboard. A backdrop below the popover's priority keeps the dismissing click or wheel from the plot and the title bar. The popover anchors the menu to the button's top-left, which is the stock dropdown offset, and the popover's own press opens the menu, so the button carries no handler |
| Waveform/spectrum splitter (`shell.rs`, `panels.rs`) | Resizable panels/handle | **Removed (#132).** The owner decided the minimap is not resizable: it has a fixed height, or width in vertical orientation, set by `[panels].minimap_size` (default `"3 rem"`). The drag strip, its state, the fraction intent and `Session::waveform_fraction` are gone; an old session carrying the field loads and drops it. The standard `ResizablePanelGroup` is not used |
| Toolbar, status range/FFT, start/recent buttons | Existing Button | **Toolbar replaced, rest retained (#130).** Toolbar controls carry no border, because a custom variant paints none, and their states are the accent surfaces in `toolbar_style`: an on control keeps the accent shade, and hover and pressed are the accent surfaces on top of it. The application button and the grid toggle are not `selected`, so they keep those hover and pressed states while they are on, and the grid glyph and the segment that changes the mode tint while hovered, while the application button's artwork does not. Orientation is a `ButtonGroup` of two segments in one frame, whose selected segment is the mode in force, is inert while selected, and names its own mode in its hint, with the Ctrl+T keycap on the unselected segment only; the divider is painted inside the second segment, because a border takes the colour of the states its own variant passes through, and each segment rounds its own outer corners, because GPUI clips to rectangles. The on state is accent 0.30 with 0.40 under the pointer and 0.52 while held. The segments and the grid toggle exist only with a document, so the title reserves exactly what is drawn. Status range/FFT keep their accepted colours and their manual hover foreground and background, because the painted hover state is that same colour, and the FFT summary additionally stays lit while its pinned hint is open. Start and recent rows keep the ghost variant, measured text-width hover surfaces and numbered shortcuts. The toolbar and zoom controls are `tab_stop(false)` and return focus to their owner on a click |
| Settings form | Existing NumberInput / Select / Button | **Retained, already standard.** Reset, Cancel and OK are standard outline and primary Buttons and the recommendation a standard ghost Button, with no custom press, hover or focus machinery. In-window compatibility is proved headlessly by `settings_editor::standard_input_tests`, which types in a `NumberInput` and chooses and dismisses a `Select` under a standard `Root`. [R7](../../ui/124-inventory.md#r7-settings-form) |
| Ruler context menu | Existing PopupMenu | **Retained, already standard.** A stock `PopupMenu`; only the entity tracking for its retained popup and its `DismissEvent` is Argand's, which is lifecycle, not a control. [R8](../../ui/124-inventory.md#r8-ruler-context-menu) |
| Metadata/analysis/shortcut hints and keycaps | Existing Tooltip / Kbd | **Retained, already standard.** Stock `Tooltip` and `Popover` behind one contract, and `Kbd` with a shared refinement and a measured width. Two binding facts are met in code: the popover binds Space to confirm, and `occlude()` also blocks the wheel. [R9](../../ui/124-inventory.md#r9-metadata-analysis-and-shortcut-hints-and-keycaps) |
| Ready-input capture canvas (`shell.rs`) | GPUI window event observation | **Retained** as a minimal passive adapter, and a canvas is required: `Window::on_mouse_event` is the only window-level mouse registration and is paint-phase only. Keys use the supported app-level observer. Dismissal never consumes or navigates. [R10](../../ui/124-inventory.md#r10-ready-input-capture-canvas) |
| Global symbolic-key interceptor (`navigation_ui.rs`) | Plot-scoped bindings/key handling | **Retained** as an adapter, one per `PlotView`, dropped with the plot and gated by window id and exact focus before it recognizes or consumes anything. `App::intercept_keystrokes` is the only pre-binding hook and is application-global. [R11](../../ui/124-inventory.md#r11-global-symbolic-key-interceptor) |
| Spectrogram, waveform, axes, minimap and cursor badges | Existing domain canvas/texture rendering | **Retained** domain drawing. The toolkit's `PlotAxis` and `Grid` take pre-computed pixels and caller-supplied labels and derive no ladder in physical units, no edge-mark policy, no locale formatting and no measured gutter. The audit found no ordinary control inside it. [R12](../../ui/124-inventory.md#r12-spectrogram-waveform-axes-minimap-and-cursor-badges) |
| Rejected numeric editor in PR #106 | NumberInput / Input | **Not ported.** The branch is unmerged and the editor rejected; #108 restarts on the standard `NumberInput` and `Input` the settings form already uses. [R13](../../ui/124-inventory.md#r13-rejected-numeric-editor-in-pr-106) |

Every row now carries its final disposition. The audit that closed it, with every
`Render` implementation, control constructor, `on_mouse*`, `on_scroll*`, `on_key*`,
`on_action`, `intercept_keystrokes` and window event registration in `crates/app/src`
mapped to a row, is [ui/124-inventory.md](../../ui/124-inventory.md). Infrastructure
observers and adapters are recorded there separately from ordinary controls. Each
retained row's verified limitation is cited to the locked 0.6.6 sources, not to a
prior decision.

Domain-drawing justification: the existing GPUI canvas/image primitives are the
standard foundation to retain, not an alternative being bypassed. Button, Tooltip
and Resizable do not implement calibrated I/Q textures, physical tick placement,
waveform envelopes or capture minimaps. Argand owns physical coordinate mapping,
sampling and overlay geometry; existing axes/navigation/render tests plus the
orientation, cursor and resize native cases cover those responsibilities. The
approved architecture retains this domain drawing; any newly discovered ordinary
control within it still needs replacement or a separately accepted exception.

## Rejected alternatives

- Keep Shell unwrapped and indefinitely retain separate settings windows: does not
  remove the infrastructure obstacle to #108 or reduce duplicated control behavior.
- Nest Root under Shell or around one input: does not satisfy the top-level lookup.
- Use the locked Root around the existing frame: its mandatory border doubles frame
  ownership. GPUI Kit's supported `Root::bordered(false)` is the selected solution.
- Replace the accepted Argand frame with the stock component frame: #126 demonstrated
  resize cursor/hit-region and bare-title regressions, so it failed the owner gate.
- Hand-write ordinary controls to avoid Root or styling constraints: takes over
  text/focus/gesture contracts unnecessarily and repeats the #106 failure.
- Rewrite DSP or introduce a custom GPU backend: unrelated to interaction ownership.
- Replace every custom drawing with a stock widget indiscriminately: domain
  visualization is legitimate; required behavior, not a numerical replacement quota,
  decides whether an ordinary control can be replaced.

## Implementation steps

### 0. Planning and rules (this revision only)

- [x] Create architecture issue #124 and record the owner's option-2 decision.
- [x] Add standard-controls-first rules and per-exception justification requirements.
- [x] Publish this versioned implementation plan with an initial inventory and class.

### 1. Compatibility checkpoint before broad migration

- [x] Capture the current native interaction/geometry baseline and finish the inventory.
      (#126, PR #134; [completed/126-ui-compatibility.md](126-ui-compatibility.md)
      and [ui/126-compatibility/](../../ui/126-compatibility/), whose `inventory.md` maps every
      production control and whose `native-results.md` records the baseline build hashes,
      the Wayland/Sway run and its display scale.)
- [x] Build a bounded verification fixture using top-level Root, its frame, a plot
      canvas, standard numeric input/select in a popover, a menu and a passive hint.
      (#126, PR #134; the fixture was `crates/app/examples/ui_compatibility.rs` with
      `ui_compatibility/model.rs`, and #133 removed it in favour of
      `settings_editor::standard_input_tests`. See
      [ui/124-inventory.md](../../ui/124-inventory.md), "Removed verification surface".)
- [x] Verify frame operations and the event matrix, including the stock menu Escape
      mismatch and passive-hint pointer/wheel distinction. Inspect locked toolkit
      APIs before choosing any replacement or exception.
      (#126, PR #134; the `Select`-inside-`Popover` deferred-draw panic is reproduced
      step by step in [ui/126-compatibility/native-results.md](../../ui/126-compatibility/native-results.md),
      and the `occlude()` wheel behaviour that separates a passive hint from a blocking
      one is a locked-toolkit observation in
      [ui/124-inventory.md](../../ui/124-inventory.md).)
- [x] Present native evidence and remaining limitations. Do not start broad migration
      if functional frame requirements fail; obtain decisions on any UX tradeoffs.
      (#126, PR #134; the recorded verdict is a gate termination, and the owner decided
      on 2026-09-22 to stop the checkpoint there rather than make a frame requirement
      fail the migration. That decision is the gate-termination step in
      [completed/126-ui-compatibility.md](126-ui-compatibility.md).)

### 2. GPUI Kit / GPUI 0.3.x migration

- [x] Replace the independently versioned GUI dependencies with the crates.io
      `gpui-kit` facade and its aligned GPUI 0.3.x graph; commit `Cargo.lock`.
      (#137, PR #138; the workspace declares only `gpui-kit = "0.6"` in `Cargo.toml`,
      `crates/app/Cargo.toml` resolves it from the workspace, and `Cargo.lock` pins the
      graph. See [completed/137-gpui-kit-migration.md](137-gpui-kit-migration.md).)
- [x] Adapt startup, focus, painting, assets and changed component APIs while keeping
      the production main window rooted directly at Shell and preserving behavior.
      (#137, PR #138; startup is `gpui_kit::application()` with `gpui_kit::init(cx)` in
      `shell.rs`, and `Shell` remained the production root until #127 put a borderless
      `Root` above it in PR #139.)
- [x] Migrate the #126 fixture and prove borderless Root, standard input/select,
      overlays and the Argand frame composition without a second frame layer.
      (#137, PR #138; the fixture's borderless window wrapped representative content in
      `chrome::Frame` so one native run exercised its border, shadow, inset and resize
      hitboxes. #127 then adopted that composition natively, and #133 replaced the
      fixture with `settings_editor::standard_input_tests`.)
- [x] Repeat the local gates, release build, dependency/license inventory,
      representative performance checks and native/three-platform validation in #137.
      The local gate, the release build and the license inventory are recorded in
      [completed/137-gpui-kit-migration.md](137-gpui-kit-migration.md). The
      production borderless-`Root` window was verified natively on Linux by #127
      (PR #139; `native_decorations_do_not_get_a_second_frame` in `chrome_tests.rs` and
      the native rows in [ui/124-integration/linux-results.md](../../ui/124-integration/linux-results.md)).
      The three-platform native part is not exercised: the owner's Linux-only decision
      of 2026-09-29 replaces the parent plan's native matrix, and Windows and macOS are
      covered by `ci/full` builds and tests only.

### 3. Root and single-frame migration

- [x] Implement and test the explicit Root-to-Shell weak-owner bridge before changing
      root lookups or application command targets; verify closed-window no-ops.
      (#127, PR #139; `Shell::bind_choose_file` and the `analysis_hint_tests` cases in
      `shell.rs`.)
- [x] Adopt `Root::bordered(false)` in the main window while retaining `chrome.rs` as
      the sole frame, inset, resize and title-bar owner. (#127, PR #139;
      `native_decorations_do_not_get_a_second_frame` in `chrome_tests.rs`.)
- [x] Update typed window handles, Shell root lookups, theme/font setup, Tab traversal,
      ready-status observation and window-level actions without duplicate registration.
      (#127, PR #139; `theme_tests` and `analysis_hint_tests` in `shell.rs`.)
- [x] Preserve native title, activation, file chooser/drop behavior, session geometry
      and the existing Root-backed settings window. Verify a current release build.
      (#127, PR #139; the `session_tests.rs` round trip and the `settings_editor` window
      itself, whose root is a standard `Root`.)

### 4. Plot ownership and overlay isolation

- [x] Introduce PlotView with bounded render inputs and typed navigation intents;
      move plot focus, pointer/readout and gesture state out of Shell. (#128, PR #143;
      `plot_view.rs` tests.)
- [x] Route keyboard navigation through plot focus and commands through explicit
      targets; keep required symbolic-key normalization scoped to plot input. (#128,
      PR #143; `every_navigation_key_emits_exactly_one_intent` and
      `plot_keys_need_the_plot_itself_focused`.)
- [x] Remove or localize the global `cx.intercept_keystrokes` plot handler. If the
      toolkit only exposes a global registration, gate it on the active PlotView
      focus before matching, dispatching or stopping propagation; this is a scoped
      adapter, not a global navigation bypass. Verify focused Input/Select symbols,
      Control commands and IME remain untouched; test both top-row and keypad zoom.
      (#128, PR #143; `another_window_never_reaches_the_plot`,
      `a_replaced_plot_stops_intercepting`, and
      `zoom_symbols_use_physical_shift_without_leaking_into_time_zoom`.)
- [x] Preserve accepted snapshot labels, resize mailbox behavior and texture retirement.
      (#128, PR #143; `retirement_clears_the_snapshot_first` in `plot_view.rs`.)
- [x] Apply the surface contracts centrally and test dismissal, release outside,
      focus loss, overlay opening during drag and document replacement (#129,
      PR #155; see `completed/129-overlay-isolation.md`).
- [x] Verify #122's cursor/guide reproduction and compatibility requirements; link
      the evidence without claiming unrelated hint-contrast work is complete (PR
      #155 maps each #122 criterion to a test or native case; #123 stays open).

### 5. Standard-control replacement

- [x] Replace zoom halves with standard Buttons and remove their obsolete state paths.
      (#130, PR #157; `the_halves_split_their_pair_in_two` and
      `a_zoom_half_zooms_once_and_leaves_the_keyboard_on_the_plot` in `plot_view.rs`.)
- [x] Replace menu/splitter implementations where the checkpoint proved compatibility;
      obtain and record per-control decisions for any retained custom implementation.
      The custom menu is replaced by the stock `PopupMenu` in a controlled `Popover`
      (#131, PR #160; the `app_menu_ui.rs` tests), and the splitter is removed, with the
      minimap at a fixed configurable size (#132, PR #162; the `panels.rs` tests).
- [x] Audit remaining forms, title/status/start controls, hints and keycaps; keep
      standard controls and remove unnecessary duplicated interaction/style machinery.
      (#130, PR #157; `the_document_controls_join_the_toolbar`,
      `the_title_reserves_exactly_what_the_toolbar_draws` and
      `the_segment_group_carries_its_own_frame`.)
- [x] Complete the inventory with replacement evidence or accepted exceptions.
      (#133; every row has a final disposition with its verified limitation, and
      [ui/124-inventory.md](../../ui/124-inventory.md) maps every render and input entry
      point in `crates/app/src` to a row.)
- [x] Remove temporary prototype UI; retain a focused verification fixture or tests
      that exercise standard in-window inputs without shipping another settings surface.
      (#133: the fixture and the library face that existed only to share `chrome.rs`
      with it are gone. `settings_editor::standard_input_tests` opens a window whose root
      is a standard `Root`, types into a `NumberInput`, opens, chooses in and dismisses a
      `Select` with one Escape, and keeps the window.)

### 6. Integration and handoff

- [x] Update AGENTS current status from its pre-#124 description to implemented ownership,
      relevant documentation and user-visible changelog entries only for shipped changes.
      (#133; "Current status" now describes the implemented `Root`, frame, `PlotView`,
      overlay and standard-control ownership and names the retained exceptions. The
      README's description of the application menu matches the stock one. The changelog
      already carried the shipped changes, so nothing was added to it.)
- [x] Run the full validation below and external class-A review; resolve substantive
      findings within the repository's three-round limit.
- [x] Move this plan to `docs/plans/completed/` before final owner review, only when
      every in-scope implementation and validation task is complete.

## Validation

Record one row per executed case in the PR or a linked repository report:
`case ID | revision/build | OS/backend/compositor + display scale/theme/orientation |
preconditions | input sequence | expected | observed | pass/fail/not exercised |
evidence link`. Split parameter combinations into separate rows; a screenshot
supports painting, while an event trace or witnessed input sequence supports
interaction. `Not exercised` is outstanding coverage, not success. CI builds are
listed separately from native cases. At minimum include these atomic oracles:

| Case | Action / setup | Expected result |
| --- | --- | --- |
| R1 | Inspect main/editor roots and frame registration | One top-level Root per application window; the main Root border is disabled and only Argand frame code owns inset/resize setup; no second border or shadow |
| R2 | Resize each edge/corner, maximize/restore, fullscreen and supported tiling | Visible edge tracks pointer; expanded windows expose no stray resize zones; restored content has one frame's insets |
| R3 | Click toolbar control, then drag/double-click bare title | Control runs once without moving/zooming window; bare title retains platform move/zoom behavior |
| F1 | Focus plot and issue each navigation binding | Exactly one intended view change; no duplicate analysis generation from a single action |
| F2 | Focus input/select and type symbols, edit text, use clipboard/undo/IME | Control receives input; no plot pan/zoom/grid/orientation action and no swallowed editing shortcut |
| F3 | Dismiss nested menu/select/editor | Accepted one-level dismissal order and focus restoration; next key reaches the restored target |
| P1 | Open menu/popover over plot and click/wheel/drag its controls | Surface handles input; plot view and drag state remain unchanged |
| P2 | Enter and leave a passive hint, with and without Alt | Arrow and no underlying readout/guides while covered; immediate restoration on exit; baseline click/wheel behavior unchanged |
| P3 | Start drag, release outside, lose focus, open overlay or replace document | Gesture ends or cancels as specified; later pointer motion cannot resume it |
| O1 | Click/wheel/type while ready status is shown, including over a menu | Status disappears; original target still handles the input once; observer does not navigate |
| G1 | Replace snapshots during progressive updates and resize | Plot stops referencing retired snapshots before retirement; two-frame window-specific release retained; no stale image or accumulating uploads |
| S1 | Preview settings then cancel; replace file; restart after accepted settings | Existing rollback, accepted/displayed labels, persistence and full-range-on-open rules preserved |

Keep the full parameter coverage below; these oracles make its expected outcomes
explicit rather than replacing the matrix with a few successful screenshots.

**Platform scope (owner, 2026-09-29).** Native interaction is verified on Linux only.
Windows and macOS are covered by `ci/full` builds and tests; their native behaviour is
not verified, and a problem found there later becomes its own Issue. This replaces the
three-platform native matrix, and #124 closes without it. The decision is recorded in
[#133](133-ui-integration.md). The owner verified each stage natively on Linux as it
landed and did not repeat the matrix on the integrated build, which changes no
shipped behaviour; [ui/124-integration/linux-results.md](../../ui/124-integration/linux-results.md)
maps every oracle to the stage that verified it and records the waived cases.

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (workspace warnings denied)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Toolkit-free tests for navigation intents, geometry, persistence and existing
      analysis invariants; GPUI-level tests where supported for focus/action routing.
      (`navigation_tests.rs`, `panels.rs`, `session_tests.rs`, `document_tests.rs`,
      `spectrogram_tests.rs`, `minimap_tests.rs`, `axes_tests.rs`, and the headless
      GPUI tests in `plot_view.rs`, `hints.rs`, `app_menu_ui.rs`, `shell.rs` and
      `settings_editor.rs`.)
- [x] Native input matrix on Linux, per stage (#127 to #132), for the cases the
      evidence map lists as verified: plot navigation, main and context menus,
      pinned and passive hints, drag and release outside, Alt guides, Tab/Shift+Tab
      and focus restoration after menus. Escape on a nested select and on the
      settings editor is proven headless only; its native run was waived by the owner
      on 2026-09-29.
- [x] Native text editing on Linux: key routing into the settings editor's fields
      with no plot navigation behind them (#128). Caret movement, selection,
      clipboard, undo/redo, invalid numeric input, blur/Enter validation and IME as
      editing behaviour were not exercised; waived by the owner on 2026-09-29.
- [x] Native frame matrix on Linux (#127): restore/maximize, fullscreen, supported
      tiling, edge and corner resize without stale regions or doubled insets,
      title-bar drag and double-click, title-bar controls, native dialogs and file
      opening. Separate runs per theme, window size and display scale were not
      recorded; waived by the owner on 2026-09-29.
- [x] Both plot orientations, grid/scale toggles, start/loaded states, progressive
      updates, the fixed minimap and its boundary in both orientations (#132),
      settings preview/cancel and file replacement.
- [x] Compare representative current-release resize/navigation responsiveness and
      texture behavior with the baseline; investigate regressions rather than attributing
      them to extraction. No extra transforms or retained image backlog on resize.
      (Texture behaviour verified in #128, G1. The responsiveness comparison against
      `c176ad2` was waived by the owner on 2026-09-29; no stage showed a perceived
      regression.)
- [x] Record exact build, platform and exercised cases. No unit-test or CI-build result
      substitutes for native event verification, and `Not exercised` remains outstanding
      coverage rather than success.
- [ ] Final independent review clean, review conversations resolved and `ci/full`
      successful for the current up-to-date revision on Linux, Windows and macOS.
      (Review: Codex `gpt-6-sol` high, agreed 2026-09-29. CI stays three-platform.)

## Post-completion

Resume #108 on the standard infrastructure with standard text controls; explicitly
reconcile its obsolete no-Root constraint. Keep its product interaction work and
review separate, and do not merge PR #106's rejected numeric editor. Close #122 only
with linked evidence for all its criteria. #123 remains independently tracked.
