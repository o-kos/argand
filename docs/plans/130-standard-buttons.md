# Issue #130: Standard zoom buttons and toolbar polish

Resolves [#130](https://github.com/o-kos/argand/issues/130).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 5 (standard-control
replacement). Predecessor [#129](completed/129-overlay-isolation.md), merged in PR #155.

## Overview

The spectrogram's corner `[+|-]` zoom pairs are hand-built `div` halves with their own
pressed state (`PlotView::pressed_zoom`), mouse-down/up/up-out listeners and reset
paths. Replace each half with a standard gpui-component `Button` and delete that
machinery. Polish the title-bar toolbar as the owner requested:

- toolbar buttons lose their visible outline frame;
- Grid and Orientation are absent until a document is open;
- orientation becomes a two-segment control showing both modes, the current one
  selected, replacing the single button whose icon showed the current mode while its
  hint named the other one;
- the zoom halves behave and present as real buttons.

Audit the remaining standard buttons (toolbar, status FFT/range, start page, settings
form) and record each one's disposition; remove only machinery that supported
composition demonstrably reproduces.

Boundaries: no cascade menu (#131), splitter (#132), hint contrast (#123), #108 editor,
DSP, analysis, texture lifetime or persisted-state change. Grid, orientation and zoom
behaviour, shortcuts, menu rows and session format stay as they are. The toolbar
visibility and appearance changes above, and the orientation segment, are the only
product-layout changes.

Implementation class: **B**, declared before implementation: two to four GUI modules
(`plot_ui.rs`, `plot_view.rs`, `app_menu_ui.rs`, `navigation_ui.rs`, possibly
`shell.rs` / `settings_ui.rs`) with local, known invariants and concrete acceptance
criteria, expected well under 400 lines. Reclassify to A before continuing if the
change grows past that or touches focus routing, retirement or analysis.

Roles for this Issue, set by the owner on 2026-09-25 (an experiment that departs from
the AGENTS.md class table): implementer **Kilo Code** with
`openrouter/stealth/space-bunny-alpha`, variant `xhigh`; reviewer **Codex**
`gpt-6-sol`, reasoning effort `high`, for every review round of this PR. Claude writes
this plan, arbitrates review disagreements, runs the gate and talks to the owner.

## Context

Verified against the locked `gpui-component` 0.6.6 (`src/button/button.rs`,
`button_group.rs`):

- `Button` paints hover and pressed through GPUI's `.hover()` / `.active()` style
  refinements. The active state belongs to the element that received the mouse down
  and ends on any mouse up, inside or outside, so a pressed look cannot stick. GPUI's
  `on_click` fires only when both down and up land on the same hitbox, so release
  outside does not activate.
- Its mouse-down handler calls `window.prevent_default()`, so a pointer press does not
  move focus. `tab_stop(false)` removes it from Tab order. A focused Button activates
  on Enter and Space (covered by the component's own tests).
- `disabled(true)` removes hover, active and click and applies disabled colours.
- `ButtonCustomVariant` carries background `color`, `foreground`, `hover` and
  `active` backgrounds. It has no border and no hover foreground. A border is drawn
  only for the `Default` variant or `outline()`, or when the caller adds one, as the
  toolbar does now through `toolbar_border`.
- `Selectable::selected(true)` switches the background to the variant's selected
  style, which for `Custom` is its `active` colour, and suppresses hover and active on
  that button.
- `ButtonGroup` is the standard segmented control. It lays out children horizontally
  or vertically, rounds only the outer corners, marks every child as a toggle
  (`aria_toggled`) and reports the resulting selection through `on_click(&Vec<usize>)`.
  A child's own `on_click` is replaced when the group has one. It is a toggle group,
  which suits orientation but not the zoom pair, whose halves are actions.

Current code:

- `plot_ui.rs` `ruler_zoom_buttons`, `zoom_pair` and `half_button` build the pairs.
  Each half sets `pressed_zoom` on mouse down, clears it on mouse up / up-out through
  `PlotView::release_press`, and on click focuses the plot and dispatches the zoom
  action. The pair is a framed, rounded, translucent container with a one-pixel
  divider; horizontal pairs sit bottom-left, vertical pairs top-right
  (`PlotGeometry::zoom_zones`, `corner_zones`).
- `plot_view.rs` owns `pressed_zoom`, `release_press` and the call in `end_gestures`.
  `navigation_ui.rs` releases the press when `ToggleScaleUi` hides the pairs.
- `app_menu_ui.rs` `toolbar` / `toolbar_button` render the application button, a
  separator, Orientation and Grid, all with a one-pixel border (`toolbar_border`).
  Orientation and Grid are rendered disabled without a document. `toolbar_width`
  assumes three controls and feeds the title's symmetric centring margin in
  `shell.rs`.
- The orientation icon is `horizontal.svg` (time arrow pointing right) in horizontal
  mode and `vertical.svg` in vertical mode, with the hint "Switch to vertical /
  horizontal orientation".
- Status FFT summary and range (`settings_ui.rs`) are Buttons with a custom variant,
  plus manual `analysis_hovered` / `range_hovered` state. That state drives the text
  colour, which the custom variant cannot express, and the pinned analysis hint's
  hover delay.
- Start page recent rows and the chooser (`shell.rs`) are ghost Buttons. Settings
  editor buttons are outline Buttons. Linux window controls in `chrome.rs` belong to
  the retained frame (parent inventory row one) and are outside this Issue.

## Decisions

- **Zoom halves are two standard `Button`s inside the existing frame container.** The
  frame (border, rounding, translucent paper background, overflow clip) and the
  one-pixel divider stay plain `div`s: they are decoration, not interaction. Each half
  is `Button::new(id).custom(..)` with transparent `color`, the current hover and
  pressed accent backgrounds as `hover` / `active`, `rounded(ButtonRounded::None)` (the
  container clips the corners), no padding, the existing 12-pixel `Icon` and
  `flex_1` across its axis. Its `on_click` focuses the plot and dispatches the
  registered zoom action, exactly once per activation. Tooltips stay
  `shortcut_tooltip` in the `Plot` context through `interactivity().tooltip`, so they
  remain passive hints (`hints::passive`).
- **Delete `pressed_zoom`, `release_press`, the half's mouse-down / up / up-out
  listeners, the `ToggleScaleUi` release and the `end_gestures` call.** GPUI's active
  state already ends on any release, and a hidden element has no active state to keep.
  Update doc comments that mention the pressed half.
- **Zoom buttons are not Tab stops** (owner, 2026-09-25). While a document is shown
  the plot owns keyboard focus (#128). Keyboard access is Ctrl+Plus/Minus and
  Ctrl+Shift+Plus/Minus. The criterion "focus, keyboard activation" is met by keeping
  the standard Button's own focus handling and Enter/Space activation intact, not by
  adding the buttons to Tab order. A click leaves focus on the plot.
- **Enabled rule unchanged:** the pairs are enabled whenever they are shown, as today.
  `disabled` stays wired so a future rule uses the standard disabled style.
- **Orientation is a `ButtonGroup` of two segments** (owner, 2026-09-25): horizontal
  (`horizontal.svg`) then vertical (`vertical.svg`), the current mode `selected`. The
  two modes are equal alternatives, not a normal state and its inversion, so neither
  a single toggle nor an icon that shows the next mode fits (Apple HIG segmented
  controls and toolbar toggles; NN/g "State-Switch Controls"). Clicking the
  unselected segment dispatches `ToggleOrientation`; clicking the selected one does
  nothing. Hints name the segment's mode, "Horizontal orientation" and "Vertical
  orientation"; only the unselected segment shows the Ctrl+T keycap, because Ctrl+T
  switches to it. Both segments are `tab_stop(false)`. The View menu row and Ctrl+T
  are unchanged.
- **Grid stays one toggle button**: grid visibility is a genuine on/off option.
  Hint "Show grid" / "Hide grid" as today. While the grid is shown the button is on:
  the variant's accent colour, with `toggled` for accessibility, not `selected(true)`.
- **No outline frames.** Drop `toolbar_border` and the `border_1` on the application
  button, Grid and the orientation segments. Selected is expressed by background
  (accent at the existing 0.18 opacity), hover by the existing 0.32 accent and pressed
  by 0.44. Selected, hover and pressed must stay visually distinct in both themes; if
  `selected(true)` makes selected identical to pressed (custom `active`), express
  selection through the variant's `color` instead and still report it through
  `selected` / `aria_toggled` where the component allows without restyling. The
  application button keeps its open-menu highlight by the same background rule.
- **Grid and Orientation render only with a document** (`self.view.is_some()`). Without
  one, neither they nor the separator before them are in the tree, so no hitbox, gap
  or focus remains. `toolbar_width` takes the same condition so the title's centring
  margin matches what is drawn.
- **Status, start page and settings buttons are retained as standard Buttons.** Record
  their disposition in the parent inventory. Manual hover state in the status bar stays
  because the custom variant has no hover foreground and the pinned hint needs the
  hover events; remove only styling that duplicates what the variant already paints
  (for example a `.bg(secondary_hover)` when hovered that equals the variant's own
  hover background), and only if the observed colours and the yellow warning
  progression are unchanged.

## Rejected alternatives

- `ButtonGroup` for the zoom pair: it marks each half as a toggle and routes clicks
  through a selection callback, which misstates zoom actions and invites a selected
  look after a click.
- Keep the `div` halves and only fix the pressed state: the stage exists to remove
  hand-built button interaction (parent inventory).
- Zoom buttons as Tab stops: Tab from the plot would move focus onto overlay buttons,
  after which arrows no longer pan; conflicts with plot focus ownership (#128).
- Orientation as a single toggle with a fixed icon and a selected look in vertical
  mode: implies horizontal is normal and vertical its inversion (owner rejected).
- Orientation icon showing the next mode ("Play/Pause" model): a layout picture reads
  as the current state, which is the ambiguity this Issue fixes.
- Disabled Grid/Orientation before a file: they do nothing without a document and
  their hitboxes break title dragging; the owner asked for them to be absent.

## Implementation steps

- [x] Replace `half_button` with a standard `Button` per half inside the retained
      frame container; keep placement, 22-pixel squares, divider, rounding,
      translucency, icons, tooltips and both orientations.
- [x] Remove `pressed_zoom`, `release_press`, their listeners and every caller
      (`end_gestures`, `ToggleScaleUi`), and update comments that describe them.
- [x] Remove the toolbar outline frames; keep selected, hover and pressed distinct.
- [x] Render the separator, orientation segment and Grid only with a document, and
      make `toolbar_width` match.
- [x] Replace the orientation button with the two-segment `ButtonGroup`, hints and
      keycap as decided.
- [x] Audit status FFT/range, start page and settings buttons; remove only
      redundant styling proven equivalent; record every disposition in the parent
      inventory table and mark the relevant phase-5 items there.
- [x] Headless GPUI tests (`gpui-kit` `test-support`): a zoom-half click dispatches
      its zoom action exactly once and leaves focus on the plot; press inside then
      release outside dispatches nothing and a later click still works; hiding the
      pairs (`ToggleScaleUi`) mid-press leaves nothing stuck; the toolbar tree has no
      Grid/Orientation without a document and has them with one; clicking the
      unselected orientation segment toggles once and the selected one does nothing.
      Where a case cannot be driven headlessly, say so here and cover it natively.

      Three cases are `plot_view.rs` (`a_zoom_half_zooms_once_and_leaves_the_keyboard_on_the_plot`,
      `a_release_outside_a_zoom_half_changes_nothing`,
      `hiding_the_pairs_during_a_press_leaves_nothing_behind`) and two are
      `app_menu_ui.rs` (`the_document_controls_join_the_toolbar`,
      `only_the_other_orientation_segment_switches`). What the three plot cases
      prove is activation, a release outside and a hide during a press, not the
      absence of a stuck pressed look, which is a surface observation. Keyboard
      traversal is the one case that cannot be driven headlessly: GPUI's
      tab-stop map orders handles by their tab-index path, and every focusable
      in the harness shares one path, so `focus_next` returns the handle that
      already holds focus. The stuck look, a document replaced during a press and
      Tab avoidance therefore stay native-matrix cases, and each of these buttons
      carries `tab_stop(false)` for the last.
- [x] Update AGENTS.md (toolbar, zoom pair and plot ownership paragraphs: no pressed
      state, segmented orientation, toolbar visibility) and add `CHANGELOG.md`
      `[Unreleased]` entries for the user-visible changes.
- [x] ➕ Owner feedback 1: segment frame and icons, on state, status hover from
      paint-time hover, zoom half corners.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Owner feedback 1 (2026-09-25)

The first native check found four problems. None was visible to a static review.
Decisions, taken with the owner on an interactive colour preview:

- **Orientation segment frame and icons.** The segment gets one shared frame with a
  one-pixel divider, both in `foreground.opacity(0.24)` (the theme `border` is almost
  invisible on the title bar). Icons are Lucide `panel-top` (horizontal: the minimap
  strip on top) and `panel-left` (vertical: the strip on the left), taken from the
  gpui-kit asset catalog, whose default bundle serves `panel-left` and
  `icon_assets!` in `assets.rs` embeds `panel-top` alone. Retire `horizontal.svg`
  and `vertical.svg` if nothing else uses them.
- **On state.** Grid on and the selected segment use accent 0.30 background with an
  accent-coloured glyph. A Grid that is on still reacts, with 0.40 under the pointer
  and 0.52 while pressed. Off controls keep hover 0.32, pressed 0.44 and the glyph
  tint. The selected segment keeps its on look under the pointer and stays inert. The
  application button with its menu open uses the same on background.
- **Status FFT and range flicker** (pre-existing from #129). The manual
  `analysis_hovered` / `range_hovered` flags go out of step with the pointer when the
  pinned hint's backdrop covers the window, so the summary lightens, darkens and
  lightens again. Drive the hover look from GPUI's paint-time hover instead (Button
  hover background and a group hover on the text colour). The FFT summary stays lit
  while its pinned hint is open. After the hint closes, the look follows the real
  pointer at once. Keep `on_hover` only where a behaviour needs it (the pinned hint's
  open delay). The accepted colours, including the yellow warning progression and its
  pressed state, do not change.
- **Zoom half corners.** GPUI clips content only to rectangles (`ContentMask` has no
  corner radii), so a pressed half painted square corners over the rounded frame.
  Each half rounds its own outer corners to match the frame.
- **No auto-repeat** for the zoom halves (owner declined it).

## Review round 1

The arbiter's decisions, all implemented on this branch.

- A selected toolbar control keeps its hover and pressed feedback, so the
  application button and the grid toggle are no longer `selected`; their on state
  is the accent shade in the custom variant, and the grid keeps `toggled` for
  accessibility. The orientation segment in force stays `selected` and inert,
  because clicking the mode in force is not an action.
- The status-bar FFT and range buttons keep their manual hover background. The
  accepted status-bar colours do not change in this Issue.
- Clicking a segment returns focus to `Shell::focus_target` before it toggles, as
  the grid toggle does.
- An actionable toolbar control tints its icon while hovered, on the grid toggle
  and the unselected segment, and the selected segment does not.
- Gaps inside the populated toolbar swallow title drag and double-click. The
  container handler predates this branch, so this is declined here and tracked in
  #158.

Round 2 adds two accepted items. A click on either segment returns focus to the
owner before the mode comparison, so the selected segment hands the keyboard back
without switching. The wording of the toolbar's on state says Grid is on rather
than selected, and the hover tint is claimed for the grid glyph and the segment
that changes the mode, not for the application button's artwork.

Round 3 is clean. Its two documentation nits (Grid described as on with `toggled`,
and focus return limited to the controls that do return it) are fixed. The reviewer
accepted both declined round-1 items.

Owner feedback 1 is implemented as decided. The two orientation segments share one
frame on the group with a painted divider, because a `Button` border takes the
colour of the states its own variant passes through, and the segment icons are the
Lucide panel strips. The on state is accent 0.30 with 0.40 and 0.52 under the
pointer, and the status controls follow the pointer through paint-time hover. The
segment icons come from the toolkit: `panel-left` is in the default bundle and
`assets.rs` embeds `panel-top` alone with `icon_assets!`, so no Lucide artwork is
vendored and the complete catalog is not linked.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Native matrix on Linux (owner), recorded in the PR as
      `case | revision | platform/backend, theme, orientation | input | expected |
      observed | pass/fail/not exercised`:
  - zoom halves: hover, press, release inside (one zoom), release outside (no zoom, no
    stuck look), rapid clicks, Ctrl+U hide while pressed, both orientations, both
    themes, focus stays on the plot (arrows still pan after a click);
  - toolbar: start page shows only the application button with the title still
    centred and draggable; opening a file adds the segment and Grid without layout
    jumps beyond that; no outline in normal state; hover, pressed and selected
    distinguishable in both themes; orientation segment and Grid clicks run once
    without moving or maximizing the window (R3), menu rows and Ctrl+T / Ctrl+G stay
    in sync; narrow window;
  - status FFT/range: hover, pressed, yellow warning pressed state, Ctrl+R unchanged;
  - file chooser opened and document replaced during a press: no stuck state;
  - keyboard only: Tab never lands on zoom or toolbar buttons with a document open.
- [ ] Windows and macOS: `ci/full`; native interaction there is not exercised unless
      the owner runs it, and is reported as such.

## Post-completion

- Continue with #131 (cascade menu) on the same standard-control basis.
