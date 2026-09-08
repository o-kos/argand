# Reusing analysis across plot sizes (#74)

## Data and rendering policy

The GUI retains up to 4096 time cells and 2048 frequency cells, preserving native
FFT bins when they fit. Every STFT frame contributes once. Up to 1024 sparse
preview frames (including the first 128) precede sequential refinement; this
budget is independent of the cache dimensions. A separate min/max envelope keeps
up to 65536 sample cells. There is no capture-length-sized FFT store or canonical
RGBA image. The dominant retained spectral accumulator is at most 32 MiB for Peak
or 64 MiB for Mean power, plus waveform, PSD, FFT and display buffers.

Rendering rebins values before shading. Peak conservatively includes every cell
that overlaps a display pixel. Mean power weights each cell's linear-power sum
by the fractional time overlap and the number of overlapping frequency bins,
then divides by the correspondingly weighted frame count. Unequal groups do not
receive equal weight. Waveform extrema include every overlapping sample cell.
Aligned boundaries reproduce the direct reduction up to floating-point rounding;
inside a cache cell, power is assumed uniform and extrema can extend a feature by
one cache cell at either edge. Enlarging beyond cache resolution adds no sub-cell
detail. Ordinary DSP/CLI reduction still uses exact requested pixel boundaries.

Changed cache columns and their empty followers update only dependent output
columns during refinement. A changed size rebuilds the display grid from retained
values. Only image-sized snapshots cross to the UI; uploads and two-frame texture
retirement retain their existing behavior.

Analysis generations change with file/range/FFT/reducer/style. Display revisions
change with dimensions; stale sizes are rejected without cancelling the ongoing
analysis. Returning to an earlier size still gets the newest display revision.
The completed analysis duration and colour scale survive redraws. Timing includes
sample reads, transforms and CPU image preparation, excluding file opening and
window drawing; it is not an isolated FFT timer.

## Validation method

Reference: i5-1235U, Linux, eight unrestricted workers, Hann 2048, hop 512,
`tests/signals/m39-repeat-1GB.wav` (500,000,000 mono PCM16 samples at 7200 Hz,
976,559 FFT frames). Original m39 and an I/Q fixture cover native visual checks.
An isolated GPU-backed Sway session uses separate configuration/session paths.
Window changes cover height, width, restoration and a larger-than-initial window.
Rapid changes during refinement and real splitter drags are checked separately.

Warm runs read the full input first and verify residency with mincore. Cold runs
use a separate fully written and fsynced copy, POSIX_FADV_DONTNEED on that copy
alone, and zero resident pages verified before launch. No global cache, scheduler,
CPU affinity or owner application settings are changed. Recordings are retained.

The harness observes the time from the resize command through the corresponding
CPU texture-preparation log, polled every 5 ms. This includes event handling,
rebinning and UI preparation, but is not physical screen-presentation latency.
The opt-in UI timer measures timer wake lateness, not physical input latency.
Process RSS includes mapped file pages and is distinct from retained cache size.

## Measurements

Three balanced repetitions per implementation/cache condition, alternating which
binary runs first. Baseline is the accepted aggregation implementation before #74.
The file and FFT settings are identical; the retained GUI grid is 4096×1025 and
the initial display grid is 1236×662. Binary hashes and all individual records are
in [74-data](74-data/). No build or other benchmark ran concurrently. Normal desktop
activity continued. Temperatures were sampled around operations, not controlled;
the first warm baseline was unusually fast and is retained. Frequency, power and
throttling counters were not recorded. These are laptop measurements, not universal
throughput or worst-case latency guarantees.

| Operation (window dimensions) | Before, median | After, median |
| --- | ---: | ---: |
| Height: 1280×800 → 1280×700 | 6.911 s | 42.2 ms |
| Width: 1280×700 → 1080×700 | 6.771 s | 21.8 ms |
| Return to 1280×800 | 7.183 s | 42.2 ms |
| Enlarge to 1680×950 | 7.670 s | 42.8 ms |
| Restore 1280×800 | 7.180 s | 37.7 ms |

The old binary completes six analyses per case: initial load plus five resizes.
The new binary completes one. Process CPU use per resize falls from 26–32 seconds
to 0.03–0.07 seconds. A separate run with 16 size changes after the first preview
also completes one exhaustive pass. Four native real/IQ × Peak/Mean-power splitter
checks each produce two cached views and one analysis; GPU access is verified at
`/dev/dri/renderD128`. Screenshots: [real splitter](74-data/real-splitter.png),
[IQ splitter](74-data/iq-splitter.png), [1 GB Peak](74-data/1gb-max.png),
[1 GB Mean power](74-data/1gb-mean-power.png).

