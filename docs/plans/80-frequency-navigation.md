# Frequency navigation (#80)

## Design

Keep the frequency viewport independent of the time range and session state. Use
physical frequency bounds from the capture metadata, with a minimum span of one
retained overview frequency cell. Frequency navigation rebins the retained overview
on the document worker without reading samples or repeating FFTs. Paint held images
in the requested two-dimensional viewport immediately, retaining a wider backdrop
until the matching view arrives. The minimap continues to cover the full capture.

Ctrl+Shift+wheel zooms frequency about the pointer; Shift+wheel pans frequency over
the spectrum and time ruler. The frequency ruler supports dragging, wheel panning,
and Ctrl+wheel zoom. Ctrl+Shift+plus/minus/0 zooms/fits frequency; Up/Down and
Ctrl+Up/Down pan by one/five frequency divisions. Existing time controls stay intact.
Every newly opened file resets both viewports.

## Implementation

- [ ] Add bounded frequency viewport arithmetic and regional overview rendering.
- [ ] Coalesce frequency display requests without invalidating analysis.
- [ ] Map held pictures and cursor levels in both dimensions, with aligned rulers.
- [ ] Add frequency gestures, actions, menu entries and conditional hand cursor.
- [ ] Cover physical bounds, reducers, stale deliveries, gestures and reset behavior.
- [ ] Run the full local gate, rebuild release, and exercise native GPU navigation.
- [ ] Complete independent review, resolve findings, and finish this plan.

## Validation

Exercise real and complex captures, including negative frequencies and an offset
center frequency; zoom at either edge, pan to both bounds, fit, reopen, resize, and
change time zoom while the frequency viewport is narrowed. Verify cached navigation
keeps FFT counts and analysis generation unchanged. Check pointer guides, ruler
resolution hints and level lookup against the visible physical frequency interval.
