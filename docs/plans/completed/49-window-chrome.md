# Issue #49: Window frame interaction and title bar layout

Resolves [#49](https://github.com/o-kos/argand/issues/49).

## Overview

Fix resize cursors, centre the title, round normal window corners, reserve a
64-pixel waveform strip, and restore double-click handling across the title bar.

## Context

The current shell uses gpui-component's `Root`, window border and `TitleBar`.
The border chooses a cursor during painting from the current pointer position
and hard-codes a zero corner radius. The title is grouped with File, and the
waveform placeholder consumes a configurable fraction of the content height.
Related work: #39 owns button hover/press restyling; #31 owns the future waveform.

## Decisions

- Interpret dimensions as logical pixels, consistent with GPUI layout.
- Use subtle 8-pixel corners for ordinary client-decorated windows, respecting
  maximized, fullscreen and tiled edges.
- Keep toolkit types inside `argand-app` and preserve native platform controls.
- Keep the current placeholder exactly 64 pixels high; record the same starting
  height in #31 without implementing waveform rendering or a splitter here.

- `Shell` replaces the toolkit `Root` as the root entity; it preserves theme font
  setup and Tab traversal. Existing buttons and popup menus do not depend on
  `Root`; future dialog/sheet/input components require explicit integration.
- Resize grips extend 6 pixels inside each free visible edge because a compositor
  may exclude the shadow from pointer input. Expanded/tiled edges have no grips.
- The title is a non-interactive sibling overlay over the whole title bar, with
  equal 128-pixel gutters; the toolkit's internal child region excludes controls.
- Existing `panels.waveform_fraction` configuration remains accepted but no longer
  sizes the placeholder, so older configuration files still load.

## Rejected alternatives

- Adding a second frame would duplicate borders, shadows and resize handlers.
- Changing dependency sources or editing the registry would not be a portable fix.

## Implementation steps

- [x] Reproduce the frame cursor and title-bar double-click failures.
- [x] Fix cursor regions and rounded frame geometry without duplicate decoration.
- [x] Centre and constrain the title; preserve dragging, double clicks and controls.
- [x] Set the waveform placeholder height to 64 pixels and align #31's requirements.
- [x] Update configuration documentation, changelog and architectural instructions.
- [x] Complete validation and external review.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Targeted tests for any extracted frame/layout policy.
- [x] Fresh-binary interaction checks: each resize edge/corner, entering content,
  File menu, title-bar dragging, double-clicks on both halves, and window controls.
- [x] Visual checks: normal/maximized geometry, both themes, long title, 64-pixel strip.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] External review with the required model and effort returns no substantive findings.

## Validation evidence

Formatting, strict Clippy, all 367 local tests and the subsequent release build pass.


- The original build retained cursor shape 29 after entering content in an isolated
  Sway session. Its fullscreen right-hand title-bar clicks sent
  `xdg_toplevel.resize(edge=8)`, using the saved restore width as a resize boundary.
- Sway does not maximize a floating window, so a nested Weston 13.0 desktop was
  used for actual maximize/restore transitions. The original build sent spurious
  resize requests there too, although Weston still restored it. The fixed build
  maximizes and restores from the left, centre and right title-bar areas without
  any resize request and restores the original rectangle.
- Final grip checks in both themes exercise all eight directions: each changes
  the expected edges, and returning to content selects cursor shape 1 (Arrow).
  File opens its popup, dragging the title moves the window, and Close closes it.
- At 1000x700 and 640x400, screenshot separators are at y=134 and y=198: exactly
  64 pixels apart in both themes, even with legacy `waveform_fraction = 0.5`.
  The short title's ink is centred within one pixel of the window centre;
  glyph side bearings account for that subpixel/layout difference. A long filename
  remains inside the equal title gutters, ends with an ellipsis at the minimum
  window size and does not overlap File or controls.
- The isolated compositors use Mesa 25.2.8 on Intel Iris Xe (ADL GT2), not software
  rendering. Windows and macOS interaction is not exercised on this Linux host;
  their unchanged toolkit controls were reviewed statically and CI covers builds.
- External review accepted the frame/root integration but found the initial title
  offset. That finding was accepted and fixed by moving the title out of the
  toolkit's asymmetric child region into a whole-bar sibling overlay. No finding
  was declined. The final review round returned no substantive findings.

## Post-completion

Merge through a Pull Request after acceptance and required checks; clean the accepted
branch according to `CONTRIBUTING.md`.
