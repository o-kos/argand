# FFT scheduling investigation (#67)

## Workload and method

Measured on Linux, Intel Core i5-1235U: CPU 0/1 and 2/3 are the two P-core
SMT pairs, 4–11 the eight E cores. Product code does not embed these identifiers.
Baseline is the #29 implementation before this branch. The input is
`tests/signals/m39-repeat-1GB.wav`: 1,000,000,044 bytes, mono PCM16, 7,200 Hz,
500,000,000 samples. Its samples repeat `tests/signals/m39.wav` continuously.
All DSP comparisons use Hann 2048, hop 512, 976,559 FFT frames, a 1214×662
Max/Oceanic spectrogram, default dynamic range and waveform enabled.

The standalone harness runs the application's progressive DSP in a private Rayon
pool. Only its worker threads receive experimental masks. Each placement has three
balanced, shuffled repetitions after two initial warmups. Batching and kernel
experiments are separate rounds; compare variants within a round, not across tables.
No build or second benchmark ran concurrently. Ordinary desktop activity was not
stopped. The harness records wall time, child CPU time, faults, context switches,
RSS, package temperature sampled once per second and a separate process's 20 ms
sleep wake lateness. That last measurement is **not UI latency**.

Warm mapped runs explicitly read the entire file beforehand and verify residency
with `mincore`. Cold runs use a separate fully written, fsynced copy, request
`POSIX_FADV_DONTNEED` on that file only and verify zero residency. No global cache
or scheduler setting is changed. Memory-mode runs repeat the already decoded small
original in RAM, preserving sample values and length; they still copy samples,
build the envelope and perform all FFTs. Opening/normalization scanning is outside
the standalone analysis timer. Actual window measurements also include opening.

Raw measurements are in [67-data](67-data/). All final Max grid hashes are
`ec469eda10ea059b`; every result contains 976,559 frames and no affinity error.
Temperatures varied substantially (roughly 59–100 °C); unusually fast runs also
occurred at high initial readings. All measured records are retained, without
post-hoc temperature filtering. Frequency, package power and throttling counters
were not captured, so thermal and competing-load causes cannot be distinguished.
These are interactive laptop measurements, not a controlled laboratory claim.

## Existing execution and costs

One coordinator OS thread creates a per-document pool capped at eight available
logical CPUs. It waits in `pool.install`, so serial reading/decode/peak scanning,
waveform accumulation, result absorption and snapshots execute inside that pool
between parallel batches. A document has one active request; generation changes
cancel it between bounded steps. Multiple application instances have separate pools.

The original 256-frame batches use Rayon work stealing with a minimum grain of
eight frames. File partitions are not statically assigned to cores. realfft/rustfft
create no internal worker pools; each request creates its FFT plans once. Scratch
is reused within a partial accumulator, but Rayon folds/reductions create fresh
partials every batch. Each FFT used to append a full image-height row vector,
even when consecutive FFTs contribute to the same display column.

A representative baseline run (8.706 s) divides into:

| Stage | Wall time |
| --- | ---: |
| Sparse preview | 0.040 s |
| Sequential read/decode and peak scan | 2.963 s |
| Waveform envelope | 1.207 s |
| Windowing, FFT, bin reduction, parallel merge and absorption | 4.215 s |
| Intermediate snapshots | 0.261 s |

These are stage-boundary timers, not sums of per-thread CPU times. FFT arithmetic
is not isolated from windowing/bin reduction within the combined stage; separating
those with an intrusive timer on every short FFT would change the hot loop. The
matrix and kernel experiments test the scheduling/aggregation hypothesis directly.
A hardware sampling attempt was rejected by the host kernel (`perf_event_paranoid=4`);
that setting was left unchanged.

A separate compact-kernel diagnostic then inserted per-frame timers, with one
warmup and three measured repetitions at 1024/32 on eight unrestricted workers.
The final hash remained identical. Median **summed worker elapsed time** was
3.346 s preparation (peak, window, scratch and row-slot setup), 4.768 s inside the
FFT-library call, and 12.330 s magnitude/power/image-row aggregation: approximately
16%, 23% and 60% of the timed frame loop. These intervals overlap across threads,
include preemption and timer overhead, and are not additive end-to-end seconds or
hardware CPU-cycle counts. They are diagnostic attribution, not throughput results.
Parallel merging, I/O, envelope and snapshots lie outside these per-frame totals.
Thus the work called “FFT rendering” spends considerably more time outside the
FFT-library call; adding FFT workers alone cannot eliminate those costs.


