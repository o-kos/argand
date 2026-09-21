# Issue #124: Standard UI infrastructure and focused plot ownership

Resolves [#124](https://github.com/o-kos/argand/issues/124).

## Overview

Adopt the standard gpui-component `Root` and its window frame, give the plot a
dedicated focused entity, and replace custom ordinary controls with toolkit
components wherever their required behavior can be preserved. This is the owner's
selected option 2, not a proposal to keep the custom frame under another wrapper.

This initial revision contains planning and policy only. No UI migration, native
interaction verification or dependency change has been performed by this revision.

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
  Root + standard window frame (one owner)
    Shell: document/analysis coordination, title bar, status, window commands
      PlotView: plot focus, pointer/readout state, hitboxes and gesture lifecycle
        existing canvas/textures, axes, waveform and minimap
        standard zoom controls
      toolkit overlays: menus, hints and editing popovers
```

This is an ownership sketch, not a requirement to draw every overlay as a normal
child; use the toolkit's supported overlay placement and focus mechanisms.
Do not create a generic widget framework or one entity per drawn tick/primitive.

- Keep analysis workers, generations, view requests and accepted snapshots under
  document/application coordination. Plot interactions submit typed view intents;
  they do not duplicate analysis state or mutate the worker directly.
- Keep the existing texture upload/coalescing/retirement owner where practical.
  Passing render snapshots into PlotView must not shorten texture lifetimes or
  create duplicate caches. Name the window when retiring a texture and retain the
  intervening-redraw contract.
- UI-only gesture/focus state moves to PlotView. Session persistence remains
  centralized; do not persist hover, focus or pressed state.
- Exactly one standard frame owns decoration insets and resizing. Keep supported
  platform title-bar controls. Small frame appearance differences are accepted;
  changes to menus, hints, shortcuts or form transactions are not implicitly accepted.
- Start with locked crates.io dependencies. If standard framing or composition
  cannot meet functional requirements, stop that phase with a reproducible
  limitation and seek an owner decision. No silent fork, cache edit, dependency
  upgrade, second frame or return to custom ordinary controls.

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
| Custom frame and Linux window controls (`chrome.rs`) | Root frame and supported title-bar controls | Replace; checkpoint resize/tiling/platform behavior first |
| Zoom halves and `pressed_zoom` (`plot_ui.rs`, shell routing) | Button with shared group styling | Replace; remove custom pressed/release/reset state; retain geometry, translucency and enabled rules |
| Main cascading menu (`app_menu.rs`, `app_menu_ui.rs`) | PopupMenu and supported composition | Prototype one-level Escape, hover switching, keyboard selection, viewport placement; unresolved mismatch needs separate owner decision |
| Waveform/spectrum splitter (`shell.rs`, `panels.rs`) | Resizable panels/handle | Compare both orientations, minimum sizes, 1-pixel separator, persisted fraction and non-restarting resize; replace if compatible, otherwise justify a narrow handle |
| Toolbar, status range/FFT, start/recent buttons | Existing Button | Retain standard behavior; consolidate supported styling where useful; preserve hover/pressed colors and focus/disabled states |
| Settings form | Existing NumberInput / Select / Button | Retain; prove in-window compatibility without changing #108 workflow here |
| Ruler context menu | Existing PopupMenu | Retain and verify focus return/navigation isolation |
| Metadata/analysis/shortcut hints and keycaps | Existing Tooltip / Kbd | Keep standard composition; centralize surface contracts, not per-call-site guards |
| Spectrogram, waveform, axes, minimap and cursor badges | Existing domain canvas/texture rendering | Retain domain drawing; document domain requirement and interaction boundaries; ordinary controls over it remain standard |
| Rejected numeric editor in PR #106 | NumberInput / Input | Do not port; reconnect #108 only after the infrastructure is ready |

Inventory all current render and input entry points before claiming completeness;
append discovered controls to this table with their disposition.

## Rejected alternatives

- Keep Shell unwrapped and indefinitely retain separate settings windows: does not
  remove the infrastructure obstacle to #108 or reduce duplicated control behavior.
- Nest Root under Shell or around one input: does not satisfy the top-level lookup.
- Wrap the existing custom frame in Root: doubles frame ownership.
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

- [ ] Capture the current native interaction/geometry baseline and finish the inventory.
- [ ] Build a bounded verification fixture using top-level Root, its frame, a plot
      canvas, standard numeric input/select in a popover, a menu and a passive hint.
- [ ] Verify frame operations and the event matrix, including the stock menu Escape
      mismatch and passive-hint pointer/wheel distinction. Inspect locked toolkit
      APIs before choosing any replacement or exception.
- [ ] Present native evidence and remaining limitations. Do not start broad migration
      if functional frame requirements fail; obtain decisions on any UX tradeoffs.

### 2. Root and single-frame migration

- [ ] Adopt Root in the main window and remove duplicate custom frame/inset handlers.
- [ ] Update typed window handles, Shell root lookups, theme/font setup, Tab traversal,
      ready-status observation and window-level actions without duplicate registration.
- [ ] Preserve native title, activation, file chooser/drop behavior, session geometry
      and the existing Root-backed settings window. Verify a current release build.

### 3. Plot ownership and overlay isolation

- [ ] Introduce PlotView with bounded render inputs and typed navigation intents;
      move plot focus, pointer/readout and gesture state out of Shell.
- [ ] Route keyboard navigation through plot focus and commands through explicit
      targets; keep required symbolic-key normalization scoped to plot input.
- [ ] Preserve accepted snapshot labels, resize mailbox behavior and texture retirement.
- [ ] Apply the surface contracts centrally and test dismissal, release outside,
      focus loss, overlay opening during drag and document replacement.
- [ ] Verify #122's cursor/guide reproduction and compatibility requirements; link
      the evidence without claiming unrelated hint-contrast work is complete.

### 4. Standard-control replacement

- [ ] Replace zoom halves with standard Buttons and remove their obsolete state paths.
- [ ] Replace menu/splitter implementations where the checkpoint proved compatibility;
      obtain and record per-control decisions for any retained custom implementation.
- [ ] Audit remaining forms, title/status/start controls, hints and keycaps; keep
      standard controls and remove unnecessary duplicated interaction/style machinery.
- [ ] Complete the inventory with replacement evidence or accepted exceptions.
- [ ] Remove temporary prototype UI; retain a focused verification fixture or tests
      that exercise standard in-window inputs without shipping another settings surface.

### 5. Integration and handoff

- [ ] Update AGENTS current status to the implemented ownership (remove old Root ban),
      relevant documentation and user-visible changelog entries only for shipped changes.
- [ ] Run the full validation below and external class-A review; resolve substantive
      findings within the repository's three-round limit.
- [ ] Move this plan to `docs/plans/completed/` before final owner review, only when
      every in-scope implementation and validation task is complete.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (workspace warnings denied)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Toolkit-free tests for navigation intents, geometry, persistence and existing
      analysis invariants; GPUI-level tests where supported for focus/action routing.
- [ ] Native input matrix on Linux, Windows and macOS: plot, main/context menu,
      passive hint, popover, nested select and separate settings window; click,
      double-click, wheel, drag/release outside, arrows, symbols, Alt guides,
      Tab/Shift+Tab, Escape/Enter and focus restoration.
- [ ] Native text editing: caret movement, selection, clipboard, undo/redo, IME,
      invalid numeric input, blur/Enter validation and disabled controls. Verify
      editing shortcuts neither navigate the plot nor get swallowed by plot interceptors.
- [ ] Native frame matrix: dark/light, narrow/wide, display scale, restore/maximize,
      fullscreen, supported tiling, move/resize edges/corners, title-bar controls,
      native dialogs and file opening. Mark platform-inapplicable cases explicitly.
- [ ] Both plot orientations, grid/scale toggles, start/loaded states, progressive
      updates, splitter resize during analysis, settings preview/cancel and file replacement.
- [ ] Compare representative current-release resize/navigation responsiveness and
      texture behavior with the baseline; investigate regressions rather than attributing
      them to extraction. No extra transforms or retained image backlog on resize.
- [ ] Record exact build, platform and exercised cases in the PR. No unit-test or
      CI-build result substitutes for native event verification. Unavailable required
      platform coverage remains outstanding; do not mark the gate complete.
- [ ] Final independent review clean, review conversations resolved and `ci/full`
      successful for the current up-to-date revision on Linux, Windows and macOS.

## Post-completion

Resume #108 on the standard infrastructure with standard text controls; explicitly
reconcile its obsolete no-Root constraint. Keep its product interaction work and
review separate, and do not merge PR #106's rejected numeric editor. Close #122 only
with linked evidence for all its criteria. #123 remains independently tracked.
