# Issue #127: Adopt borderless Root with single Argand frame ownership

Resolves [#127](https://github.com/o-kos/argand/issues/127).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 3.
Blocked by [#137](137-gpui-kit-migration.md), which landed in PR #138.

## Overview

Make GPUI Kit's `Root` the root entity of the main application window with
`Root::bordered(false)`, keeping `chrome.rs` as the sole frame, inset, shadow,
resize and title-bar owner. `Root` strongly owns `Entity<Shell>` as its child;
everything that assumed the window root was a `Shell` moves to explicit
`WeakEntity<Shell>`/`WindowHandle<Root>` handles with safe no-ops for closed
windows. No behavior change beyond the ownership itself: no second frame
layer, no duplicated focus or action handlers, settings window untouched.

## Context

- After #138 the graph runs on `gpui-kit` 0.6.6 / GPUI 0.3.6; the fixture
  proves `Root::bordered(false)` beside the stock composition and frames its
  borderless window with the production `argand::chrome::Frame` (library
  target `argand`, `pub mod chrome`).
- Current production wiring (`crates/app/src/shell.rs`):
  `cx.open_window(options, ...)` roots the window directly at `Shell`, the
  returned handle is `WindowHandle<Shell>`; `Shell::bind_choose_file` holds
  that handle for three app-level actions (ChooseFile, UseRecommendedRange,
  EditAnalysis); `dismiss_window_ready_status` reaches the shell through
  `window.root::<Self>()` from a capture-phase canvas mouse observer.
- `chrome::Frame::for_window(window)` + `frame.render(content, cx)` own
  padding, corner radii, shadow and resize regions; `window.set_client_inset`
  is called from there. Root must add nothing on top.
- The settings editor window already uses a Root-backed composition
  (`settings_window: Option<gpui_kit::WindowHandle<gpui_kit::component::Root>>`)
  and is out of scope beyond regression checks.
- The rejected stock frame from #126/#135 stays rejected; the system
  decoration alternative remains rejected by #135.

Implementation class: **A**. Per the owner's 2026-09-22 session decision the
implementation is done in-session and reviewed by the owner directly; the
external class-A model pair is not used.

## Decisions

- The window root becomes `Root`; `Shell` is created first as a child entity
  and handed to `Root::new(shell, window, cx).bordered(false)`. The returned
  `WindowHandle<Root>` is dropped once bindings are installed: nothing
  downstream needs it, because updates go through `WeakEntity<Shell>`.
- App-level action handlers capture `WeakEntity<Shell>` and the originating
  `WindowHandle<Root>`; updates go through the weak entity, so a closed
  window or dropped shell is a safe no-op instead of a failed root lookup.
- Correction found during review: `App::observe_keystrokes` fires for every
  window, not only the one that registered it, so a ready-status observer
  that captures a fixed `WeakEntity<Shell>` at construction dismisses the
  wrong window's banner as soon as a second window exists (the settings
  editor). The observer resolves the shell through the window the event hit
  instead, via the same centralized `Shell::dismiss_window_ready_status`
  helper the pointer/wheel interceptor already uses.
- `chrome.rs` keeps calling `window.set_client_inset` and rendering resize
  regions; Root is configured with `bordered(false)` and nothing else, so no
  second frame, inset or hit layer exists.
- Focus and key contexts stay as they are (`FocusNext`/`FocusPrevious` and
  plot bindings on the existing contexts); Root's focus infrastructure joins
  the chain, and any handler it would duplicate is removed rather than kept
  in parallel.
- Documentation: the AGENTS.md status description is updated to the
  Root-owned reality on this branch, since AGENTS.md ships in the same PR
  and must stay synchronized with the code it describes.

## Rejected alternatives

- Keeping the shell as root and rendering a Root inside it: Root is designed
  as the window root; nesting it duplicates focus/overlay layers and was the
  shape #126 rejected.
- Adopting the stock frame or system decorations: rejected by #126/#135;
  `chrome.rs` remains the sole frame.
- Reaching the shell through `window.root::<Root>()` plus a child accessor:
  keeps hidden coupling to the root type at every call site; weak entities
  are explicit and safe by construction.

## Implementation steps

- [x] Start the focused branch and open a Draft PR closing this issue.
- [x] Root adoption in `shell::run`: create `Entity<Shell>` first, wrap in
      `Root::new(...).bordered(false)`; the startup handle is `WindowHandle<Root>`
      and is not retained further (the shell's own window id serves updates).
- [x] Move ChooseFile / UseRecommendedRange / EditAnalysis dispatch to
      `WeakEntity<Shell>` + `WindowHandle<Root>` with safe no-ops.
- [x] Rewire the ready-status dismissal to resolve through the window an
      event hit rather than a captured weak entity (see the corrected
      decision above); remove the `window.root::<Self>()` assumption.
- [x] Audit remaining root-type assumptions (grep `root::<`, `WindowHandle`),
      register every observer and action exactly once, keep editor-local
      precedence for Settings/Range actions.
- [ ] Verify focus traversal, popup dismissal and title-bar drag against the
      Root chain; remove any handler Root makes redundant. In particular,
      `Root` now binds its own `tab`/`shift-tab` keys (`Tab`/`TabPrev`,
      context `"Root"`) alongside Shell's existing `FocusNext`/`FocusPrevious`
      (context `"Shell ..."`); confirm one Tab press advances focus by one
      element, not two, now that both contexts are simultaneously active.
- [x] Update AGENTS.md architecture status and the fixture/README wording to
      the Root-owned reality.
- [ ] Complete the validation matrix and move this plan to
      `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in
      `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Native (owner, Wayland CSD first): R1–R3 frame matrix
      (restore/maximize/fullscreen/tiling, edge and corner resize without
      stale regions or doubled insets, title-bar drag and the accepted
      maximize double-click), O1 overlay isolation, S1 settings window with
      standard fields, commands dispatched from plot, menu and editor focus
      and after window closure; session geometry, window title, activation,
      file chooser and drag-and-drop preserved; toolbar controls do not start
      a window move or maximize.

## Post-completion

- Update the parent #124 progress/evidence rows for stage 3.
- The native-evidence ledger feeds the follow-up stages (#128 plot ownership,
  #129 overlay isolation) which build on the Root-owned window.
