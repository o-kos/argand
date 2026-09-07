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

## Rejected alternatives

- Adding a second frame would duplicate borders, shadows and resize handlers.
- Changing dependency sources or editing the registry would not be a portable fix.

## Implementation steps

- [ ] Reproduce the frame cursor and title-bar double-click failures.
- [ ] Fix cursor regions and rounded frame geometry without duplicate decoration.
- [ ] Centre and constrain the title; preserve dragging, double clicks and controls.
- [ ] Set the waveform placeholder height to 64 pixels and align #31's requirements.
- [ ] Update configuration documentation, changelog and architectural instructions.
- [ ] Complete validation and external review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] Targeted tests for any extracted frame/layout policy.
- [ ] Fresh-binary interaction checks: each resize edge/corner, entering content,
  File menu, title-bar dragging, double-clicks on both halves, and window controls.
- [ ] Visual checks: normal/maximized geometry, both themes, long title, 64-pixel strip.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] External review with the required model and effort returns no substantive findings.

## Post-completion

Merge through a Pull Request after acceptance and required checks; clean the accepted
branch according to `CONTRIBUTING.md`.
