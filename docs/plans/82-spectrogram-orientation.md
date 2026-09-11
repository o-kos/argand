# Spectrogram orientation (#82)

## Design

Expose a visible Horizontal/Vertical button beside View and persist its active
mode in the session. Missing mode defaults to Horizontal. Vertical mode places
time from top to bottom and frequency from left to right, with the full-capture
minimap in a vertical strip on the left. Switching orientation preserves the
current physical time/frequency ranges and does not repeat FFTs.

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

- [ ] Add a toolkit-neutral orientation model and backward-compatible session state.
- [ ] Add the visible mode button and preserve physical viewports on switching.
- [ ] Orient images, held/backdrop mappings and deep-preview strips.
- [ ] Orient ruler layout, labels, units, grid and Alt badges.
- [ ] Orient minimap/splitter geometry and all pointer/navigation hitboxes.
- [ ] Test coordinate mapping, pixel order, gestures and session compatibility.
- [ ] Run the complete local gate and rebuild release.
- [ ] Exercise both modes on native GPU with real/IQ captures, navigation and restart.
- [ ] Complete independent review and finish the plan.