Snapshots are rate-limited to 20 Hz. Dirty-column caching avoids repeated shading,
but each published result still owns full image/grid arrays. UI delivery is
asynchronous, uploads coalesce, and CPU texture preparation copies/swizzles the
whole image before GPUI submission. No complete file read or FFT runs on the UI
thread. GPU presentation and physical input-to-display latency require separate
measurement; a quick first frame alone does not prove responsiveness.

## Worker count and placement: baseline algorithm

Medians of three repetitions. Mixed masks use **one logical CPU per P core**.
Wake p99 is the median of each run's separate-process p99, not a pooled percentile.

| Pool / allowed CPUs | Time, s | Range, s | CPU, s | Wake p99, ms |
| --- | ---: | ---: | ---: | ---: |
| 4 / unrestricted | 10.820 | 10.19–12.12 | 26.95 | 0.174 |
| 6 / unrestricted | 10.771 | 10.08–10.87 | 31.95 | 0.174 |
| 8 / unrestricted | 9.419 | 9.18–10.44 | 32.44 | 0.302 |
| 10 / unrestricted | 10.649 | 9.39–10.82 | 39.54 | 0.701 |
| 12 / unrestricted | 10.507 | 9.61–11.19 | 42.07 | 0.742 |
| 4 / E CPUs 4–7 | 12.026 | 11.01–12.63 | 29.78 | 0.175 |
| 6 / E CPUs 4–9 | 10.541 | 10.13–10.59 | 29.88 | 0.168 |
| 8 / E CPUs 4–11 | 9.509 | 9.40–10.85 | 32.59 | 0.122 |
| 6 / CPUs 0,2,4–7 | 9.942 | 9.91–11.31 | 28.35 | 0.174 |
| 10 / CPUs 0,2,4–11 | 9.105 | 9.08–10.84 | 33.69 | 0.357 |

There is no evidence of nested FFT oversubscription. More unrestricted workers
consume more CPU without a reliable throughput advantage. E8 performs similarly
to unrestricted 8 and leaves P cores available, making it a useful opt-in policy.
Mixed 10 is only slightly faster, within the observed spread; it does not justify
pinning the default. E4 sacrifices throughput; E6 is an intermediate budget.

## Batch and aggregation experiments

Baseline, eight unrestricted workers, three repetitions per pair:

| Batch / minimum grain | Median, s |
| --- | ---: |
| 256 / 8 | 9.346 |
| 512 / 8 | 9.816 |
| 1024 / 8 | 9.923 |
| 1024 / 32 | 9.138 |
| 1024 / 128 | 9.095 |
| 1024 / 256 | 10.465 |
| 1024 / 1024 | 15.697 |
| 2048 / 8 | 9.200 |

One indivisible task per batch loses parallelism. Merely increasing the batch
while retaining tiny tasks is not enough. Larger grains reduce allocation,
work-stealing and merge overhead, but must leave enough tasks for heterogeneous
workers. Precomputing bin-to-row ranges alone failed to improve the separate
kernel experiment (9.083 s vs 8.677 s baseline) and was rejected.

Max aggregation can combine consecutive frames for one output column inside each
partial before the parallel merge. FFT coverage, power accumulation and time peak
remain exhaustive. A separate frame counter preserves PSD normalization. Mean
reduction continues to retain individual logarithmic values and their counts.

Final compact-only comparison, four balanced repetitions, eight unrestricted
workers (all runs retained, including two unusually fast late runs):

| Variant | Median, s | Individual times, s | Median CPU, s |
| --- | ---: | --- | ---: |
| Baseline 256/8 | 8.836 | 9.060, 8.967, 8.706, 8.199 | 29.91 |
| Compact 256/8 | 8.744 | 8.594, 8.952, 8.894, 8.522 | 32.07 |
| Compact 1024/32 | 7.461 | 6.952, 7.969, 8.045, 5.072 | 26.37 |
| Compact 2048/64 | 7.858 | 8.629, 7.840, 7.875, 5.414 | 27.68 |

The overall median improves 15.6%, but the two middle, thermally comparable rounds
suggest a more conservative gain of about 10%. Do not promise the fastest 5 s run.
1024 is selected over 2048 for bounded cancellation latency and similar throughput.
The production minimum grain is `max(1, batch_frames / (workers * 4))`; Rayon
retains dynamic work stealing rather than fixed ownership of portions of the file.
Decoded batch memory is capped at 4 MiB, except when one FFT itself exceeds that.
This bound can reduce the actual frame count for large FFTs or complex captures.
A second bound limits estimated `N log2(N)` work to 1024 reference 2048-point
transforms, or one larger FFT. This also bounds high-overlap workloads whose
unique input buffer is small. It is a work bound, not a wall-clock deadline.

