# Issue #132: Fixed minimap height instead of a splitter

Resolves [#132](https://github.com/o-kos/argand/issues/132).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 5 (standard-control
replacement), inventory row "Waveform/spectrum splitter".
Predecessor [#131](completed/131-stock-application-menu.md), merged in PR #160.

## Overview

The boundary between the full-capture waveform minimap and the spectrogram is a
hand-built splitter: a 5-pixel invisible strip over the 1-pixel separator
(`PlotView::splitter`), a `splitter_dragging` flag, `drag_splitter` on every
pointer move, a `PlotIntent::WaveformFraction` and a persisted
`Session::waveform_fraction`. The Issue asked to replace it with the standard
`ResizablePanelGroup`.

The owner decided on 2026-09-28 that the minimap is not resizable at all. It keeps
the height it has by default today, 3 rem (48 logical pixels with the default font),
in both orientations. Removing the control is the maximum reduction of custom
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

- **The minimap height is always 3 rem**, rounded to device pixels as today.
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
- [x] Fix the minimap at 3 rem in `panels.rs` and its callers; update its tests.
- [x] Remove `Session::waveform_fraction`; test that an old session carrying it loads.
- [x] Update AGENTS.md, the parent inventory row and phase-5 item, and CHANGELOG.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Native check on Linux (owner): both orientations, the minimap at 3 rem with a
      session that had an adjusted split, no resize cursor on the boundary, a press
      on the boundary behaves as the minimap or spectrum beneath it, window resize
      keeps the minimap height.

## Post-completion

- Continue with #133 (integration and handoff).
