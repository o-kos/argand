# Issue #132: Fixed minimap height instead of a splitter

Resolves [#132](https://github.com/o-kos/argand/issues/132).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 5 (standard-control
replacement), inventory row "Waveform/spectrum splitter".
Predecessor [#131](131-stock-application-menu.md), merged in PR #160.

## Overview

The boundary between the full-capture waveform minimap and the spectrogram is a
hand-built splitter: a 5-pixel invisible strip over the 1-pixel separator
(`PlotView::splitter`), a `splitter_dragging` flag, `drag_splitter` on every
pointer move, a `PlotIntent::WaveformFraction` and a persisted
`Session::waveform_fraction`. The Issue asked to replace it with the standard
`ResizablePanelGroup`.

The owner decided on 2026-09-28 that the minimap is not resizable at all. Its size
is fixed at 3 rem by default (48 logical pixels with the default font), and set in
the configuration since owner feedback 1: its height in horizontal orientation and
its width in vertical orientation. Removing the control is the maximum reduction of custom
interaction the inventory asks for, so no standard replacement is needed.

Boundaries: the minimap's drawing, navigation, the 1-pixel separator, the spectrum
layout below it and the analysis pipeline do not change. No session version bump.

Implementation class: **B**, declared before implementation: a removal across
`plot_view.rs`, `plot_ui.rs`, `navigation_ui.rs`, `panels.rs` and `session.rs`
with local, known invariants, touching no worker, generation or texture path.

Roles, set by the owner on 2026-09-28: implementer **Claude in session**; reviewer
**Codex** `gpt-6-sol`, reasoning effort `medium`.

## Context

- `panels::waveform_height(total, rem, fraction, scale)` returns 3 rem without a
  fraction and clamps an adjusted one; both `Shell::measure_time_scheme` and the
  plot canvas call it with `session.waveform_fraction`.
- `Session` has no `deny_unknown_fields`, so a session file that still carries
  `waveform_fraction` loads with the field ignored once the field is gone.
- `[panels].waveform_fraction` in the configuration is already legacy: readable,
  repaired, and not used to size the panel.
- The locked `gpui-base` 0.6.6 `ResizablePanelGroup` was read before this decision
  (default minimum 100 pixels, pixel sizes in its own state, a 1-pixel handle with
  4-pixel padding); it is not used.

## Decisions

- **The minimap has a fixed size across**, high or wide by orientation, 3 rem by
  default and set in the configuration (owner feedback 1), rounded to device
  pixels as today.
  `panels::waveform_height` loses its fraction parameter.
- **Delete the splitter:** `PlotView::splitter`, `drag_splitter`,
  `splitter_dragging` and every check of it, `PlotIntent::WaveformFraction` and its
  Shell handler, and `PlotSnapshot::fraction`.
- **Delete `Session::waveform_fraction`** and its load-time filtering. An old
  session with the field loads and ignores it; a test proves that. The session
  version stays 10, because the layout gains nothing a reader must know about.
- The boundary keeps no resize cursor; the pointer over it is whatever the minimap
  or the spectrum shows there.

## Rejected alternatives

- Standard `ResizablePanelGroup`: the owner does not want the boundary to move.
- Keep reading the saved fraction: a remembered size the user can no longer change
  would be a height nobody can fix.

## Implementation steps

- [x] Remove the splitter control, its drag state, intent and handler.
- [x] Fix the minimap at a set size in `panels.rs` and its callers; update its tests.
- [x] Remove `Session::waveform_fraction`; test that an old session carrying it loads.
- [x] Update AGENTS.md, the parent inventory row and phase-5 item, and CHANGELOG.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Native check on Linux (owner): both orientations, the minimap at its default and a configured size with a
      session that had an adjusted split, no resize cursor on the boundary, a press
      on the boundary behaves as the minimap or spectrum beneath it, window resize
      keeps the minimap height.

## Review round 1

Codex `gpt-6-sol` medium found no leftover code path and no geometry or pointer
regression, and four documentation and test items, all accepted. The README user
guide still described dragging the boundary and is rewritten. The parent plan's
native checklist asked for a splitter resize during analysis and now asks for the
fixed minimap and its boundary in both orientations. The compatibility test now
loads every readable session version with no split, a valid one and a NaN one,
and checks that the geometry survives the write-back without the field. The
CHANGELOG entry names the width that is fixed in vertical orientation.

Round 2 added two accepted items: the README still mentioned dragging the panel
separator in its redraw paragraph, and several texts said the minimap is 3 rem
high in both orientations where vertical orientation fixes its width. Both are
fixed.

Round 3 found no substantive issue. The owner then tried 4 rem natively and
accepted it; the constant, its tests and every text that named 3 rem changed with it.

## Owner feedback 1 (2026-09-29)

- [x] **The size is a configuration option.** The owner asked for the minimap size in
      the configuration, with its unit written out. `[panels].minimap_size` in
      `argand.toml` takes `"<number> rem"` (1 to 20, following the interface font)
      or `"<number> px"` (16 to 320 logical pixels), with or without a space and in
      any case, default `"3 rem"` after the owner tried 4 rem and chose to keep
      3 rem as the default. The value must be quoted, because TOML cannot read a
      bare `3 rem`; the owner kept that over accepting a bare number as rem, and
      the distributed template's comment says so. `panels::MinimapSize` parses and prints it and
      converts it to logical pixels. An unusable value is logged and replaced by
      the default alone, keeping the rest of the file, as the other repaired keys
      do, including a value that is not a string at all. The unit is taken from
      the end, so `1e1 rem` reads as 10 rem. The size reaches the plot through `PlotSnapshot::minimap_size`. There is
      no settings-window control. Tests cover both units, spacing and case,
      out-of-range, missing and unknown units, the round trip through `Display`,
      the fallback that keeps the rest of the file, and the distributed template.

A targeted check of this change found three items, all accepted and fixed: a
non-string value discarded the whole configuration, `1e1 rem` was refused, and
two plan lines still said the minimap is fixed at 4 rem.

## Post-completion

- Continue with #133 (integration and handoff).
