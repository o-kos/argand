# Issue #158: Title-bar toolbar gaps swallow window drag and double-click

Resolves #158.

## Overview

The `title-toolbar` container in `crates/app/src/app_menu_ui.rs` stops left
mouse-down and double-click for its whole box through `.occlude()` and container
level handlers, so the few bare pixels between the application button, the
orientation segments, Grid and the separator neither activate a control nor move
or maximize the window. The drag itself lives one level up: the `title-bar` div
in `shell.rs` arms `title_drag_pending` on a left press and calls
`start_window_move()` on the following move, and zooms the window on a double
click.

The change moves the consumption from the container to the controls. The
container keeps only its right-press suppression, so bare pixels inside the
toolbar bubble to the title bar and take the platform move and zoom behaviour,
while each control is wrapped to stop a left press and a double click before
they can arm anything.

## Context

- gpui-component 0.6.6's `Button` does not stop propagation on a left press
  (`src/button/button.rs:784`): it calls `window.prevent_default()` to avoid
  focus and suppresses text selection, and stops propagation only while loading.
  The container's handlers are therefore the only thing today that keeps a press
  on the application button, a segment or Grid from reaching the title bar's
  drag arming, which parent plan #124 case R3 forbids.
- `.occlude()` on the container has been there since the toolbar's first
  appearance (#95) with no recorded reason of its own. The application menu
  isolates its input itself: the open menu occludes and draws a window-level
  backdrop, both documented in `AGENTS.md` and covered by
  `the_backdrop_keeps_the_window_beneath_the_menu`.
- The separator and the `gap_1()` spacing between the container's children are
  container pixels, not control pixels, and the issue names them as pixels that
  should drag.
- The real window move and maximize cannot be proven in CI. The headless test
  asserts the arming state the title bar's drag reads, `Shell::title_drag_pending`,
  which the existing test module already reaches. That state machine is
  Linux-only (`shell.rs` builds the dragging title bar under
  `cfg!(target_os = "linux")`; the other platforms leave moving and zooming to
  the native title bar), so the test is gated the same way. The production
  change is platform-neutral: the gaps fall through to whatever owns the title
  gestures on each platform, and the controls consume their own presses
  everywhere.

## Decisions

- Consumption moves to per-control wrappers rather than staying on the
  container. The container cannot both cover its gaps and consume only its
  controls, because the gaps are its own box; a box is either stopped or passed
  through.
- A `title_control` helper wraps the application popover, the orientation group
  and Grid. On a left press it calls `window.prevent_default()` and stops
  propagation, and it stops a double click, which is exactly the treatment a
  press on those pixels received from the container handler before, so control
  behaviour is unchanged.
- The wrapper covers each control's whole element, including the orientation
  group's frame padding. The frame is part of the control; the pixels between
  controls are what the issue asks to free.
- The container keeps its right-press stop. The title bar shows the window menu
  from its own bare regions, and the toolbar suppressed the right press over its
  box before; nothing in this issue changes that.
- `.occlude()` is removed. The menu, its backdrop and the passive tooltips carry
  their own input isolation, and the existing menu and hint tests must hold
  without it.
- Native verification of the actual move and maximize on Linux stays with the
  owner, per the owner's Linux-only decision of 2026-09-29 recorded in the #133
  plan.

## Rejected alternatives

- Keeping the container as an occluding box and attaching the title-bar drag
  machine to it. The controls do not stop their own presses, so the container
  would arm on a control press exactly as the title bar would, and per-control
  wrappers become necessary anyway. Duplicating the pending state on the
  container adds a second drag machine for no gain.
- Chaining handlers onto the returned builders. `Button`, `ButtonGroup` and
  `Popover` in gpui-component 0.6.6 are `IntoElement` wrappers around a private
  render and expose no arbitrary pointer handlers, so a wrapping element is the
  supported composition.
- Letting the controls keep bubbling and filtering by geometry at the title bar.
  That couples the title bar to the toolbar's layout and re-implements hit
  testing the toolkit already does.

## Implementation steps

- [x] Wrap the three toolbar controls with a helper that stops a left press and
      a double click, with `prevent_default` on the press as before.
- [x] Remove the container's `.occlude()`, its left-press stop and its
      double-click stop, keeping the right-press stop.
- [x] Cover the geometry with a headless test: a press in the gap between the
      application button and the orientation group arms the title drag, presses
      on the three controls never do, and the arming clears on release.
      Mutation-check that the test fails with the container restored.
- [x] Update `docs/ui/124-inventory.md` rows I07 and the R-disposition text for
      the moved entry points, re-running its recorded audit command.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [x] The existing toolbar and menu tests hold unchanged, including
      `the_backdrop_keeps_the_window_beneath_the_menu`,
      `the_document_controls_join_the_toolbar` and
      `the_title_reserves_exactly_what_the_toolbar_draws`.

## Post-completion

- The owner verifies natively on Linux that a drag from a toolbar gap moves the
  window and a double click maximizes it, and that pressing a control still
  never starts a move.
- Class B. Implementer: this session. Reviewer: GPT-6 Sol at medium reasoning
  effort through the `codex` CLI, agreed with the owner.