| Initial-load measure, median | Before | After |
| --- | ---: | ---: |
| Warm full analysis | 6.810 s | 7.804 s |
| Cold full analysis | 9.467 s | 9.911 s |
| Warm first image paint call | 72.7 ms | 88.2 ms |
| Cold first image paint call | 121.4 ms | 121.0 ms |
| Warm process CPU through completion | 27.54 s | 31.48 s |
| Warm UI timer p99 lateness | 8.472 ms | 8.456 ms |
| Cold UI timer p99 lateness | 8.102 ms | 8.158 ms |
| Warm resident memory after completion | 143.6 MiB | 166.1 MiB |
| Warm peak resident memory | 268.1 MiB | 264.2 MiB |
| Cold peak resident memory | 160.5 MiB | 175.2 MiB |

This improves resize latency, with a measured first-pass cost: about 14.6% warm and
4.7% cold in these medians. The overview retains finer frequency/sample detail and
additional rendering state; these measurements do not isolate their individual
costs. Warm baseline times span 4.845–7.196 s, new times 7.512–7.921 s. Cold baseline
times span 9.041–12.069 s, new times 9.756–9.975 s. The reference spectral cache itself
is 16.02 MiB for Peak or 32.03 MiB for Mean power, distinct from process RSS.
The bounded preview budget avoids scaling sparse input touches with cache width.

## Numerical comparison

Direct progressive analysis and the overview use the same input, Hann 2048,
hop 512, output 1236×662 and default colour range. These are comparisons of dB
values before colour quantization, not screenshot similarity scores.

| Input and mode | Absolute dB error median | p95 | p99 | Maximum |
| --- | ---: | ---: | ---: | ---: |
| Original m39, Peak | 0 | 0 | 0 | 0 |
| Original m39, Mean power | 0 | 0 | 0.0000038 | 0.0000076 |
| I/Q HFDL, Peak | 0 | 0 | 0 | 0 |
| I/Q HFDL, Mean power | 0 | 0 | 0.0000038 | 0.0000076 |
| Repeated 1 GB m39, Peak | 0 | 0.621 | 2.649 | **17.531** |
| Repeated 1 GB m39, Mean power | 0.177 | 0.732 | 1.314 | **7.293** |

The large-file differences are the declared time-cell approximation, not rounding
noise. One cached time cell spans about 16.95 seconds of this 19.29-hour recording;
one initial display column spans about 56.18 seconds. Peak never falls below the
direct pixel maximum in this comparison, but a strong feature across a boundary
can raise its neighbor substantially. Mean power differences have both signs
(minimum signed difference −3.762 dB). The finite cache is unsuitable for claiming
exact arbitrary time-bin boundaries or recovering sub-cell structure through zoom.
No small amplitude-error bound is claimed for arbitrary signals.

Frame counts are exactly 976,559 for both large-file paths. Full PSD values and
sample peaks match exactly in all six fixture comparisons. Every rendered waveform
minimum is no higher and every maximum no lower than its direct counterpart, so
short extrema are preserved conservatively. Original m39 and HFDL spectra fit
all their 826 and 180 FFT frames in the cache respectively. Automated tests also
exercise grouped frequency bins for a larger FFT.

Standalone renders at 5000×1200, wider than the time cache, complete in 99 ms for
Peak and 113 ms for Mean power on the 1 GB cached data. These are single CPU-render
observations, not native high-DPI/GPU latency measurements. Their output deliberately
contains no new sub-cell detail. Raw fixture comparisons are in `74-data/accuracy-*`.

## Limits of validation

Windows/macOS runtime behavior, other GPU backends, multi-monitor/high-DPI changes,
physical input-to-photon latency, thermal steady state and extreme FFT sizes were
not measured. Native display checks extend to 1680×950; larger-than-cache output is
covered by toolkit-independent rendering. Zoom/pan, exact tile-level reconstruction,
live style reuse and faster first-file analysis remain separate work. Large source
recordings and the separate cold-copy fixture are retained.

## Checks and review

Automated checks cover exact cache-fitting real/IQ results, unequal time and
frequency groups, conservative waveform peaks, large-FFT frequency grouping,
complete frame/PSD accounting with compressed time cells, cancellation, display
revision filtering and return-to-previous-size handling. Incremental cached images
are compared with fresh full renderings, including empty followers and scale changes.

Review corrections: add separate display revisions; describe CPU image-preparation
cost accurately in the timing hint; bound sparse preview work independently of the
cache and remove stale splitter documentation. Native measurements are complete. The follow-up review is pending; its final result
will be recorded before owner acceptance.
