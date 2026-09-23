# Issue #128: Extract focused PlotView while preserving analysis and GPU resource ownership

Resolves [#128](https://github.com/o-kos/argand/issues/128).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 4 (ownership and
keyboard routing). Blocked by [#127](completed/127-borderless-root.md), merged in PR #139.

## Overview

Move plot focus, key context, pointer/readout position, hitbox geometry and gesture
lifecycle out of `Shell` into a `PlotView` entity. Shell keeps the document, the
analysis worker and its generations, view ranges and tick schemes, session
persistence, and every GPU texture upload, coalescing and retirement. PlotView
turns input into typed intents and paints an immutable snapshot that Shell
supplies. The global symbolic-key interceptor becomes a per-PlotView registration
gated on that PlotView's own focus.

Boundaries: no standard-control replacement (zoom pairs stay custom until #130,
splitter until #132), no central overlay policy (#129), no DSP or public
`argand-core`/`argand-dsp` API change, no persisted state added.

Implementation class: **A**, declared before handoff: it changes GPU texture
lifetime ownership boundaries and spans more than five code files. Per the owner's
2026-09-23 decision, as for #127, the implementation is done in-session and
reviewed by the owner directly; the external class-A model pair is not used.

## Context

Current ownership (all in `crates/app/src`):

- `shell.rs` `Shell` holds the plot interaction state: `pointer`, `pan`,
  `frequency_pan`, `pressed_zoom`, `plot_geometry`, `panel_bounds`,
  `splitter_dragging` and `badge_metrics`. It also holds the view state:
  `view`, `frequency`, `time_scheme`, `frequency_scheme`, `tick_pan` and `plot`
  (the measured `PlotSize` that sizes analysis requests). The resources are
  `texture`, `deep_preview`, `backdrop`, `backdrop_refresh`, `waveform` and
  `upload_pending`.
- `Shell::render` puts `track_focus(&self.focus)` and the key context
  `"Shell Plot Horizontal"` / `"Shell Plot Vertical"` (or `"Shell StartPage"`) on the
  whole window's content. Every descendant, including the ruler `PopupMenu`,
  therefore inherits `Plot` bindings.
- Whole-window mouse handlers on that root element (`drag_splitter`,
  `pointer_moved`, `finish_pan`, `finish_splitter`) keep drags tracking outside the
  plot. In GPUI 0.3.6, `on_mouse_move` fires only while the element's hitbox is
  hovered, and `on_mouse_up_out` fires only in the capture phase for a non-hovered
  hitbox. A child entity's element does not see moves outside its bounds.
- `plot_ui.rs` measures and paints through one `canvas`. Prepaint defers
  `Shell::layout_panels` (geometry, panel bounds, scheme resets, `resize`).
  Paint uses clones of `texture`, `deep_preview` and `backdrop`, which are
  captured per frame.
- `navigation_ui.rs` binds the plot keys in the `Plot` context and handles every
  navigation action on the Shell root (`navigation_actions`). Gestures (`wheel`,
  `begin_pan`, `minimap_press`, `pointer_moved`) mutate Shell's view directly. The
  app-wide `cx.intercept_keystrokes` in `init` normalizes symbolic zoom keys with the
  physical Shift state. It also swallows Ctrl+G/Ctrl+T/zoom keys when a popup that
  inherits `Plot` is focused (the #82 guard).
- Texture retirement: the free function `shell.rs::release` (two `on_next_frame`
  callbacks, then `drop_image` naming the window). It is called from `upload`,
  `release`, `DeepPreview::release` and the `backdrop.rs` replacement paths.
- `app_menu_ui.rs` returns focus to `shell.focus` on dismissal, before toolbar
  dispatch and before menu command dispatch (`dismiss_application_menu`, the toolbar
  `on_click`). Opening the menu clears `shell.pointer`.
- `settings_ui.rs::finish_settings` restores `view`/`frequency` and clears
  `frequency_scheme` on Cancel. The status bar shows `Shell::cursor_readout`.

Verified in the locked GPUI 0.3.6 (`window.rs::dispatch_key_event`), key dispatch
runs in this order:
keystroke interceptors, then binding match on the focused dispatch path, then
element key listeners (capture, bubble), then keystroke observers. An element's
`capture_key_down` therefore runs after a binding has already matched. So it cannot
correct the lost Shift on symbol keys. `intercept_keystrokes` is the only pre-match
hook, and it fires for every window.

Also verified: gpui-component `Button` calls `prevent_default` on mouse down, so a
click does not take focus. It is a tab stop, so Tab can move focus onto one. Deferred
context-menu popups inherit the key context of their anchor element's ancestors.

Headless GPUI tests are available. The `gpui-kit` feature `test-support` provides
`#[gpui_kit::test]`, `TestAppContext` and `gpui_kit::test::TestWindowExt`
(`render_frame`, `press`, `click`, `scroll`, `drag`). Before writing this plan, a
probe in `crates/app/tests/` checked that a `ctrl-=` binding in a child's `Plot`
context reaches the focused child's action handler. Call `press` through
`cx.update_window(handle.into(), ...)`, not inside an entity update; the latter
panics with "already being updated". Enabling the feature as a dev-dependency adds
about 110 lines to `Cargo.lock`.

## Decisions

### Ownership

| State | Owner | Notes |
| --- | --- | --- |
| Document, analyst, generations, requests, `plot: PlotSize` | Shell | Unchanged |
| `view`, `frequency`, `time_scheme`, `frequency_scheme`, `tick_pan`, settings backups | Shell | View changes arrive as intents; Shell bounds, requests analysis, maintains backups |
| `texture`, `deep_preview`, `backdrop`, `backdrop_refresh`, `waveform`, `upload_pending` | Shell | Sole upload, coalescing and retirement owner |
| Session fields (`waveform_fraction`, grid, scale UI, orientation, ruler mode) | Shell | PlotView never writes the session |
| Focus handle, `Plot` key context, navigation action handlers | PlotView | |
| `pointer`, `pan`, `frequency_pan`, `splitter_dragging`, `pressed_zoom` | PlotView | `pressed_zoom` stays until #130 removes it |
| `plot_geometry`, `panel_bounds`, `badge_metrics` | PlotView | Shell reads them through the entity when it needs them; no copy |
| Ruler context menu tracking (`open_menu`, `menu_dismiss`) | PlotView | The ruler menu is plot UI; the application menu stays Shell's |

No generic widget framework and no entity per primitive: one `PlotView` entity,
with the existing canvas, axes, minimap and badge code reused inside it.

### Lifetime

- PlotView is created per document when the metadata arrives (`Effect::Opened`,
  where `reset_view` runs). It is stored beside the document in `OpenFile`, with
  the Shell subscription to its intents. Replacing or closing the file drops both.
  That subscription lifetime is the stale-intent protection: a replaced
  PlotView cannot reach the new document.
- Shell focuses the new PlotView when it is created, unless focus has meanwhile
  moved off the shell handle (for example into the application menu); the focus
  target then returns it to the plot. `Shell::open` moves focus to
  the shell handle before dropping the old PlotView. If the document switches to
  `Showing::Failed` while a PlotView exists, the same transition moves focus to the
  shell handle, so focus never rests on a handle absent from the rendered frame.
- Existing resets map to explicit PlotView methods, and none is dropped:
  - `open`: the whole PlotView goes.
  - `choose_file`: the pressed zoom half.
  - Opening the application menu: the pointer.
  - `toggle_orientation`: drags, splitter drag, pointer and geometry.
  - Hiding the scale controls: the pressed half.
  - `bound_view`: both pan drags.
  - The start of a wheel gesture: both pan drags.

### Snapshot and texture retirement

- `PlotSnapshot` is a plain struct of shared references (`Arc` and `Copy` values),
  built by Shell from its own fields and replaced whole every frame. It holds:
  - the extents, texture, deep preview, backdrop clone and held picture view;
  - the first-paint marker and the minimap panel;
  - orientation, waveform fraction, held tick schemes, grid and scale-UI flags;
  - the ruler mode, time view and sample count, and whether frequency is zoomed.

  It holds only `Arc` clones, never copied grids or images. PlotView never uploads,
  retires or caches a texture.
- Shell publishes the snapshot to PlotView in every `Shell::render`, after
  `upload`/`prepare_backdrop`/`prepare_deep_preview`, and before returning the
  element that contains the PlotView. PlotView must not become a cached view: its
  render has to run in the frame in which the snapshot was set.
- Retirement has one entry, `shell::retire`, taking the current PlotView (if any),
  the images, the window and the context. It first clears that PlotView's snapshot
  synchronously. Only then does it schedule the existing two-callback release on the
  originating window. It does not notify the plot: retirement also runs inside
  `Shell::render`, where a notification would schedule a redundant frame. The
  release's first callback refreshes the window, and every Shell render publishes
  a fresh snapshot before the plot renders. `backdrop.rs` and `DeepPreview`
  return the images they give up instead of releasing them. After the change the
  scheduling code (`on_next_frame` + `drop_image`) exists in exactly one place.
  Window teardown keeps today's behavior: callbacks for a closed window never run
  and target no other window.

### Intents

PlotView emits intents through `EventEmitter`, and Shell handles them in one
`subscribe_in` callback. Intents carry toolkit-neutral values (views, fractions,
divisions, sizes), never pixels that Shell must reinterpret. Shell validates each
one against the current file (sample count, FFT floor, frequency cells), as the
current methods do. Suggested shape, which the implementer may refine:

- `Layout { plot: PlotSize, time_length_changed, frequency_length_changed }`
  replaces `layout_panels`'s Shell half. Resets and `resize` happen in Shell.
- `Pointer`: the pointer or readout changed. Shell repaints the status bar and
  applies the ready-status dismissal that `set_pointer` performs today.
- Time: zoom about an anchor, pan by fraction, pan by divisions, start, end, fit,
  drag from an origin view, minimap step and centre.
- Frequency: zoom about an anchor, pan by fraction, pan by divisions, fit, drag from
  an origin view.
- `WaveformFraction(f32)`: Shell stores it in the session and saves.

A pointer event still updates both viewports before one analysis request (#80): a
single drag intent may carry both axes, or Shell may coalesce them. Either way it
never makes two requests. Scheme holding keeps its current moments: at gesture start
and before pans.

### Focus, key context and actions

- While a document is shown, the plot owns the main window's keyboard focus (owner
  decision, 2026-09-23). This is the usual desktop convention: the canvas owns the
  keyboard, and commands are reached through the menu (F10) and shortcuts.
  - Every toolbar and status-bar `Button` gets `tab_stop(false)`: the application
    menu button, the toolbar buttons (`app_menu_ui.rs`), and the range, summary and
    hint buttons (`settings_ui.rs`). `chrome.rs` window controls are not focusable.
  - Clicking a Button already never takes focus (`prevent_default` on mouse down).
  - The start page (recent rows, chooser) and the separate settings window keep
    their tab stops.
  - Overlays (application menu, ruler context menu, and later the #108 settings
    popover) take focus temporarily and return it through the focus-target helper
    below.
- The PlotView focus handle sits on the plot surface element together with
  `key_context("Plot Horizontal" | "Plot Vertical")` and the navigation action
  handlers (zoom, pan, fit, Home/End, frequency). Shell's content keeps `"Shell"` or
  `"Shell StartPage"` only.
- Focusable overlays of the plot (the ruler context menu) are anchored in siblings
  of the plot surface element, not in its descendants. They therefore do not inherit
  `Plot` bindings. This replaces the #82 swallow guard.
- The session commands stay Shell handlers: `ToggleGrid`, `ToggleScaleUi`,
  `ToggleOrientation` and the ruler modes. They are reached by bubbling from plot
  focus and from menu/toolbar dispatch. Their bindings stay in the `Plot` context.
- Shell gains one focus-target helper: the PlotView focus while a plot is shown,
  otherwise the shell handle. The places that focus `shell.focus` before
  dispatching use it:
  - application menu dismissal and command dispatch;
  - the toolbar buttons;
  - the zoom pairs;
  - `choose_file`;
  - the ruler menu's `action_context`.

  Menu Fit time/Fit frequency therefore reach the PlotView handler deliberately.
  No navigation action becomes application-global.
- The interceptor moves from `navigation_ui::init` to PlotView and stays a
  `cx.intercept_keystrokes` registration, since that is the only pre-match hook.
  Its subscription is owned by PlotView and dropped with it. It returns without
  recognizing, dispatching or consuming anything unless two conditions hold: the
  event's window is PlotView's window, and PlotView's handle is exactly the focused
  one (`is_focused`, not `contains_focused`). It handles only the symbolic zoom and
  fit keys that need physical-Shift normalization. Ctrl+G and Ctrl+T return to
  plain bindings. It keeps the ready-status dismissal call it makes today.

### Pointer tracking outside the plot

While a pan, frequency pan or splitter drag is active, PlotView registers a
window-level mouse-move listener (`window.on_mouse_event`) during paint. GPUI
clears it every frame. The listener handles only moves outside the plot's bounds.
Moves inside stay with the plot's own `on_mouse_move`, which also covers moves
that arrive before the first frame after the press: GPUI dispatches mouse events
against the last rendered frame. Release outside is `on_mouse_up_out`. No such
listener exists while no gesture is active.

### Tests

Add `gpui-kit` with `features = ["test-support"]` to `crates/app` dev-dependencies
and commit the lock change. Keep it dev-only: `cargo tree -e normal -p argand` must
not change.

## Rejected alternatives

- Keeping the navigation action handlers on Shell: keyboard navigation would then
  not be owned by plot focus, and view-menu dispatch would still depend on focus
  sitting on an ancestor.
- Keeping one persistent PlotView for the whole window: replacement would need a
  hand-written reset of every field, and stale intents from a previous document
  would need a generation check. Creating one per document gives both by
  construction.
- A cached PlotView view, or a snapshot pushed only on resource changes: a missed
  push would leave a retired image paintable. Publishing in every render plus
  clearing at retirement is simple to audit.
- Moving the symbol normalization into `capture_key_down` on the plot element:
  verified to run after binding match, so Ctrl+Shift+plus would already have
  zoomed time.
- Removing the zoom bindings and matching all zoom keys in a key listener: the
  keycaps and `Kbd::binding_for_action` lookups need registered bindings.
- Moving the view ranges and tick schemes into PlotView: they drive analysis
  requests and settings rollback, which the issue keeps in Shell.

## Implementation steps

- [x] Add the headless test dependency and a minimal test harness for PlotView.
- [x] Introduce `PlotView` (new module, for example `plot_view.rs`) with its focus
      handle, snapshot slot, intent enum and the moved UI state. Move the canvas,
      splitter, unit hints, zoom pairs and ruler context menu rendering from
      `plot_ui.rs`/`navigation_ui.rs`/`shell.rs` into it.
- [x] Move gesture handling (wheel, pan, minimap press, pointer, splitter, layout
      deferral) into PlotView, emitting intents. Replace the whole-window Shell
      handlers with gesture-scoped window listeners.
- [x] Handle intents in Shell. Move the navigation action handlers to the plot
      surface and keep the session commands on Shell. Add the focus-target helper
      and use it at every former `shell.focus` dispatch site.
- [x] Create the PlotView on `Effect::Opened`, drop it with the document, and apply
      the focus transitions and the mapped resets listed above.
- [x] Route all retirement through the single entry, clearing the snapshot first.
      Make `backdrop.rs` and `DeepPreview` return the images they give up.
- [x] Move the interceptor into PlotView with its window and exact-focus gate, and
      restrict it to symbol normalization.
- [ ] Headless tests:
  - [x] Every navigation binding produces exactly one intent and one view change
        with plot focus.
  - [x] Plot keys do nothing with another focus beside the plot, or in another
        window. This covers Ctrl+G and symbol zoom.
  - ⚠️ With the ruler popup open, the headless test's assertions pass. It still
        fails GPUI's exit leak check, because gpui-component's ContextMenu retains
        its PopupMenu through an Rc cycle (#144). The case moves to the native F2
        check.
  - [x] Symbol zoom via top-row and keypad keys, with and without Shift.
  - [x] The snapshot is cleared before retirement is scheduled, and
        `drop_image` happens only after two frames.
  - [x] A replaced PlotView drops its snapshot and gestures, and its intents no
        longer reach Shell.
  - [x] A drag that leaves the plot still tracks and finishes on release outside.
  - ➕ Tab/Shift+Tab never landing on a toolbar or status-bar button needs a
        running Shell with an open file. It is covered by the native check.
- [x] Update AGENTS.md (the current-status bullets on Shell focus and key contexts,
      the interceptor paragraph in #82, and the navigation section) and #124's
      stage 4 rows that this issue covers. The CHANGELOG records the two
      user-visible differences. Tab no longer moves focus onto toolbar or status-bar
      buttons. Ctrl+U no longer toggles the scale controls behind the ruler context
      menu.
- [x] Set `tab_stop(false)` on the toolbar and status-bar Buttons listed above.
- [ ] Complete validation and move this plan to `docs/plans/completed/`.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Native (Linux Wayland first), recorded per #124's row format:
  - [ ] F1: each navigation binding in both orientations produces one view change
        and one analysis generation.
  - [ ] F2: the settings editor inputs, the ruler popup and the app menu receive
        their keys, with no plot navigation behind them.
  - [ ] P3: a drag released outside the window ends the gesture. So do opening a
        menu or the file chooser mid-drag, and replacing the document mid-drag.
  - [ ] G1: progressive updates, resize, splitter drag during analysis, zoom
        beyond 1024× (deep preview) and orientation toggle leave no stale image and
        no growing upload backlog. So does opening another file while old frames
        are in flight.
  - [ ] S1: settings preview then Cancel restores view, frequency and labels.
        Accept persists, and a restart restores the accepted settings.
  - [ ] Top-row and keypad Ctrl+plus/minus, with and without Shift.
        Ctrl+Shift+0 and Ctrl+0.
  - [ ] With a document shown, Tab/Shift+Tab never move focus onto toolbar or
        status-bar buttons, and plot keys keep working after any button click. The
        start page keeps Tab over its recent rows and chooser. The Alt guides, the
        readout, and the ready-status dismissal are unchanged.

## Post-completion

- Mark #124 stage 4's PlotView rows done with evidence links. Overlay-policy rows
  remain for #129.
