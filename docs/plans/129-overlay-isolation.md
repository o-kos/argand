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

- hints own the pointer inside their visible box;
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
  scroll. `occlude()` blocks scroll too, which the parent plan and #122 rule out
  for hints.
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

### Hints own their box

- One helper, in a new `hints.rs`, turns a gpui-component `Tooltip` into the
  view a `.tooltip` builder returns. It renders the standard Tooltip with its
  margin removed, wraps it in an element with `block_mouse_except_scroll()`
  and an arrow cursor, and puts the 12-pixel margin back outside that element.
  The blocking area is therefore exactly the visible box, border included, and
  the transparent margin blocks nothing.
- All four builders go through it, so a hint added later gets the rule by
  using the builders.
- Owner decision (2026-09-24): a hint takes the clicks inside its box. A click on
  a hint no longer starts a plot pan or drag under it, and the analysis hint's
  Buttons receive their clicks alone. The wheel still passes through to the plot.
  This refines #122's "clicks and wheel gestures are unaffected", which was
  written before the analysis hint gained buttons.
- The plot keeps deciding its cursor, readout and guides from its own hitbox
  hover state. It gets no list of open hint rectangles.

### One gesture interruption path

- `PlotView::interrupt` ends everything transient: pan, frequency pan, splitter
  drag, a pressed zoom half, the pointer and readout, and its own ruler menu
  unless that menu is the surface opening. It replaces the ad hoc combinations
  in `choose_file`, the application menu and `bound_view`.
- Shell calls it when the application menu opens, when the settings window
  opens, before the file chooser, and when the main window deactivates. PlotView
  calls it when its ruler menu opens. Document replacement drops the PlotView
  and its gestures with it, as today.
- A drag keeps following the pointer while it crosses a hint. The mechanism is
  chosen in the prototype step: the drag tracker must see moves that the plot's
  own blocked hitbox no longer receives, without handling any move twice.

### Keyboard and focus

- #128 already routes keys through plot focus and returns menu focus through
  `Shell::focus_target`. This stage adds the missing tests rather than new
  routing: dismissing the ruler menu or one application-menu level with Escape
  returns focus so that the next key reaches the restored target exactly once.
- Hints never take focus. The helper adds no focus handle.

## Rejected alternatives

- `occlude()` on hints: it would also swallow wheel gestures over a hint,
  contrary to the parent plan and #122.
- A plot-side list of open hint rectangles: explicitly rejected by #122 and the
  parent plan, and it would break for every hint added later.
- Blocking only the hint's inner content: the Tooltip's padding and border
  would keep the crosshair and guides.
- Wrapping the whole tooltip view, margin included: the transparent 12-pixel
  ring around every hint would swallow clicks meant for the plot.
- Pointer capture for drags: no element exposes a `div`'s hitbox id in the locked
  version. Revisit only if the prototype finds no simpler tracker.

## Implementation steps

### 1. Prototype and inventory

- [ ] Inventory every overlay entry point: `.tooltip`, `.hoverable_tooltip`,
      `Tooltip::element`, `PopupMenu`/context menus, the application menu, the
      settings window, the file chooser, `on_mouse*`/`on_scroll*` handlers and
      window-level listeners in `crates/app/src`. Record each with its
      contract row in this plan.
- [ ] Prove the hint helper in a headless test: a hint over a plot-like hitbox
      makes it not hovered, takes clicks, passes wheel and shows an arrow; its
      margin blocks nothing.
- [ ] Choose the drag-tracker mechanism for drags crossing a hint and record it here.

### 2. Hints

- [ ] Add `hints.rs` with the surface helper; route the four builders through it.
- [ ] Keep hint sizes, placement, typography and hoverable behaviour unchanged.
- [ ] Tests: arrow cursor, cleared readout and absent guides over a hint; the
      plot's own pointer returns on the first move back; wheel over a hint still
      pans the plot; a click on a hint starts no plot gesture; the analysis
      hint's Buttons act once.

### 3. Gesture interruption

- [ ] Add `PlotView::interrupt` and call it from every activation listed above.
- [ ] Remove the ad hoc combinations it replaces.
- [ ] Tests: opening each surface during a drag ends it; later moves do not resume
      it; release outside still ends a drag; deactivation ends a drag; a
      replaced document leaves no gesture behind.

### 4. Menus, keyboard and the ready status

- [ ] Tests: wheel, click and drag over the application menu and the ruler menu
      leave the plot view and drag state unchanged.
- [ ] Tests: Escape on the ruler menu and on each application-menu level returns
      focus as specified, and the next navigation key produces exactly one intent.
- [ ] Tests: while the ready status shows, a click or wheel over a menu dismisses
      it and still reaches the menu once, and navigates nothing.

### 5. Documentation and validation

- [ ] Add an "Overlay surfaces (#129)" section to `AGENTS.md` and update the #122
      statement in "Plot ownership (#128)".
- [ ] `CHANGELOG.md`: hints show an arrow and hide guides and readout; overlays and
      window switches end drags.
- [ ] Update the parent plan's phase 4 rows with evidence links.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
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

## Post-completion

- Close #122 through the Pull Request, with its evidence linked.
- Mark #124 phase 4's remaining rows done with evidence links.
