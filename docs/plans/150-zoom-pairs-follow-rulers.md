# Issue #150: Place each zoom button pair beside its own ruler in both orientations

Resolves [#150](https://github.com/o-kos/argand/issues/150).

## Overview

The translucent `[+|-]` zoom pairs over the spectrogram (#99) sit in the same
corners in both orientations: the time pair bottom-left, laid out horizontally,
and the frequency pair top-right, laid out vertically. That matches horizontal
orientation. In vertical orientation time runs down the right ruler and
frequency along the bottom, so each pair sits beside the other axis's ruler.

Place each pair beside the ruler of the axis it zooms:

- horizontal orientation (unchanged): time pair bottom-left along the bottom
  ruler, frequency pair top-right along the right ruler;
- vertical orientation: time pair top-right along the right ruler, laid out
  vertically; frequency pair bottom-left along the bottom ruler, laid out
  horizontally.

Actions, tooltips, keycaps, dimensions, the 8-pixel inset, the minimum span,
the shared visibility of both pairs, the arrow cursor and the gesture exclusion
stay as they are.

Implementation class: **B**. A local GUI placement fix in one module with
concrete geometry. It is implemented in-session at the owner's request. The
reviewer model is agreed with the owner before any review round (interim
process, #147).

## Context

- `plot_ui.rs::corner_zones(spectrum, visible)` returns `[time, frequency]`
  zones in fixed corners. `plot_geometry` stores them in
  `PlotGeometry::zoom_zones`.
- `zoom_pair` already derives its layout from the zone's shape
  (`width > height` means `[+|-]` in a row, otherwise a column), so a zone in
  the other corner lays itself out correctly without further changes.
- `PlotGeometry::controls_at` and `over_scale_buttons` treat both zones alike,
  regardless of which axis they zoom, so the gesture exclusion needs no code
  change. It gains a test in vertical orientation.
- #130 will replace the custom halves with standard `Button`s. It is blocked by
  #129 and not underway; it inherits the placement rule from `corner_zones`.

## Decisions

- `corner_zones` takes the orientation. It computes the bottom-left row zone
  and the top-right column zone once, and assigns them to `[time, frequency]`
  by the orientation. The pair order and index meaning of `zoom_zones` stay
  `[time, frequency]`, so `ruler_zoom_buttons` keeps binding the time actions
  to index 0.
- The bottom-left zone is always a row and the top-right zone always a column:
  each pair runs along its ruler.

## Rejected alternatives

- Rotating only the pair's layout while keeping the corners: the pair would
  still sit beside the other axis's ruler, which is what the Issue reports.

## Implementation steps

- [ ] Make `corner_zones` place the pairs by orientation.
- [ ] Test `corner_zones` placement and layout in both orientations.
- [ ] Test the gesture exclusion (arrow cursor, no drag) with vertical-orientation zones.
- [ ] Update the "identically in both orientations" statement in `AGENTS.md`.
- [ ] Add a `CHANGELOG.md` entry.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Owner check in the release binary: both orientations, Ctrl+T switching,
      Ctrl+U hiding, tooltips and clicks on each pair.

## Post-completion

None.
