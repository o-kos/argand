# Minimap drag latency investigation (#84)

The application-only experiments reduce CPU work but fail the motion criterion.
A diagnostic GPUI immediate-presentation hook meets the horizontal real-signal
motion target in two runs. It is not a production fix or full acceptance of #84;
PR #93 remains a draft investigation.

## Method and limits

Use a private Sway Wayland session with a headless 1600 × 1000 output, scale 1,
backed by the Intel GPU at `/dev/dri/renderD128`. The application uses its native
GPUI/Blade renderer. This is hardware rendering, but it is not a physical-display
input-to-photon measurement. The output reports zero for its refresh field;
observed Wayland frame callback intervals have 5th/50th/95th percentiles of
16/16/17 ms. Do not describe that field as a measured monitor refresh rate.

Open `tests/signals/m39.wav`, use a 1500 × 900 window and horizontal orientation,
fit time, then zoom five times (32×). The detected bright viewport is 47 physical
pixels wide, including edge pixels. Grab its centre and move by 12 logical pixels
every 10 ms, repeatedly reversing in the capture interior. Capture for 6.5 seconds
with `grim -c`; the cursor and content come from the same compositor capture.
No compiler runs concurrently with these measurements.

The offset metric follows the original issue: absolute change in the distance
from the first white cursor pixel to the bright viewport's left edge, relative
to the stationary grab. It is a screen-space proxy. Capture timing and compositor
scheduling affect the distribution. Correcting the detected cursor coordinate by
the calibrated five-pixel hotspot offset does not change the outside counts in
these runs. Capture counts differ because screenshot acquisition is not fixed-rate.

## Experiments

| Variant | Frames | Median error | P95 | Maximum | Cursor outside |
| --- | ---: | ---: | ---: | ---: | ---: |
| Accepted orientation implementation | 153 | 24 px | 36 px | 48 px | 62 |
| Baseline with CPU phase probes | 199 | 24 px | 36 px | 48 px | 89 |
| Prepare a scene after each minimap motion event | 198 | 24 px | 36 px | 48 px | 81 |
| Cache the waveform as dark/bright textures, run 1 | 196 | 24 px | 48 px | 48 px | 79 |
| Same texture cache, run 2 | 175 | 24 px | 48 px | 48 px | 67 |
| Cache + early scene + FFT request on release | 197 | 24 px | 24 px | 25 px | 57 |
| Same combination + immediate GPUI presentation, run 1 | 199 | 0 px | 12 px | 13 px | 0 |
| Same immediate-presentation prototype, run 2 | 200 | 0 px | 12 px | 13 px | 0 |
| Immediate presentation with ordinary during-drag FFT requests | 180 | 0 px | 12 px | 24 px | 1 |

Raw positions and summary measurements are in [84-data](84-data/).
The baseline and cache escape screenshots preserve the visual failure.
Neither the early scene nor the cache alone fixes the issue. Differences in
outside-frame percentages are not evidence of a reliable improvement.

## CPU work versus presentation

Opt-in `argand::ui_latency=trace` probes measure CPU scopes, not presentation.
The instrumented baseline's waveform paint takes 1338 µs at the median and
1516 µs at P95. Cached texture painting takes 8 µs and 18 µs respectively.
Axes painting remains approximately 0.35 ms at the median. Pointer handling is
approximately 20–25 µs. Shell rendering has a long tail when it prepares a new
spectrogram texture (approximately 4.5 ms at P95).

Matching each processed minimap motion to the first following paint of that
requested start gives a median of 9.33 ms and P95 of 20.48 ms in the instrumented
baseline. The combined experiment gives 0.82 ms and 0.96 ms respectively. All 639
processed moves have a matching paint in the combined run; ordinary frame
coalescing means only 357 baseline moves do. These scopes end at the beginning of
waveform painting, before the rest of the scene and GPU presentation. They are
not end-to-end input latency. The matching summaries are recorded separately in
[84-data/event-first-paint.json](84-data/event-first-paint.json).

GPUI 0.2.2's public `Window::draw` prepares the scene and marks it for presentation;
it does not submit it to the platform renderer. Its private `Window::present`
is called by the platform frame callback. `WaylandWindow::frame` requests the next
surface frame callback and invokes this rendering path. Therefore, preparing a
scene earlier does not by itself submit a buffer earlier. This source inspection
identifies a scheduling boundary; it does not prove that every remaining pixel
of lag originates there.

## Review and remaining work

Review confirmed the private/public API boundary and did not establish a new
texture-retirement defect. It identified three limits that remain relevant:

- Early submission may block later input in swapchain acquisition/GPU waits;
  `Window::defer` releases the entity borrow but does not leave the platform input
  callback. Calls in this prototype are not coalesced, including unchanged moves
  at capture bounds. Short pointer-handler scopes exclude the deferred drawing.
- Screenshot sampling is irregular (the combined run's median interval is
  32.43 ms, maximum 147.16 ms). It does not separately measure input delivery,
  presentation or physical display latency. GPUI requests `DisplaySync::Recent`,
  but the selected WSI mode was not recorded. The controlled immediate-present
  comparison supports the scheduling hypothesis in this environment, not a
  universal attribution of all lag to GPUI or a conversion of pixels to milliseconds.
- Cached images and old quads round fractional device bounds differently. The CPU
  raster test does not establish GPU equivalence at fractional/high display scales.

The combined and immediate-presentation prototypes passed formatting, strict
Clippy, 541 workspace tests and release builds. The two immediate-presentation
runs meet the stated horizontal real-signal motion criterion; the variant with
ordinary during-drag FFT requests has one cursor escape and does not meet it.
Both orientations, I/Q input, high display scale, cancellation and outside release
still need validation for any production correction. The FFT-on-release prototype
has a known missing-mouse-up dispatch gap and is not suitable to ship.

The owner declined connecting a custom GPUI dependency. All experimental runtime
changes have therefore been removed from the working application; registry
manifests and lockfile remain unchanged. Reproduction-only patches and their
limitations are in [84-data/prototypes](84-data/prototypes/README.md).

Issue #84 remains open and PR #93 remains Draft. No application-only correction
meeting the motion target has been established. A future supported early-submission
API or a different validated solution is needed before completing this issue.
