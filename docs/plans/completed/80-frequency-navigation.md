# Frequency navigation (#80)

## Design

Keep the frequency viewport independent of the time range and session state. Use
physical frequency bounds from the capture metadata, with a minimum span of one
retained overview frequency cell, subject to physical-coordinate precision. Frequency navigation rebins the retained overview
on the document worker without reading samples or repeating FFTs. Paint held images
in the requested two-dimensional viewport immediately, retaining a wider backdrop
until the matching view arrives. The minimap continues to cover the full capture.

Ctrl+Shift+wheel zooms frequency about the pointer; Shift+wheel pans frequency over
the spectrum and time ruler. The frequency ruler supports dragging, wheel panning,
and Ctrl+wheel zoom. Ctrl+Shift+Up/Down/Home zooms/fits frequency; Up/Down and
Ctrl+Up/Down pan by one/five frequency divisions. Existing time controls stay intact.
Every newly opened file resets both viewports.

## Implementation

- [x] Add bounded frequency viewport arithmetic and regional overview rendering.
- [x] Coalesce frequency display requests without invalidating analysis.
- [x] Map held pictures and cursor levels in both dimensions, with aligned rulers.
- [x] Add frequency gestures, actions, menu entries and conditional hand cursor.
- [x] Cover physical bounds, reducers, stale deliveries, gestures and reset behavior.
- [x] Run the full local gate, rebuild release, and exercise native GPU navigation.
- [x] Complete independent review, resolve findings, and finish this plan.

## Validation

Exercise real and complex captures, including negative frequencies and an offset
center frequency; zoom at either edge, pan to both bounds, fit, reopen, resize, and
change time zoom while the frequency viewport is narrowed. Verify cached navigation
keeps FFT counts and analysis generation unchanged. Check pointer guides, ruler
resolution hints and level lookup against the visible physical frequency interval.

## Results

The complete local gate passed (527 tests), and the release binary was rebuilt.
Native Linux GPU checks covered real and I/Q captures, a 14 MHz center offset,
frequency zoom/pan/fit, both frequency limits, ruler dragging, Shift+wheel, Alt
readouts and reopening through Recent without restarting. A 1 GB capture also
passed repeated Home/End jumps between one-FFT time windows with frequency zoom;
no image drawing errors were logged. Frequency-only gestures kept the analysis
completion count unchanged; a separate time zoom correctly triggered one analysis.

Independent review identified a missing overlap guard for distant held images.
Both ordinary and deep-preview painting now reject nonintersecting time/frequency
windows before producing GPU coordinates. No findings were declined; the final
review round was clean. Native checks here cover Linux; cross-platform compilation
and tests remain part of the full CI gate after owner acceptance.
