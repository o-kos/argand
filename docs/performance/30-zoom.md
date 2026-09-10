# Zoom replacement cost (#30)

Owner testing rejected the earlier navigation redraw despite the previous native
smoke checks. The former path replaced held detail with 128 sparse frames, then
refined it and resolved the final colour scale. Even a one-frame view rendered a
full pixel-width image, duplicated in a preview and a final delivery.

## CPU measurements

Linux, Intel Core i5-1235U, release mode, five repetitions per case, warm file
reads, FFT 2048 / hop 512, Max, default absolute dynamic range, a 1500 x 700
spectrogram and 1500 waveform columns. The table uses the eight-worker pool.
Opening/normalization, UI scheduling, GPU upload and presentation are excluded.
These are local medians, not latency guarantees.

| Capture | FFT frames | Previous preview + final | Final-only view |
| --- | ---: | ---: | ---: |
| Real m39 | 1 | 23.44 ms | 0.36 ms |
| Real m39 | 16 | 22.33 ms | 6.04 ms |
| Real m39 | 128 | 28.14 ms | 9.74 ms |
| Real m39 | 512 | 50.79 ms | 16.24 ms |
| IQ i16 HFDL | 1 | 24.89 ms | 0.53 ms |
| IQ i16 HFDL | 16 | 27.90 ms | 7.22 ms |
| IQ i16 HFDL | 128 | 39.86 ms | 13.17 ms |

Raw one/eight-worker runs and the before/after harnesses are in
[30-zoom-data](30-zoom-data/). `publish=true` asks the callback to render;
the final-only run calls it zero times. The harness is a temporary Cargo binary
with path dependencies on `crates/{core,dsp,io}` and Rayon. Copy the repository
lockfile and run it offline in release mode; adjust the local fixture root in the
source. Both final paths retain overview data for subsequent resize/style changes.

The serial FFT threshold is capped at 128 frames and also by the configured
batch's decoded-memory and FFT-work bounds. Parallel image preparation remains
available; a single-threaded FFT does not require disabling it.

## Coverage and limitations

Regression tests compare final-only and progressive results for real/IQ,
Max/MeanPower, 1/16/128/257 frames, offset ranges and a trailing waveform peak.
They check frame counts, PSD, grid values, colour bytes, waveform extrema,
single sequential reading, bounded cancellation, compact GUI replies, progress-only
notifications, incomplete-backdrop cursor exclusion, widest-picture retention and cached
resize/style changes. A broader retained picture provides data only where it
has previously been computed. A retained sparse preview is an approximate visual placeholder; it has no
numeric cursor level. Time outside the retained picture extent stays uncovered.
Only one widest picture is retained, not an unbounded history of every visited range.

Native inspection found two additional filtering hazards: atlas-edge contamination
of one-column textures, and interpolation between different FFT frames in compact
multi-column textures. Texture edges are now replicated and clipped. Only a single
FFT frame stays compact; multi-frame images keep pixel-width cells and reuse
identical column calculations/shading. The table above measures this corrected implementation.

Native GPU checks before the review follow-up covered real Max, IQ MeanPower and
a one-hour real recording, keyboard zoom to one FFT and back, and early navigation
that cancels initial refinement. They sampled 35 frames per zoom-out sequence and
20 frames for the early interruption; every sampled spectral column retained signal
pixels, with no blank-area replacement. Single-frame views had constant horizontal
colour away from grid lines after edge padding. These are sampled screencopy frames,
not an exhaustive record of every GPU presentation or a test of the owner's desktop.

The first review requested four changes. Incomplete backdrops now have no numeric
readout, style-only changes shade retained values off the UI thread and coalesce the
foreground delivery for a paired swap, narrower panned ranges cannot replace the
widest picture, and long navigation publishes nonblocking progress-only updates.
Keeping the already-shown sparse picture as a visual placeholder is deliberate;
it is not promoted to measured cursor data. Follow-up GPU checks passed for the hour-long real capture and IQ MeanPower.
A grayscale change on a zoomed real capture followed by zoom-out sampled 20 frames:
all sampled columns remained covered, and none of the sampled spectral pixels
retained the previous coloured palette. The current full gate passes 469 tests;
the release binary was rebuilt after it. Raw frame counts, selected traces and
representative PNGs are in `30-zoom-data`. Native runs used local virtual-keyboard
and screencopy helpers; the evidence is not a standalone compositor setup script.
The second review found that a backdrop upgrade could discard a foreground delivery
parked for a style swap. Retention now waits while that swap owns the delivery;
a regression test covers wider-picture and completion upgrades. The third review
returned no remaining substantive findings.

The interaction follow-up passed on the rebuilt native application: ordinary wheel
scrolling and left-dragging on the time ruler changed the start sample while keeping
106176 visible samples; wheel scrolling over the spectrogram changed the span.
Cursor screenshots showed an arrow over the waveform and time ruler, and a crosshair
over the spectrogram. Closing after zoom omitted the view from the session file.
Injecting a legacy 2048-sample saved view and restarting still displayed the full
58.987-second capture. The measured ranges are in `ruler-navigation.json`.

The separate interaction review returned no substantive actionable defects.
