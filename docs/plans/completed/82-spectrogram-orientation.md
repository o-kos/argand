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
