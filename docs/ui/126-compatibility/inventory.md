# UI control and interaction inventory

Scope: the production `crates/app/src` tree at the start of #126. No production
control has been replaced by this checkpoint. The example is verification tooling,
not a new production settings surface. Parent: [approved plan](../../plans/124-standard-ui-architecture.md).

## Reproducing the inventory

From the repository root, search both explicit handlers and extension methods:

```sh
rg -n 'impl Render|fn render\(|Button::new|NumberInput::new|Select::new|PopupMenu|Tooltip::|\.on_(mouse|scroll|key|action|click|hover|drop)|intercept_keystrokes|on_mouse_event|observe_keystrokes' crates/app/src
rg -n 'canvas\(|\.context_menu|\.tooltip|hoverable_tooltip|track_focus|focus_handle|window_control_area|observe_window|on_release' crates/app/src
```

Explicit render/control/input handlers occur in `shell.rs`, `chrome.rs`,
`plot_ui.rs`, `navigation_ui.rs`, `app_menu_ui.rs`, `settings_ui.rs` and
`settings_editor.rs`. Drawing and lifecycle helpers below cover the related
`axes.rs`, `cursor_guides.rs`, `waveform.rs`, `spectrogram.rs`, `backdrop.rs` and
`shortcuts.rs` responsibilities. Toolkit-neutral models are not additional widgets.
Search results are an entry-point inventory, not proof that native behavior passes.

## Production inventory

