# Issue #129: Centralize overlay input isolation and focus restoration

Resolves [#129](https://github.com/o-kos/argand/issues/129) and
[#122](https://github.com/o-kos/argand/issues/122).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 4 (surface
contracts). Blocked by [#128](completed/128-plot-view.md), merged in PR #143.

## Overview

Surfaces drawn over the plot must own the input they cover, through toolkit
layering rather than plot-side knowledge of their rectangles. Today a hint over
the spectrogram keeps the crosshair cursor, and Alt guides and the readout keep
following the pointer underneath it (#122). Opening a menu or the settings
window, or losing window focus, does not end a drag in progress.

Apply the event contracts of the parent plan centrally:

- the analysis hint, the one interactive hint, owns all input while open and
  closes only on a click outside, Enter or Escape (the #108 contract);
- menus own covered pointer and wheel input, which they already do, and this is
  now tested;
- every overlay activation, window deactivation and document replacement ends
  the plot's gestures through one path;
- keyboard dismissal restores focus so the next key reaches the right target.

Boundaries: no standard-control replacement (#130), no menu mechanics (#131), no
splitter (#132), no hint contrast (#123), no #108 editor. No DSP, analysis or
texture-lifetime change, and no persisted state.

Implementation class: **A**, declared before implementation: it changes input
routing and gesture lifecycle across the plot, shell, menus and hints. Per the
owner's 2026-09-24 decision the implementation is done in-session and reviewed
by the owner directly, without codex. The reviewer model for any review round is
agreed with the owner first (interim process, #147).

## Context

Verified against the locked `gpui-pre` 0.3.6, `gpui-component` 0.6.6 and
`gpui-base` 0.6.6.

- Every Argand hint is a native GPUI tooltip (`.tooltip` / `.hoverable_tooltip`)
  whose view is a gpui-component `Tooltip::element`. Four builders make them:
  `shell.rs::shortcut_tooltip`, `shell.rs::metadata_tooltip`,
  `settings_ui.rs::analysis_tooltip` and `plot_ui.rs::unit_tooltip`. The
  analysis hint is hoverable and contains two standard Buttons (use the
  recommendation, edit settings).
- `Window::draw` prepaints the tooltip after the root and deferred draws and
  before computing `mouse_hit_test`, so tooltip hitboxes are topmost in the frame
  they appear in. The tooltip view is placed at the pointer plus one pixel;
  gpui-component's `Tooltip` renders `div().child(BaseTooltip.m_3()...)`, so the
  visible box is inset by a 12-pixel transparent margin.
- Cursor resolution takes the last-painted request whose hitbox is hovered. A
  plot hitbox under a hint remains hovered unless something above it blocks
  it, so the plot keeps receiving moves and computing its crosshair and guides.
- `HitboxBehavior::BlockMouseExceptScroll` (`block_mouse_except_scroll()`)
  makes every hitbox behind it not hovered while still letting them handle
  scroll. `occlude()` blocks scroll too.
- `div().on_mouse_move`, `on_mouse_down` and `on_hover` fire only while the
  element's hitbox is hovered. PlotView's `time-plot` element already clears its
  pointer on `on_hover(false)` and sets it again on the next move, so a blocked
  plot drops its readout and guides without knowing why.
- `PopupMenu` (ruler context menu) and the application menu overlay both
  `occlude()`; the plot's wheel handler uses the hover-based `on_scroll_wheel`,
  which respects that.
- `Window::capture_pointer(HitboxId)` exists, but no element exposes the hitbox
  id of a `div`, and the toolkit itself never calls it.
- Gesture interruption today: `choose_file` releases a pressed zoom half and
  dismisses the ruler menu; opening the application menu dismisses the ruler
  menu and clears the pointer; `bound_view` cancels drags. Window deactivation,
  opening the settings window and opening the ruler menu do not end a drag.
- `Shell::ready_input_observer` listens to mouse down and wheel in the capture
  phase at window level and never stops propagation; keystrokes reach it
  through `observe_keystrokes` and the two interceptors.
- Headless GPUI tests (`gpui-kit` `test-support`) can hover, move, click,
  scroll, drag and advance the clock past the tooltip delay.

## Decisions

### Hints

- A passive hint (`.tooltip`) lives only while its trigger is hovered: GPUI
  hides it on the first frame the pointer is outside the trigger's bounds. The
  pointer can therefore be inside it only over its trigger, which keeps the
  pointer, its cursor and its clicks. Blocking a passive hint was tried and
  rejected: a GPUI tooltip opens 13 pixels from the pointer and can cover its own
  trigger (a 22-pixel zoom half, a status-bar item), and a blocking box then took
  the trigger's click. A test guards this. Passive builders return their view
  through `hints::passive`, so no caller builds a bare Tooltip.
- #122's cursor criterion holds for passive hints through their triggers. The
  only triggers on the spectrogram are the zoom halves, which already show an
  arrow and suppress the guides. Unit captions sit on the rulers, where no
  guides are drawn.

### The analysis hint follows the #108 contract (owner decision)

The analysis hint is the only interactive hint: it shows the settings and
carries two Buttons. The owner's native checks rejected three successive
revisions, each built on GPUI's hoverable tooltip:
`block_mouse_except_scroll()` let the wheel through; keyboard focus taken only
with the pointer inside the hint left the keys acting with the pointer on the
trigger; and a hoverable tooltip closes when the pointer leaves. The owner then
pointed to the lifecycle agreed in #108 for the settings popover and chose to
apply it to this hint now (2026-09-24). The editor itself stays in #108.

- `hints::PinnedHint` holds the open state and emits `Pinned::Opened` and
  `Pinned::Closed { revert }`. `hints::pinned` renders a standard
  gpui-component `Popover` anchored above the trigger with the standard Tooltip
  look (`appearance(false)` around a `Tooltip` entity without its margin).
- Opening: hovering the trigger for 500 ms, as for a tooltip. A right click on
  the trigger toggles it, because the popover needs a trigger button and the
  left click keeps opening the settings window.
- Closing: a click outside, Enter (keeps the values), Escape (reverts to the
  settings at opening), and every command that replaces the surface: Ctrl+,
  (settings window), F10, Ctrl+O and opening a file. Moving the pointer away
  does not close it.
- Isolation: `hints::backdrop` renders a transparent `occlude()` layer over the
  window, deferred below the popover. The plot beneath gets no pointer, readout,
  Alt guides, clicks or wheel, and the click that closes the hint is consumed.
  The popover takes keyboard focus while open and restores it on close
  (gpui-base `PopoverState`), so `Plot` bindings never match. Shell ignores
  `FocusNext`/`FocusPrevious` while it is open, because `tab_group` orders tab
  stops but does not keep focus inside. Commands reach Shell as before, since
  the popover renders in the trigger's subtree.
- Escape: the popover's own `Cancel` binding closes it. A `Cancel` handler on
  the focused content first reports a reverting close and propagates. Shell
  restores `hint_opening` if the settings changed.
- Shell calls `interrupt_plot` when the hint opens.

### One gesture interruption path

- `PlotView::end_gestures` ends pan, frequency pan, splitter drag and a pressed
  zoom half. `PlotView::interrupt` also dismisses the ruler menu and clears the
  pointer and readout. `interrupt` replaces the ad hoc combinations in
  `choose_file` and the application menu. `bound_view` keeps its own drag
  cancellation, because it reacts to a view change, not to an overlay.
- Shell calls `interrupt` when the application menu opens, when the settings
  window opens and before the file chooser. Opening the ruler menu and
  deactivating the main window call `end_gestures` only: the menu restores the
  pointer when it closes, and an inactive window keeps its readout as before.
  Document replacement drops the PlotView and its gestures with it, as today.
- A drag keeps following the pointer while it crosses a hint. The drag tracker
  inserts its own hitbox with the plot's bounds and handles exactly the moves
  for which that hitbox is not hovered: beyond the plot, and over anything that
  blocks it. The plot's own listener handles the rest, so no move is handled
  twice. This replaces the tracker's bounds test, which stalled a drag over a
  hint (a negative control confirmed the stall).

### Keyboard and focus

- #128 already routes keys through plot focus and returns menu focus through
  `Shell::focus_target`. This stage adds the missing tests rather than new
  routing: dismissing the ruler menu or one application-menu level with Escape
  returns focus so that the next key reaches the restored target exactly once.
- Passive hints never take focus. The pinned analysis hint takes it while open,
  as a popover does (see above).

## Inventory

| Entry point | Surface | Contract |
| --- | --- | --- |
| `shortcut_tooltip` (toolbar, application button, status range, start page, zoom halves) | Passive hint | `hints::passive` |
| `metadata_tooltip` (status file and metadata fields) | Passive hint | `hints::passive` |
| `analysis_tooltip` (two Buttons) | Pinned hint | `hints::PinnedHint` popover with a backdrop, #108 lifecycle |
| `unit_tooltip` (ruler unit captions) | Passive hint | `hints::passive` |
| `NoTooltip` (application button while its menu is open) | None | Renders nothing, exempt |
| Application menu overlay (`app_menu_ui.rs`) | Menu | `occlude()`, stops move and wheel propagation, own focus, one-level Escape |
| Ruler context menu (`PopupMenu` via `context_menu`) | Menu | Toolkit `occlude()`, tracked by `PlotView::open_menu` |
| Settings editor (`settings_editor.rs`) | Separate native window | Window-local input, #128 tests |
| File chooser | Native dialog | Takes the pointer, gestures end before it opens |
| `time-plot` handlers (wheel, down, move, up, up-out, hover), splitter, zoom halves, drag tracker | Plot | Hover-based, so any blocking layer above wins |
| Title bar handlers (`shell.rs`), window controls (`chrome.rs`) | Frame | Outside the plot, unchanged |
| `ready_input_observer` (capture-phase down and wheel), `observe_keystrokes`, symbol interceptor | Ready-status observer | Never consume input |
| `observe_window_activation` | Window | Gains gesture interruption on deactivation |

## Rejected alternatives

- Keeping the analysis hint a GPUI hoverable tooltip with a blocking box and
  focus while hovered: rejected by the owner's native checks (wheel, keys, and
  closing on leave), see above.
- A plot-side list of open hint rectangles: explicitly rejected by #122 and the
  parent plan, and it would break for every hint added later.
- Pointer capture for drags: no element exposes a `div`'s hitbox id in the locked
  version, and the tracker's own hitbox gives the same result.

## Implementation steps

### 1. Prototype and inventory

- [x] Inventory every overlay entry point (see "Inventory" below).
- [x] Prove the hint helper in a headless test: a hint over a plot-like hitbox
      makes it not hovered, takes clicks and wheel and shows an arrow; its
      margin blocks nothing. A plain tooltip, kept as a control test, lets both
      the pointer and the click through.
- [x] Choose the drag-tracker mechanism for drags crossing a hint and record it here.

### 2. Hints

- [x] Add `hints.rs`; route the passive builders through `hints::passive`.
- [x] ➕ Keep passive hints unblocked after a test showed a blocking hint taking
      its own trigger's click.
- [x] ➕ Owner decision: the analysis hint becomes a pinned `Popover` with the
      #108 lifecycle and a backdrop (see Decisions).
- [x] Tests: a short hover opens nothing; leaving keeps the hint open; while
      open, the plot is not hovered and gets neither keys nor wheel; a click
      outside closes it without reaching the plot and gives the keyboard back;
      Enter keeps and Escape reverts. Controls without the backdrop or without
      the Escape handler fail them.
- [x] Native: the hint's look and position above the FFT summary, its two
      Buttons, Ctrl+R and Escape reverting it, Tab staying inside, and the
      readout and Alt guides absent while it is open.

### 3. Gesture interruption

- [x] Add `PlotView::interrupt` and `end_gestures` and call them from every
      activation listed above.
- [x] Remove the ad hoc combinations `interrupt` replaces.
- [x] Tests: an interrupted drag clears the readout and does not resume on later
      moves; release outside still ends a drag; a replaced plot leaves nothing
      behind (#128 test).
- [x] Native: F10, Ctrl+, and Ctrl+O during a drag, a window switch during a drag,
      and a right click on the time ruler during a drag. The ruler menu leaks in
      headless tests (#144), and the others need Shell.

### 4. Menus, keyboard and the ready status

- [x] Test: an occluding layer, as both menus are, takes wheel, click and drag
      from the plot. This fails if the plot's handlers stop being hover-based.
- [x] Native: wheel, click and drag over the application menu and the ruler menu
      leave the plot view unchanged.
- [x] Native: Escape on the ruler menu and on each application-menu level returns
      focus as specified, and the next navigation key acts exactly once.
- [x] Native: while the ready status shows, a click or wheel over a menu dismisses
      it and still reaches the menu once, and navigates nothing.

### 5. Documentation and validation

- [x] Add an "Overlay surfaces (#129)" section to `AGENTS.md` and update the drag
      tracker statement in "Plot ownership (#128)".
- [x] `CHANGELOG.md`: the pinned analysis hint; overlays and window switches end
      drags.
- [ ] Update the parent plan's phase 4 rows with evidence links.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Native cases on the current release build, one row each in the Pull
      Request (`case | build | platform/backend | input | expected | observed |
      result`): P1 (menus over the plot), P2 (every hint over the spectrogram,
      with and without Alt, entering and leaving), P3 (drag with release
      outside, window switch, F10, Ctrl+, and Ctrl+O during a drag, file
      replacement), F3 (Escape on each menu, then a navigation key), O1 (ready
      status dismissed by click or wheel over a menu). Platforms without a
      native run are listed as not exercised.
- [ ] #122 evidence: each of its acceptance criteria mapped to a test or native
      row, linked from the Pull Request.

### Owner's native check (Ubuntu, 2026-09-25)

The listed native cases passed. Two findings were added to this Issue at the
owner's decision:
- [x] ➕ A pointer resting over the plot was not known until it moved: after
      opening a file from the start page, and after a hint or menu closed by a
      key. `time-plot`'s `on_hover` fires on layout changes under a still
      pointer too, and now takes the pointer up (`PlotView::pick_up_pointer`)
      while the window is hovered. A first attempt used a hitbox probe after
      each frame; the owner's check found the crosshair and readout blinking on
      the first key, because GPUI's modality-aware hover cleared the pointer and
      the probe restored it. `HoverListenerMode::InputModalityIndependent` keeps
      the hover through key presses, and the probe is gone. Tests: key presses
      keep the pointer (a control with the default mode fails), and a resting
      pointer is taken up only inside the window.
- [x] ➕ Tab and every bound key hid the mouse pointer until the mouse moved
      (GPUI's default `CursorHideMode::OnTypingAndAction`). The application
      now sets `CursorHideMode::Never`.

## Review

External review with GPT-6 Sol, high reasoning (owner's choice, 2026-09-25).

Round 1:
- Accepted: Space closed the pinned hint through the popover's `Confirm`. A
  `NoAction` binding in the deeper `PinnedHint` context now leaves it open; a
  test and a control cover it.
- Accepted: the Escape test only checked the event. Two Shell-level tests now
  check that a reverting close restores the opening settings and that other
  closes keep them; a control without the revert fails.
- Accepted: the status-bar readout stayed over the corner zoom buttons, where
  the Alt guides were already hidden. `cursor_readout` now skips them too.
- Accepted: the plan said hints never take focus; it now limits that to
  passive hints. Validation items are ticked as they are run.
- Declined: enforcing #122's rule inside `hints::passive`. A passive hint is
  visible only while its trigger is hovered, so the pointer is inside it only
  over the trigger, and what the plot shows there is the trigger's concern, not
  the hint's. On the spectrogram the only triggers are the zoom halves, whose
  zones already suppress the crosshair, guides and now the readout through
  `PlotGeometry::controls_at`; #130 replaces them with standard Buttons.
  Enforcing it in the hint would block the pointer and so the trigger's own
  click, which a test guards.

Round 2 confirmed the fixes and accepted the reasoning behind the decline.
- Accepted: the passive-hint test would pass without the hint being shown, and
  used a 60-pixel trigger. It now uses a 22-pixel trigger like a zoom half and
  asserts that the hint is shown and covers the clicked point before checking
  that the trigger gets the click.

Round 3 confirmed the round 2 fix.
- Accepted (P1): a right click opened the pinned hint while the settings window
  was open, and the two surfaces kept separate rollbacks, so Escape in the hint
  could restore a setting the editor had just cancelled. `PinnedHint` is now
  disabled from the editor opening until it finishes; a disabled hint renders
  its trigger without the popover. Tests cover a disabled hint on hover and on a
  right click (a control that keeps the popover fails), and the Shell sequence
  of opening the editor, failing to open the hint, and opening it after the
  editor closes.
- Accepted (P2): the Pull Request description's review section was stale; it
  is updated.
- ➕ Found while testing: a right click on any toolbar or status-bar `Button`
  takes keyboard focus, because the toolkit prevents the default focusing only
  for the left button. Pre-existing and unrelated to this Issue, tracked in
  #156.

Additional round at the owner's request, after the owner's native check, on
the changes since round 3:
- Accepted (P1): after `MouseExited` GPUI keeps the last position and hit test,
  and `is_window_hovered` means an active window on macOS, so a stale position
  could be taken up. Shell now follows window-level moves and exits
  (`pointer_presence`); the plot takes a pointer up only while the mouse is in
  the window, and the plot's pointer clears when it leaves.
- Accepted (P2): an orientation change cleared the pointer while the plot stayed
  hovered, so nothing restored it. The pointer is now kept and filtered through
  the new geometry at the next layout.
- Accepted (P2): the pick-up test called the method directly. The tests now
  uncover a plot under a still pointer and let the real hover listener run,
  with and without the mouse in the window. Controls without the pick-up, with
  the old orientation clearing, or without the window check each fail.
- Declined: taking up the pointer when the settings window closes over a still
  mouse. The plot is not covered there; the mouse was in another window, and
  no reliable signal says it rests over the plot again before it moves. Taking
  it up on the first move matches ordinary desktop behaviour.

## Post-completion

- Close #122 through the Pull Request, with its evidence linked.
- Mark #124 phase 4's remaining rows done with evidence links.
