# Argand implementation roadmap

This roadmap focuses on the core functionality: **display, editing, and spectrum rendering** covered by requirements 1 through 5. The plugin and worker design from requirement 6 is deferred and recorded at the end.

See [`AGENTS.md`](../../AGENTS.md) for the technology stack and architecture. Each phase ends with a verifiable "Done when" criterion.

> This is the long-term project roadmap, not an active Issue plan. Implementing any roadmap item requires a GitHub Issue and a separate active plan as defined in [`CONTRIBUTING.md`](../../CONTRIBUTING.md).

## Current CLI implementation

Before the GUI, the project gained the `aspec` CLI in `crates/cli`. It renders a signal file's spectrogram and averaged spectrum to PNG. It is not a disposable prototype: the domain model, readers, and transforms live in `argand-core`, `argand-io`, and `argand-dsp`, and the GUI will reuse them unchanged. See [`README.md`](../../README.md) for details.

The following work is complete:

- **Phase 1:** `Signal`, `SignalMeta`, `SampleType`, the `SampleSource` trait, and the `FormatReader` layer in `argand-io`. Supported inputs include WAV with uint8, int16, int32, float32, and unnormalized CoolEdit 16x8 float samples; FLAC; and raw files. Containers are detected from content rather than extensions. Files use memory mapping and lazy reads, and consumed pages are released to the kernel. A 30-minute, 172 MB capture renders in about 0.8 seconds while RSS remains around 60 MB and does not grow with the selected duration.
- **Phase 3:** configurable STFT size, window, and overlap; frames folded into image columns as they are computed so memory follows output size rather than input duration; six color schemes; and callback-based progress reporting.
- **Phase 4:** a two-sided spectrum from `-Fs/2` to `+Fs/2` with `fftshift` for complex signals, a one-sided spectrum for real signals, physical frequency axes through `center_frequency`, and consistent 0 dBFS readings for full-scale tones in both domains.

The GUI still needs the waveform and min/max pyramid from Phase 2, editing from Phase 5, the detailed spectrum window from Phase 6, and the application shell and configuration from Phase 0. The `argand-edit` and `argand-app` crates do not exist yet, and GPUI is not part of the dependency tree.

Dependencies are pinned in `Cargo.lock`. The repository does not track `vendor/`. For local offline builds, fetch dependencies in advance with `cargo fetch --locked` or generate `vendor/` with `./vendor-update.sh sync` from the repository root.

## Phase overview

| # | Phase | Outcome |
|---|---|---|
| 0 | Scaffold and application shell | An empty window runs on all three operating systems and reads configuration |
| 1 | Data model and file loading | ✅ Complete in `argand-core` and `argand-io` |
| 2 | Waveform | Fast time-domain navigation and zooming |
| 3 | Spectrogram | ✅ Complete in `argand-dsp`; GUI integration remains |
| 4 | I/Q correctness | ✅ Complete and covered by tests |
| 5 | Selection and editing | Cut, copy, paste, and undo comparable to ocenaudio |
| 6 | Detailed spectrum window | Deep analysis of a selection in a separate window |
| — | Deferred | Plugins and worker design |

## Phase 0 — Scaffold and application shell

Goal: launch an empty GPUI window on Linux, Windows, and macOS and read configuration.

- Add the Cargo workspace crates described in `AGENTS.md`, initially as empty scaffolds.
- Add the `argand` application with a GPUI and gpui-component window, a base theme, and dock placeholders for waveform, spectrogram, and panels.
- Persist window size and position between sessions.
- Pin GPUI and gpui-component to fixed Git revisions.
- Load `argand.toml` through serde and TOML and initialize tracing logs.
- Add CI builds for Linux, Windows, and macOS.

**Done when:** `cargo run -p argand` opens a window on all three operating systems, with native testing on the current host; cold startup is under one second; and configuration is applied.

## Phase 1 — Data model and file loading

Goal: open a file and obtain an in-memory representation of its signal.

- Add `Signal` with real or complex samples, `sample_rate`, `center_freq`, metadata, `Selection`, and units to `argand-core`.
- Add the `FormatReader` trait to `argand-io` with probe, open, and read operations over a lazy sample source. Include a built-in WAV reader, temporarily treating stereo as I/Q for tests, plus a stub reader for custom formats.
- Memory-map input files and parse them lazily without loading entire captures into RAM.
- Open files through the UI and display sample rate, duration, and sample type metadata.

**Done when:** WAV and a test I/Q dump open successfully; the status area shows sample rate, duration, and type; and memory does not grow linearly with file size.

## Phase 2 — Waveform

