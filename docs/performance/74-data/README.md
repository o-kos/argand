# Recorded #74 validation

`native.jsonl` contains every final native run. `summary.json` contains medians of
three runs per binary and file-cache state. `binaries.sha256` identifies the measured
release and saved aggregation baseline. The benchmark alternates baseline/current,
then current/baseline, then baseline/current; no samples are dropped.

The Python scripts are the exact host harnesses, retaining their recorded paths:
`native.py` uses an isolated Sway session through `common.py` under
`/tmp/ocenaudio-bench`, configurations/results under `/tmp/argand-74`, the saved
`baseline-argand`, and the workspace `target/release/argand`. Run `native.py matrix`
for three warmed comparisons and the Mean-power/in-flight-resize checks, and
`native.py cold` for three comparisons on a separately copied/fsynced
`/tmp/argand-74/cold-copy.wav`. The scripts require Sway, grim and the real GPU.
They verify warm/full or cold/zero page residency with mincore. Cold eviction uses
POSIX_FADV_DONTNEED only on that separate copy.

`visual.py` performs real pointer drags with the host's existing virtual-pointer
helper in `/tmp/argand-start-page`. Its four JSON records verify the render device,
one analysis pass, and cached redraws. Selected original screenshots are retained.
No screenshots were synthesized or retouched.

`accuracy-*.txt` records direct progressive analysis versus `analyze_overview` and
`Overview::render`, with eight Rayon workers, Hann 2048, hop 512, 1236×662 output,
waveform enabled and default dynamic range. It reports all FFT counts, dB error
percentiles, PSD/sample-peak equality and conservative waveform bounds. The script
also renders five additional sizes from the same overview without a sample source.
Those single-run DSP timings are diagnostic and are not the native before/after
performance comparison. Inputs are the repository's retained m39, repeated 1 GB
m39 and iq_i16-hfdl fixtures; large recordings are not committed here.
