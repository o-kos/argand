# Spectrogram orientation (#82)

## Design

Expose a visible Horizontal/Vertical button beside View and persist its active
mode in the session. Missing mode defaults to Horizontal. Vertical mode places
time from top to bottom and frequency from left to right, with the full-capture
minimap in a vertical strip on the left. Switching orientation preserves the
current physical time/frequency ranges and does not repeat FFTs, except when
a longer time axis requires the existing representability floor to expand an
extreme-index range. Keep that correctness guard and its normal analysis restart.

Keep source grids in their toolkit-neutral time/frequency order. An orientation
model maps physical fractions, axis lengths, pointer deltas and rectangles to the
screen. Rotate uploaded image pixels for vertical display, including held images
and deep-preview strips, with the existing delayed texture retirement. Cache
redraw dimensions still name time columns and frequency rows.

Rulers, units and their resolution hints, grid strokes, cursor values, Alt guides,
minimap navigation and splitter dragging all follow the chosen layout. Wheel
semantics remain axis-based (time pan, Ctrl time zoom, Shift frequency pan,
Ctrl+Shift frequency zoom); each ruler pans/zooms its own axis. Arrow panning follows
screen direction in each orientation. Zoom shortcuts retain their named axes.

## Implementation

- [x] Add a toolkit-neutral orientation model and backward-compatible session state.
- [x] Add the visible mode button and preserve physical viewports on switching.
- [x] Orient images, held/backdrop mappings and deep-preview strips.
- [x] Orient ruler layout, labels, units, grid and Alt badges.
- [x] Orient minimap/splitter geometry and all pointer/navigation hitboxes.
- [x] Test coordinate mapping, pixel order, gestures and session compatibility.
- [x] Run the complete local gate and rebuild release.
- [x] Exercise both modes on native GPU with real/IQ captures, navigation and restart.
- [x] Complete independent review and finish the plan.

## Review decisions

- The first review identified an unconditional range-preservation claim that
  ignored the existing time-coordinate precision floor. Keep the shared resize
  guard: bypassing it would make adjacent coordinates indistinguishable at
  extreme sample indices. Document the exceptional expansion and recalculation
  in this plan, README and AGENTS rather than weakening the guard. The second
  review confirmed this decision and identified the same unconditional wording
  in CHANGELOG; its release note now states the exception too. The final
  wording-only round was clean.

## Validation

- Formatting, strict Clippy and the full local suite passed (535 tests).
- The release binary was rebuilt from the implementation.
- Native Linux GPU checks covered real and I/Q captures, mode/restart persistence,
  both rulers and wheel modifiers, screen-relative arrows, minimap and splitter
  dragging, unit hints, context menus, Alt guides, grid visibility and resizing.
- A 1 GB real capture exercised repeated Home/End transitions at the one-FFT
  time span, narrow frequency views, deep-strip rotation and resize. No image
  errors were logged. Ordinary orientation switches did not repeat analysis.
- Native Windows/macOS checks were not available locally; full three-platform CI
  runs when the Draft is accepted for final review.

## Owner follow-up: readable frequency resolution

- [x] Select frequency resolution units from Hz per device pixel, independently
  of the ruler caption; trim redundant decimal zeros and retain numeric locale
- [x] Cover sub-hertz values, unit thresholds and localized output
- [x] Run local checks and focused independent review
- [x] Rebuild release from the final code

Focused review found redundant zeros in scientific frequency mantissas. The fix
trims only the mantissa, preserving exponent and integer zeros; time hints retain
their previous output. The final round was clean. All 536 tests, formatting and
strict Clippy passed.

## Owner follow-up: frequency mouse gestures

- [x] Reproduce missing spectrum frequency drag and Ctrl+Shift+wheel on native Linux
- [x] Accept Shift-remapped wheel deltas and enable both spectrum drag axes
- [x] Submit one request after updating both viewports during diagonal drags
- [x] Cover remapped wheel events and constrained ruler/minimap hitboxes
- [x] Run full local gate, release, native gesture checks and focused review

All 538 tests, formatting and strict Clippy passed; release rebuilt. Native Linux
GPU events reproduced both failures before the fix and verified Ctrl+Shift+wheel
on spectrum and both rulers, Shift+wheel, diagonal dragging in both orientations,
frequency dragging with fitted time, ruler constraints and release of the drag.
Frequency-only motion retained analysis counters. Focused independent review
was clean, with no findings to accept or decline.

## Owner follow-up: restore frequency zoom shortcuts

- [x] Reproduce Ctrl+Shift+plus incorrectly changing the time viewport
- [x] Restore plus/minus frequency bindings and remove Up/Down zoom alternatives
- [x] Resolve physical Shift before action matching, scoped to Plot focus
- [x] Cover shifted symbols, keypad aliases and unrelated modifiers in tests
- [x] Complete local gate, release, native top-row/keypad checks and focused review

Native testing also caught inherited Plot bindings firing time zoom behind the
time-ruler popup. Descendant focus now consumes zoom keys without dispatching
a plot action; windows without Plot context are untouched.

All 539 tests, formatting and strict Clippy passed; the final release was rebuilt.
Native Linux GPU tests passed in both orientations for top-row and keypad
Ctrl/Shift zoom, exactly one axis update per keystroke, removed arrow alternatives,
menu/settings focus isolation and restored plot focus after menu dismissal.
Focused independent review was clean, with no findings to accept or decline.

## Owner follow-up: grid shortcut

- [x] Bind Ctrl+G to the existing ToggleGrid action in Plot context and document it
- [x] Run local gate, rebuild release, verify native toggling and complete focused review

Focused review identified inherited Plot bindings toggling the grid behind an
open popup. Ctrl+G now uses the existing plot-shortcut focus guard; popup/input
focus consumes it without dispatch, while direct menu actions remain unchanged.

All 539 tests, formatting and strict Clippy passed; release rebuilt. Native Linux
GPU checks verified both grid states in both orientations, persistence, matching
menu checkmarks and Ctrl+G label, popup focus isolation and unchanged analysis
counters. The final focused review was clean.

## Owner follow-up: restore the left-side vertical minimap

- [x] Revert the right-side minimap experiment, including geometry and splitter direction
- [x] Keep unit-caption placement unchanged pending a separate layout discussion
- [x] Run the full local gate, rebuild release and complete focused review

The owner preferred the original left-side minimap after trying the right-side
layout. Frequency-unit placement remains a design discussion, not part of this revert.

Formatting, strict Clippy and all 539 tests passed; release rebuilt. Focused
independent review confirmed that application code exactly matches the previous
left-side layout, documentation is consistent and no caption redesign was added.
The review was clean. No new native GUI run was needed for this exact restoration.

## Owner follow-up: remove the vertical top gap

- [x] Reproduce the four-logical-pixel inset in the release and geometry test
- [x] Align vertical spectrum and minimap to the panel top without changing unit placement
- [x] Run local gate/release, verify native rendering and complete focused review

The vertical layout retained an explicit outer top margin. The shared frame now
starts at zero in both orientations; the existing vertical ruler test checks this
at display scales 1, 1.25 and 2.

All 539 tests, formatting and strict Clippy passed; release rebuilt. Native Linux
GPU before/after captures show the four background rows replaced by spectrum
pixels. Zoom and the Alt time badge near the top edge were checked. Focused review
confirmed shared minimap alignment, retained edge-label filtering and unchanged
horizontal geometry; no findings remained.
