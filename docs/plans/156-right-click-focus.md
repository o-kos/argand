# Issue #156: A right click on a toolbar or status-bar button takes keyboard focus from the plot

Resolves #156.

## Overview

gpui-component 0.6.6's `Button` calls `window.prevent_default()` only on a left
mouse-down (`src/button/button.rs:784`), so GPUI's default mouse-down focusing
still applies when the right or middle button presses one. A right click on an
Argand toolbar or status-bar `Button` therefore moves keyboard focus to that
button, even with `tab_stop(false)`, and plot keys stop working until the plot
is clicked again. #128 established that clicks on these buttons never take
focus; that held only for the left button.

The change prevents the focusing default for the right and middle press on the
toolbar trio and the two status-bar items, through the same wrapping composition
the toolbar already uses.

## Context

- `Button`'s render is private and exposes no pointer-handler passthrough, so
  the prevention cannot be attached to the button itself; a wrapping element is
  the supported composition, as `title_control` already does for the toolbar's
  drag consumption (#158).
- The toolbar controls are already wrapped by `title_control`
  (`app_menu_ui.rs`), which consumes left press and double click for the title
  drag. The status-bar FFT summary (`analysis-settings`) and the actionable
  range item (`analysis-range`) are placed as plain children of the status bar.
- The non-actionable range item is a plain `div`, which the issue does not name.
- The zoom halves over the plot carry the same theoretical defect, but their
  composition shares `flex_1` and the pair frame's rounded-corner state, and the
  issue does not name them; they are recorded here rather than silently widened.
- The plot owns keyboard focus while a document is shown (#128), and
  `Shell::focus_target` is how every control returns it.

## Decisions

- A `keeps_focus(id, child)` wrapper prevents the focusing default through
  `capture_any_mouse_down`, named for what it keeps. `title_control` builds on
  it and adds its drag consumption, so the toolbar trio gets the prevention from
  the composition it already has and the helper stays single-purpose.
- The prevention runs in the capture phase, not the bubble phase, and the
  difference is why the first attempt failed. The core's focus transfer listens
  on the bubble phase and fires only while the default is not yet prevented
  (`gpui-pre-0.3.6/src/elements/div.rs:2765`), and the bubble dispatch walks the
  listeners in reverse registration order
  (`gpui-pre-0.3.6/src/window.rs:5779`), so the button's own transfer runs
  before any ancestor's bubble handler could answer. The capture phase walks
  root to leaf first, so the wrapper's prevention is already in place when the
  button's transfer looks. The wrapper's press still reaches the button's own
  handlers, which is why the pinned hint's right-press toggle keeps working.
- The prevention covers every press but the left one, and the distinction is
  not optional. The pressed surface a control paints settles in a bubble
  listener gated on the same default-prevented flag the focus transfer checks
  (`gpui-pre-0.3.6/src/elements/div.rs:3251`), and a capture-phase prevention
  for the left press would arrive before that listener and drop the pressed
  highlight from Grid, an unselected orientation segment and the actionable
  range item. The toolkit's own left-press handler prevents the focusing
  default on the button itself, so the left press needs nothing here.
- No `stop_propagation` in the helper. The toolbar container already stops right
  presses itself (#158 kept that), and the status bar has nothing beneath these
  items that a right press would disturb; propagation is a separate question
  from the focusing default, and only the default is the defect.
- The helper lives in `app_menu_ui.rs` beside `title_control`, visible to
  `settings_ui.rs` as a sibling under `shell`, which is how the modules already
  share `pub(super)` helpers.
- The FFT summary's wrapper sits around the whole pinned-hint trigger, because
  `hints::pinned` takes a typed `Button` and cannot be given a wrapped one; a
  capture handler on the box covers the button inside it wherever it sits.
- The pinned hint opening on the right press takes the keyboard while it is
  open, which is its designed behaviour and not the defect; what the test
  asserts there is that the toggle still reaches the button through the wrapper
  and that the plot has the keyboard back once the hint closes.

## Rejected alternatives

- Fixing it in the toolkit. The locked gpui-component 0.6.6 is a crates.io
  dependency; the wrapper is the composition the repository's conventions
  prefer, and a `Button` pointer-handler passthrough would be an upstream
  change for a three-line local fix.
- `tab_stop(false)` is already set on every one of these buttons and does not
  cover click focusing, which is the defect.
- Wrapping the zoom halves. The pair frame sizes its halves by `flex_1` and
  rounds the outer corners through the button's own variant, and a wrapper
  between them changes that geometry; the issue does not name them, so widening
  the diff there is not justified by the acceptance criteria.

## Implementation steps

- [x] Add the `keeps_focus` wrapper and build `title_control` on it.
- [x] Wrap the status-bar FFT summary and the actionable range item.
- [x] Cover it with a headless test: with the focus target held, middle and
      right presses on the application button, an orientation segment, Grid, the
      actionable range item and the FFT summary leave the focus target where it
      was, the FFT summary's right press still toggles the pinned hint through
      the wrapper, a left press on Grid still acts, and the focus target holds
      to the end. Mutation checks cover both the prevention removed and the
      status-bar wrapper removed. The harness has no PlotView, so the focus
      target is the shell handle, and the pressed surface itself is paint the
      harness cannot see; both limits are stated rather than papered over.
- [x] Update `docs/ui/124-inventory.md` for the moved and added entry points,
      re-running its recorded audit command.
- [x] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The existing toolbar, status-bar and focus tests hold unchanged.

## Post-completion

- The owner verifies natively on Linux that a right or middle click on each
  named control leaves the plot working, per the owner's Linux-only decision of
  2026-09-29 recorded in the #133 plan.
- Class B. Implementer: this session. Reviewer: GPT-6 Sol at medium reasoning
  effort through the `codex` CLI, agreed with the owner.