## File cache

Baseline, eight unrestricted workers; medians of three repetitions:

| Source | Time, s | Verification |
| --- | ---: | --- |
| Warm mapped PCM16 | 8.606 | 100% resident before each run |
| Cold separate mapped copy | 10.692 | 0% resident; 7,584 major faults/run |
| Repeated decoded samples in RAM | 8.593 | No major faults |

Storage can add about two seconds here, but removing mapping and integer decoding
does not remove the dominant processing costs. The memory source has a small
working set and is not a model of a fully materialized multi-gigabyte float array.

## Final native window comparison

The final release (including both review fixes) was measured in an isolated
GPU-backed Sway/Wayland session at 1280×800. The native label measurement yields
1236×662 plot pixels, identical across all native cases but different from the
standalone table. FFT, signal, reducer, colors and waveform settings remain fixed.
Two explicit warmups precede three balanced shuffled repetitions. Every input is
warmed before opening. This series has more variable background conditions than
the preliminary native series, which is archived separately and not pooled here.

The baseline has the same opt-in 20 ms UI timer and texture-preparation probes.
No compute-affinity restriction is applied to the process as a whole. Exact E4,
E6 and mixed masks use the measurement-only thread-name interceptor; production
`efficiency` discovers E8 itself. All observed main-thread masks remain `0–11`;
all FFT worker masks match their intended policy. The GUI build is separate from
the owner's current binary. The ordinary UI trace overhead is present in both
compared versions and is disabled in normal use.

| Variant | Analysis median, s | Range, s | Process CPU, s | UI lateness p99, ms | Worst UI lateness, ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline, 8 unrestricted | 8.798 | 8.250–9.472 | 34.29 | 9.476 | 33.290 |
| New, 8 unrestricted | 7.047 | 6.780–7.685 | 28.18 | 9.342 | 18.880 |
| New, 4 unrestricted | 8.419 | 7.831–12.752 | 23.49 | 8.820 | 27.243 |
| New, 6 unrestricted | 7.466 | 7.459–7.526 | 24.58 | 8.784 | 9.920 |
| New, 4 E cores (4–7) | 8.369 | 8.316–9.096 | 24.51 | 8.440 | 9.261 |
| New, 6 E cores (4–9) | 8.037 | 7.544–8.181 | 26.75 | 8.445 | 11.606 |
| New, 8 E cores (discovered) | 6.902 | 6.826–7.234 | 27.68 | 6.644 | 19.250 |
| New, mixed 10 (0,2,4–11) | 7.425 | 6.429–8.926 | 30.85 | 9.895 | 25.871 |

Native analysis improves **19.9%** at the default worker budget, with **17.8% less
process CPU time**. E8 is 21.6% faster than baseline and has a lower median p99
UI lateness. Mixed 10 does not beat unrestricted 8 in this series. Six unrestricted
workers are a useful lower-CPU compromise, about 6% slower than the new default.
The three-run sample and changing laptop conditions do not establish a universal
optimum. In particular, a 12.75 s four-worker run is retained rather than discarded.

UI values are timer dispatch lateness above the requested 20 ms interval, measured
on the main loop from window creation until the completion event. They are not
input-to-photon latency or a frame-time measurement. Each reported p99 is the
median of three per-run p99 values; the last column is the worst sample in any run.
After completion, the corresponding idle p99 values are about 0.44–0.48 ms.
CPU texture-preparation p99 medians are 1.789 ms baseline, 2.019 ms new default and
1.417 ms E8. Default result-delivery age p99 stays around 0.18 ms. Actual GPU upload,
compositor CPU cost, presentation fences and physical input latency are not isolated.

First-picture medians are 74.15 ms baseline, 71.13 ms new default and 86.52 ms E8;
they include the asynchronous opening path and a rendered frame. The analysis
column excludes opening and rendering. File-open calls themselves were below
0.2 ms in this warmed series. A separate-process wake probe during analysis has
p99 medians 0.779 ms baseline, 0.233 ms new default and 0.186 ms E8. This is evidence
about a small competing timer workload, not every other desktop application.