| ID | Owner / entry point | Current category and behavior | Candidate / disposition | Required evidence or retained responsibility |
| --- | --- | --- | --- | --- |
| C01 | [chrome.rs](../../../crates/app/src/chrome.rs), `Frame::for_window/render` | Custom client frame, shadow, inset and edge hitboxes | Replace with Root frame in #127 | R1/R2: viewport vs restore geometry, expanded/tiling regions, scale, edges/corners; stock-frame compatibility is unverified |
| C02 | `chrome::controls` | Custom div minimize/maximize/close with hover/active backgrounds | Standard TitleBar controls, #127 | R3: actions once, no title dragging, platform controls and close corner; no custom replacement authorized here |
| C03 | [shell.rs](../../../crates/app/src/shell.rs), `title_bar` | Linux move/release/outside handlers and context window menu; stock TitleBar on other platforms | Supported TitleBar move/zoom/window-menu behavior, #127 | Toolbar hit testing must prevent unintended window move/maximize |
| C04 | [app_menu_ui.rs](../../../crates/app/src/app_menu_ui.rs), `toolbar`, toolbar button helpers | Standard Button, custom variant, icon/group styling; anchor/title-hitbox guards | Retain Button; audit styles in #130 and title integration in #127 | Normal/hover/pressed/disabled/focus, full-window title alignment, trigger bounds |
| C05 | `app_menu_ui`, `ApplicationMenu`, `application_menu_panel/row/key`, [app_menu.rs](../../../crates/app/src/app_menu.rs) | Custom cascade state, div rows, scroll/placement and overlay focus/occlusion | Stock PopupMenu composition where possible, #131 | Stock Cancel recursively closes parents; accepted one-level Escape is a demonstrated source-level mismatch. Candidate is not yet native-verified; any retained navigation must be separately approved |
| C06 | `app_menu_ui::NoTooltip` | Empty Render adapter suppressing the pressed application-button hint | Inspect supported tooltip visibility API when replacing menu; do not treat as a custom button | Preserve tooltip lifecycle while menu is open; keep only a minimal adapter if needed |
| C07 | [navigation_ui.rs](../../../crates/app/src/navigation_ui.rs), `time_context_menu/time_scale_items` | Standard PopupMenu with retained entity, action context and dismiss subscription | Retain, #129/#131 | F3: correct focus/Alt-feedback restoration without an extra pointer movement |
| C08 | [plot_ui.rs](../../../crates/app/src/plot_ui.rs), `half_button`, corner pair rendering | Custom div zoom halves and Shell `pressed_zoom` lifecycle | Standard Buttons in a styled group, #130 | Geometry/translucency, each zoom action once, tooltip, disabled, release outside, hide/show and chooser reset paths |
| C09 | `shell::splitter/drag_splitter/finish_splitter`, [panels.rs](../../../crates/app/src/panels.rs) | Custom drag handle; domain size constraints and persisted fraction | ResizablePanelGroup/ResizablePanel, #132 | Both orientations, 1-pixel separator, minimum/default/restored size, live size observation, no analysis restart; stock on_resize is release-only in inspected source |
| C10 | `shell::recent_button` and start-page chooser | Standard Button styled to text width; shared opening actions | Retain, #130 | Focus/keyboard opening, numbered recent order, hover width and narrow layout; availability model is not a widget |
| C11 | [settings_ui.rs](../../../crates/app/src/settings_ui.rs), `range_control`, analysis summary | Standard Button in actionable states; informational div otherwise, tracked hover foreground | Retain standard behavior, #130 | Accepted warning/neutral normal-hover-pressed colors, pending/failed non-actionability, no action from informational states |
| C12 | `settings_ui::analysis_tooltip`, live owner observation | Standard Tooltip with content rows and standard recommendation/edit Buttons | Retain composition; central surface policy in #129 | Owner-live values, row alignment, pointer/Alt and input isolation; no migration to #108 workflow in this issue |
| C13 | `shell::metadata_tooltip/shortcut_tooltip`, `plot_ui::unit_tooltip` | Standard Tooltip; custom measurement/aligned content and unit hitboxes | Retain shared helpers; central contracts in #129 | Every hint owns arrow/readout behavior; #122 click/wheel compatibility; #123 contrast remains independent |
| C14 | [shortcuts.rs](../../../crates/app/src/shortcuts.rs) | Standard Kbd with display-name/style/measurement adapter | Retain composition | Keep typography/whitespace and platform symbols; no separate keyboard implementation |
| C15 | [settings_editor.rs](../../../crates/app/src/settings_editor.rs), Editor Render and fields | Standard NumberInput, Select and Button in a separate Root-backed window | Retain; standalone popover probe establishes possible future composition | Caret/selection/IME, validation, reset/preview/commit/cancel, Tab and nested Escape; product transactions stay unchanged |
| A01 | `shell::run`, `bind_choose_file`, settings action handlers | Root identity/window-handle assumptions and application-level Open/Settings/range routing | Explicit weak-Shell/Root-window bridge, #127 | Correct owning window, editor-local precedence, closed-window no-op; no leaked strong owner |
| A02 | `shell::ready_input_observer`, retained `observe_keystrokes` subscription | Passive window capture canvas and keystroke observer | Preserve passive observation using supported GPUI hooks; migrate identity lookup in #127/#129 | O1: sees consumed menu input for status dismissal but never consumes or navigates |
| A03 | `navigation_ui::init`, `plot_shortcut`, `navigation_actions` | Application-registered symbolic-key interception plus Shell-wide Plot action context | Plot-scoped routing, #128 | Normalize only when PlotView owns focus; Input/Select/IME and text shortcuts must not be swallowed |
| A04 | `shell::middle/render`; navigation pointer/pan/wheel helpers | Plot-specific start/wheel handlers, shell-wide move/release handlers and geometry filtering | PlotView focus/hitboxes/gesture lifecycle, #128/#129 | F1/P1/P3: exposed target only, release outside, lost focus, overlay activation, replacement |
| A05 | `shell::run`, external-path drop, window activation/appearance/bounds and recent refresh callbacks | Application lifecycle and platform hooks | Keep scoped application responsibilities, #127/#128 | No added UI-thread filesystem work, duplicate listeners or accidental plot commands; preserve configuration/session semantics |
| A06 | Editor release/focus/subscription callbacks | Form lifetime and transactional settings adapter | Retain with standard fields, #127/#129 | Closing/cancelling/restoring and opening another file; do not substitute focus loss for accepted transaction semantics |
| D01 | `plot_ui::spectrogram`, [axes.rs](../../../crates/app/src/axes.rs), [cursor_guides.rs](../../../crates/app/src/cursor_guides.rs) | Domain canvas measuring/painting calibrated axes and guides | Retain GPUI canvas and domain models; move interaction ownership, #128 | Physical units, common plot/axis geometry, both orientations, Alt precision and clipping; standard buttons/menus do not implement RF axes |
| D02 | [waveform.rs](../../../crates/app/src/waveform.rs), `minimap_panel`, navigation geometry | Domain envelope painting/minimap hit testing | Retain drawing; PlotView owns gestures, #128 | I/Q envelope, full-capture extents, viewport indication and minimap navigation; not a replacement for an ordinary widget |
| D03 | [spectrogram.rs](../../../crates/app/src/spectrogram.rs), [backdrop.rs](../../../crates/app/src/backdrop.rs), deep-preview helpers | GPUI RenderImage/paint_image adapters and retained domain textures | Retain Shell resource ownership, #128 | G1: snapshot replacement before two-callback window-specific retirement, bounded caches and no stale painting |

