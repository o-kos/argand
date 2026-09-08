# Reference-host experiment archive

These are the exact temporary harness sources used for the recorded experiment,
not installed application code or a portable benchmark suite. They intentionally
record the reference host's CPU masks, file paths and `/tmp/argand-67` layout.
Do not run them unchanged on another host. The maintained portable current-path
benchmark is `crates/app/examples/fft_scheduling.rs`.

To reconstruct the historical DSP comparison, export baseline revision b8482cd to
an isolated scratch directory, retain its core/io/dsp crates and locked workspace
dependencies, and add a standalone `fft-scheduling-bench` package using `main.rs`.
Its dependencies are argand-core, argand-dsp, argand-io, rayon, anyhow, serde_json
from the workspace and libc 0.2. Apply `baseline-instrumentation.patch` with `git apply --unidiff-zero`, build a
release binary, save it as `baseline-bench`; apply `compact-instrumentation.patch`
to a fresh baseline export and save that binary as `compact-only-bench`.
Never apply these instrumentation patches to the implementation branch.
`timed-kernel-instrumentation.patch` adds intrusive per-frame timers for diagnostic
phase attribution only; it is not used for comparative throughput numbers.

The runners inject BENCH_BATCH/BENCH_GRAIN and apply masks inside Rayon start
handlers. `run-matrix.py` records residency, process costs, temperature and the
external wake probe. `run-cache.py` creates a separate cold copy; no global cache
flush is used. The ranges-only and ranges-plus-compaction binaries were exploratory
rejected/intermediate variants; their raw results remain in `kernels.jsonl`.

`native.py` requires an isolated GPU-backed Sway session and its IPC helper from
the reference experiment. It compares the baseline with identical UI timer and
CPU texture-preparation probes against the current application. `compute-affinity.c`
is a measurement-only pthread-name interceptor used to constrain exact E4/E6/mixed
masks before compute work starts; the UI process mask is never changed. The E8
native run instead exercises production CPUID discovery. Each run saves observed
main/worker masks and logs. The native helper is an evidence archive, not a
cross-platform UI test.
