# AGENTS.md — Argand

Argand is a cross-platform signal editor and analyzer focused on **I/Q (complex)** signals. Its UX is inspired by ocenaudio: waveform and spectrogram views with selection-based editing. Unlike an audio editor, Argand also handles complex samples and radio-frequency semantics.

> This file is shared project context for coding agents. Keep it synchronized with the code: update it whenever an architectural decision or invariant changes.

## Purpose and audience

Argand is designed for viewing, navigating, editing, and performing spectral analysis on recorded signals, including SDR/RF captures. Its target user works with custom or unusual formats and needs a fast, lightweight native application rather than a heavy framework.

## Technology stack

- **Language:** stable Rust organized as a Cargo workspace.
- **GUI:** GPUI by Zed with gpui-component by Longbridge. Both frameworks use the Apache-2.0 license.
- **Rendering:** GPUI, whose graphics backend is migrating from Blade to wgpu. Spectrograms are rendered as RGBA textures through image elements; waveforms use paths and quads. GPUI does not expose a public hook for a custom shader or wgpu pass within a frame; see "Rendering boundary."
- **DSP:** rustfft and realfft.
- **Configuration:** serde and TOML.
- **Logging:** tracing.

## Core requirements and invariants

1. Support Linux, Windows, and macOS.
2. Ship one binary plus configuration with minimal external dependencies. Pin GPUI and gpui-component to fixed Git revisions because their crates.io releases lag behind development.
3. Keep cold startup and the first waveform or spectrogram frame fast.
4. Provide the core display and editing operations available in ocenaudio.
5. Provide a separate detailed spectrum window.
6. A plugin system for formats and processing, with or without visualization, is planned around a separate worker process. It is currently deferred; see the end of `docs/plans/IMPLEMENTATION_PLAN.md`.

## Domain-specific rules

- A signal can be **real** or **complex** I/Q baseband.
- I/Q samples use interleaved `f32` values by convention: `I, Q, I, Q, ...`.
- A complex signal has a two-sided spectrum from `-Fs/2` to `+Fs/2` and requires `fftshift`. A real signal has a one-sided spectrum from `0` to `Fs/2`.
- Frequency axes use physical hertz derived from `center_frequency` and the true capture `sample_rate`. The sample rate may be in the megahertz range and must not be treated as an audio sample rate. Audio playback is not a project goal.
- The I/Q waveform view combines the I and Q channels; spectrogram mode is available separately.
- Editing operates on the underlying sample array. Cut, copy, and paste must keep I and Q together.

## Cargo workspace architecture

- **argand-core:** domain types such as `Signal`, real or complex sample metadata, `Selection`, and units. It also owns toolkit-independent render view models such as `WaveformEnvelope`, `SpectrogramTile`, and primitive lists. It must not depend on GUI or heavy DSP code.
- **argand-dsp:** STFT and spectrogram generation, Welch PSD, window functions, min/max pyramid construction, resampling helpers, and frequency shifting. It depends on rustfft and must not depend on GUI code.
- **argand-io:** WAV and other format readers behind the `FormatReader` interface: probe, open, and read through a lazy sample source. This is the future connection point for custom formats through the worker.
- **argand-edit:** a planned editing engine using a piece table over the memory-mapped original and inserted buffers, a command stack for undo and redo, and a clipboard.
- **argand-app:** the planned application binary using GPUI and gpui-component with skin support. It owns the main waveform and spectrogram window, cursors, selections, scrolling, status, transport, and the detailed spectrum window. It converts core view models into GPUI images, quads, and paths. This is the single shipped binary.
- **argand-worker:** a planned processing worker binary that loads C ABI libraries and communicates through a stdio protocol. It is deferred.
- **argand-abi:** planned C ABI contract and protocol types. It is deferred.

## Rendering boundary

GPUI types must never leak into `argand-core`, `argand-dsp`, `argand-io`, or `argand-edit`. Those crates return textures, envelopes, primitive lists, and other toolkit-neutral data. Only `argand-app` converts them into GPUI images, quads, and paths.

This boundary keeps the toolkit replaceable. If GPUI proves too restrictive for a custom GPU canvas, the fallback is egui/eframe with wgpu without rewriting the core crates.

## Performance strategy

- Memory-map input files and parse them lazily instead of loading complete captures into RAM.
- Build a multi-level min/max peak pyramid for waveform level-of-detail selection.
- Compute spectrogram STFT tiles progressively, showing a coarse result first and refining it in background threads. Upload tiles as GPU textures and use transforms for zooming and panning.
- Use a piece table so cuts and pastes remain `O(1)` for multi-hour files.
- Keep expensive work off the UI thread by using rayon or dedicated worker threads.

## Build and run

- After the GUI scaffold exists, run it with `cargo run -p argand --locked`; build releases with `cargo build --release --locked`.
- Pin GPUI and gpui-component to fixed Git revisions in `Cargo.toml`.
- Commit the complete dependency graph in `Cargo.lock`; do not commit `vendor/`. Run `cargo fetch --locked` before an offline build, then use `cargo build --frozen`.
- Run rustfmt and Clippy for every change: `cargo fmt --all -- --check` and `cargo clippy --all-targets --locked -- -D warnings`.
- A real GPU is required. Software rendering and some virtual machines may degrade performance.

## Engineering conventions

- Use `thiserror` in libraries and `anyhow` in binaries. Non-test code must not use `unwrap` or `expect` on external data. Panics must not unwind across FFI boundaries.
- Read `argand.toml` at startup from beside the binary or from the platform-specific configuration directory.
- Express units and axes in physical hertz and seconds. Never confuse audio sample rates with the true RF capture sample rate.
- Direct conversation with the project owner must be in Russian only.
- All repository content and project communication outside that direct conversation must be in English. This includes source comments, user-facing messages, logs, documentation, branch names, commit messages, Pull Request titles and descriptions, issues, and release notes.
- Write code comments only when the intent is not evident from the code itself, and keep them concise.
- Never state or imply in commit messages, Pull Requests, issues, or release notes that an AI tool was used during development.

## Git workflow

- Follow the complete process in `CONTRIBUTING.md`.
- Every implementation must have a GitHub Issue, a focused branch, and a versioned plan in `docs/plans/`.
- Open a Draft Pull Request early and link it to the Issue with a closing keyword.
- Update plan checkboxes in the commits that complete the corresponding work. Do not record commit hashes in plans.
- Move a finished plan to `docs/plans/completed/` before final review.
- `main` is protected. Merge only through a Pull Request using squash merge after all checks pass and all review conversations are resolved.

## Current status

- The name and icon assets are currently in `icons/`; move them to the appropriate asset directory when the application scaffold is added. The set includes `argand.svg`, `argand.ico`, `argand.icns`, PNG sizes, and a monochrome glyph.
- The current implementation focus is display, editing, and spectrum analysis: phases 0 through 6 in `docs/plans/IMPLEMENTATION_PLAN.md`.
- The plugin and worker design is deferred to the final section of `docs/plans/IMPLEMENTATION_PLAN.md`.