No additional control is introduced by notice text, form-row layout, frame corner
geometry, menu item models, recent-file availability or toolkit-free navigation
models. These are content/layout/domain helpers of the entries above. PR #106's
rejected numeric editor is not in this production tree and must not be ported.

## Locked-toolkit evidence and open decisions

Paths below are relative to the gpui-component 0.5.1 source located by Cargo, not
an instruction to modify the registry. Inspect the exact lockfile version rather
than current upstream documentation when repeating the checkpoint.

| Observation | Source | Consequence (not a native pass) |
| --- | --- | --- |
| Inputs require window's top-level Root | `root.rs` read/update; `input/element.rs` and `input/state.rs` | Retain InputState under real Root in the fixture; wrapping only a subtree is not sufficient |
| Root always composes window_border | `root.rs`, Render | Remove custom frame only with migration; do not nest both frames |
| Stock resize hitbox uses window_bounds size and has no explicit maximized guard | `window_border.rs`, Render | R2 is a go/no-go check; do not assume our viewport/expanded fixes are preserved |
| Stock popover supplies focus, cancellation, deferred placement and occlusion | `popover.rs`, Popover/PopoverState | Test standard composition first; retained input/select entities must not be recreated in the content closure |
| Both Popover and Select introduce deferred overlays | `popover.rs`, RenderOnce; `select.rs`, popup rendering | Native opening of Select inside Popover panics in the locked graph; see [reproduction](native-results.md). Do not infer that a top-level Root alone makes this composition usable |
| Stock menu Cancel recursively dismisses parents | `menu/popup_menu.rs`, dismiss | One-level Escape needs supported composition or a separately approved exception; do not quietly weaken UX |
| ResizableState publishes sizes and notifies on resize; on_resize runs on mouse-up | `resizable/mod.rs`, sizes/resize_panel; `resizable/panel.rs`, ResizePanelElement paint | Observe state notifications for live dimensions; release-only callback cannot alone replace current continuous display resizing |
| ButtonCustomVariant has foreground/background/border/hover/active styling | `button/button.rs` | Standard composition is the first candidate; verify painted foreground and nested icon states, not only API presence |
| NumberInput emits Step events rather than owning numeric policy | `input/number_input.rs`, NumberInputEvent | Use a small validated application-value adapter, as the existing settings editor does; this is not a reason to hand-write text editing |
| Full occlusion blocks wheel as well as pointer hitboxes | GPUI 0.2.2 `window.rs`, HitboxBehavior | #122 needs independent pointer and click/wheel evidence; blanket occlusion may change its contract |

The standard-control candidates are not accepted exceptions. Native checks and
owner decisions remain required before keeping custom controls or changing UX.