Final spectrogram and waveform image regions match the baseline **pixel for pixel**
for all seven new native policies in the first round. Archived full-window
[before](67-data/native-before.png) and [after](67-data/native-after.png) images also
show the independently measured completion times. Numerical DSP equivalence is
also checked independently of rendering. No grid, ruler, waveform-style or progress
indicator change is part of this issue.

## Chosen policy and automatic planning proposals

Current defaults are a conservative fixed budget: up to eight available logical
CPUs, 1024-frame batches subject to the memory bound, dynamic tasks and no affinity.
`[analysis] workers`, `batch_frames` and `affinity` allow explicit experiments.
Affinity `efficiency` discovers allowed E cores on Linux Intel hybrid x86-64 using
[CPUID core types](https://www.intel.com/content/www/us/en/developer/articles/guide/12th-gen-intel-core-processor-gamedev-guide.html).
A temporary probe thread tests allowed CPUs; only compute workers are subsequently
pinned. A class is an identifier, not a numeric performance ranking. Unsupported
platforms/CPUs and discovery errors fall back to ordinary scheduling with warnings.
A pinning failure is also logged; success is not silently assumed. With four
workers, this policy allows all detected E cores rather than arbitrarily choosing
four CPU IDs. No CPU frequency heuristic or host-specific numbering is embedded.

A future automatic planner should:

1. Intersect topology with the process/cgroup CPU budget; discover physical cores,
   SMT siblings and efficiency classes through platform APIs. Treat missing data
   as unknown, not as identical performance.
2. Offer explicit interactive and throughput goals. For interactive hybrid CPUs,
   test an E-only candidate and a mixed candidate with one thread per physical P
   core. For homogeneous CPUs, start below the available physical-core count.
   Add SMT siblings only when measurements demonstrate a benefit.
3. Calibrate with a small, bounded sample of the actual FFT size, real/IQ domain,
   reducer and image height. Reuse useful computed work. Compare throughput, CPU
   time and UI event-loop latency; avoid a benchmark sweep on every file opening.
4. Estimate task cost and choose enough queued tasks for work stealing, bounded
   by decoded-buffer memory and a cancellation/interactive latency budget. A
   frame count alone cannot predict latency for very large FFT sizes.
5. Cache a plan keyed by CPU/topology, process budget, workload and power mode.
   Invalidate on affinity/quota/power changes. Adapt slowly with hysteresis and a
   minimum improvement threshold; avoid reacting to a single hot or busy run.
6. Reduce the budget when UI latency persistently rises; increase it only after a
   stable interval. Never change global scheduler policy or override a user cap.

Only the explicit settings, memory bound and optional E-core discovery are
implemented here. Automatic calibration, physical-core/SMT budgeting across all
platforms, thermal feedback and runtime latency-driven adaptation remain proposals.

## Reproduction and limits

A portable window-free benchmark of the implemented path is included:

```sh
CARGO_TARGET_DIR=/tmp/argand-67/target cargo build -p argand --release --locked --example fft_scheduling
/tmp/argand-67/target/release/examples/fft_scheduling tests/signals/m39-repeat-1GB.wav 8 1024
```

It prints opening and analysis times separately, first publication, update count,
FFT count and final grid hash. It does not warm/evict caches or pin CPUs for you.
Apply experimental masks only to compute threads in a harness, never `taskset` to
the GUI process. Trace `argand::ui_latency=trace` for UI timer lateness, result age
and CPU texture preparation; diagnostics are disabled by default.

The reference workload is mono PCM16 with FFT 2048. Numerical tests also cover
real/IQ, Max/Mean, waveform, PSD and batch boundaries, but throughput at other FFT
sizes, complex/multi-gigabyte captures, battery mode and other CPU families has not
been established. Linux is locally tested; Windows/macOS fallback paths still
need their platform CI/runtime validation. E-only affinity is not evidence of
better behavior for every desktop workload or power policy.

## Validation and independent review

Formatting, strict Clippy and all 410 tests pass. Tests compare real/IQ Max/Mean
outputs, PSD normalization, waveform envelopes and FFT counts across scheduling
options; they also check configuration repair, actual buffer capacity and
cancellation during high-overlap refinement. The separate release build follows
the gate; the owner's existing `target/release/argand` and large recordings are
preserved. The first independent review found two substantive issues: a memory
bound alone did not bound FFT work at high overlap, and shrinking a preallocated
buffer did not shrink its capacity. Both were accepted and fixed as described
above. No findings were rejected. The second implementation review was clean.
The PR remains Draft for owner feedback; cross-platform full CI is not yet claimed.
