# Issue #49: Window frame interaction and title bar layout

Resolves [#49](https://github.com/o-kos/argand/issues/49).

## Overview

Fix resize cursors, centre the title, round normal window corners, reserve a
3-rem waveform strip, and restore double-click handling across the title bar.

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
- Keep the current placeholder 3 rem high (48 logical pixels at the default 16-pixel font size); record the same starting
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
- [x] Set the waveform placeholder height to 3 rem and align #31's requirements.
- [x] Update configuration documentation, changelog and architectural instructions.
- [x] Complete validation and external review.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Targeted tests for any extracted frame/layout policy.
- [x] Fresh-binary interaction checks: each resize edge/corner, entering content,
  File menu, title-bar dragging, double-clicks on both halves, and window controls.
- [x] Visual checks: normal/maximized geometry, both themes, long title, 3-rem strip.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] External review with the required model and effort returns no substantive findings.

## Initial validation evidence

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

## Owner refinements

- [x] Use `h_12` for the waveform strip: 3 rem, twice the `h_6` status row.
  This is 48 logical pixels at the default font size and follows UI/DPI scaling.
- [x] Paint the Linux Close hover/press background with the frame's top-right radius.
  Keep Windows/macOS toolkit controls and the whole-bar title overlay.
- [x] Raise File from extra-small (12) to small (14), matching popup menu text.
- [x] Split metadata into bordered fields with tooltips; spell `iq · i16` and show
  duration as minutes:seconds.milliseconds. Preserve optional centre frequency.
- [x] Validate refinements at normal/high DPI.
- [x] Repeat external review of the refinements.

## Refinement validation evidence

- Formatting, strict Clippy and all 369 tests pass; the release binary was rebuilt
  afterwards. Duration tests cover I/Q frame counts, real samples, millisecond
  precision and rounding across minute/hour boundaries.
- At scale 1, separators are at y=134 and y=182 (48 physical/logical pixels).
  At scale 2, they are at y=268 and y=364 (96 physical, 48 logical pixels).
  Both 1000x700 and 640x400 logical windows keep this height.
- All eight resize directions and return-to-content Arrow cursors pass at both
  scales. File opens at the left edge with text matching the popup, while the
  title remains centred. An initial missing flex container moved File to the
  centre during development; the fresh-binary check caught it and it was fixed.
- Weston checks perform three maximize/restore pairs: left/centre/right title
  double clicks and the maximize button. Each restores the original rectangle,
  none emits a resize request, and Minimize emits its expected request.
- A 24 kHz I/Q WAV with 5,312,160 frames shows `3:41.340`. In the light theme,
  all five fields including `12.579 MHz` fit at the minimum width. Tooltips show
  each field's explanation at both scales and in both themes. Close hover and
  pressed backgrounds follow the
  rounded normal-window corner; maximized controls have square corners.
- External review found that right-clicks on window controls opened the system
  menu. The finding was accepted: the handler now belongs only to the left
  title-bar region, whose height spans the bar. Protocol checks reproduce one
  menu request from each control before the fix and zero afterwards; left, centre
  and right title areas still send one request each. No finding was declined.
  The final review round returned no substantive findings.
- Windows/macOS interactions remain untested locally; their existing toolkit
  controls are retained. Cross-platform compilation is covered by CI.

## Post-completion

Merge through a Pull Request after acceptance and required checks; clean the accepted
branch according to `CONTRIBUTING.md`.

## Status wording refinement

- [x] Separate sample domain and storage format with a middle dot (`iq · i16`).
- [x] Use explanatory tooltips without repeating the displayed value: file
  container type, samples format, signal sample rate and signal duration (m:ss.ms).
- [x] Complete local validation, fresh release build and focused external review.

Formatting, strict Clippy and all 369 tests pass. A fresh release window shows
`iq · i16` and explanatory tooltips without values. The focused external review
returned no substantive findings. Remote CI runs in the background; its result
must be checked on the final commit before merge.