Goal: provide fast time-domain viewing, zooming, and panning.

- Build a min/max peak pyramid in background work in `argand-dsp`.
- Expose `WaveformEnvelope` as a toolkit-independent render model from `argand-core`.
- Render the envelope with GPUI paths or quads, choose level of detail by zoom, and provide zoom, pan, scrolling, and a time axis.
- Show I and Q in one track for complex signals and prepare the view for spectrogram mode.

**Done when:** multi-hour files scroll and zoom smoothly, and the first frame appears almost immediately by rendering pyramid levels as they become available.

## Phase 3 — Spectrogram

Goal: provide a fast, progressive time-frequency view.

- Generate STFT data in `argand-dsp` with configurable FFT size, window, and overlap, then map magnitudes through dB and a color map into RGBA tiles.
- Produce a coarse STFT immediately, refine it in background work, and cache tiles.
- Display tiles as GPUI images, zoom and pan with transforms, and share a linked time axis with the waveform.
- Switch or combine waveform and spectrogram views and expose live FFT settings for size, window, overlap, and color scheme.

**Done when:** the spectrogram renders progressively without blocking the UI, its time axis stays synchronized with the waveform, and FFT parameter changes remain responsive.

## Phase 4 — I/Q correctness

Goal: handle complex signals correctly.

- Produce a two-sided spectrum from `-Fs/2` to `+Fs/2` with `fftshift`.
- Express the frequency axis in physical hertz through `center_frequency` and `sample_rate`.
- Open a real interleaved-f32 I/Q capture and verify its spectrum and bandwidth.

**Done when:** a test tone with a known frequency peaks at the correct location in the two-sided spectrum, the frequency scale is correct in hertz, and a real signal produces a one-sided spectrum.

## Phase 5 — Selection and editing

Goal: provide basic editing comparable to ocenaudio.

- Add a piece table in `argand-edit` over the memory-mapped original and inserted buffers.
- Add an undo and redo command stack and a segment clipboard.
- Support selection, including multiple selections, cut, copy, paste, delete, and trim or crop. Keep I and Q together for complex signals.
- Invalidate waveform pyramid levels and spectrogram tiles only for affected regions.

**Done when:** cuts and pastes in multi-hour files are immediate, undo and redo remain stable, and waveform and spectrogram views update correctly after edits.

## Phase 6 — Detailed spectrum window

Goal: provide deep analysis of a selected region.

- Open the current selection in a separate GPUI window.
- Provide a high-resolution FFT, averaged Welch PSD, and a zoomable detailed spectrogram.
- Add an independent toolbar for FFT size, window, averaging, dynamic range, markers, and peak search.

**Done when:** a selection opens in a detailed spectrum window with independent settings, markers and peak search work, and the window updates when the selection changes.

## Cross-cutting work

- **Rendering boundary:** prevent GPUI types from leaking into core crates; see `AGENTS.md`.
- **Concurrency:** keep DSP and cache construction outside the UI thread.
- **Configuration:** persist view and FFT settings between sessions.
- **Tests:** add unit tests for STFT, PSD, and frequency-scale correctness in `argand-dsp`; piece-table and undo or redo behavior in `argand-edit`; and a golden test for peak placement.

## Risks and mitigations

- GPUI is a moving target whose backend is migrating from Blade to wgpu. Pin revisions and preserve the rendering boundary so egui with wgpu remains a viable fallback.
- GPUI limits custom GPU canvases to its exposed primitives. Render spectrograms as textures and waveforms as paths or quads. If dense rendering becomes a bottleneck, rasterize the waveform into a texture or replace the toolkit.
- A real GPU is required. Account for degraded behavior in virtual machines and headless environments.

## Deferred — plugins and worker

The transport design uses a separate **worker process** with `run`, `serve`, and `describe` modes. The worker loads C ABI libraries with `dlopen`, isolating unsafe FFI and crashes from the GUI.

The control plane uses framed messages over stdio for commands, events, and small results. For bulk data, the worker reads the input file itself and writes large output to a file or shared memory while sending only references over the control channel. A shared-memory ring buffer is reserved for continuous real-time streams rather than batch processing of a selection.

The host builds settings UI from a JSON parameter schema supplied by the plugin; GUI code never crosses the process boundary. The contract has three levels: formats, non-visual processing such as resampling, and visual processing where demodulators or decoders return typed results for a separate window.

Open questions for implementation:

- batch processing of a selected region versus a continuous real-time stream;
- whether the worker reads input itself or receives samples from the GUI;
- whether a shared-memory ring buffer is necessary.
