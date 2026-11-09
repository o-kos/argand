# Architecture overview

Workspace ownership, persistent configuration and application identity. Start with [the repository rules](../../AGENTS.md); development policy lives in [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Purpose and audience

Argand is designed for viewing, navigating, editing, and performing spectral analysis on recorded signals, including SDR/RF captures. Its target user works with custom or unusual formats and needs a fast, lightweight native application rather than a heavy framework.

## Technology stack

- **Language:** stable Rust organized as a Cargo workspace.
- **GUI:** GPUI Kit 0.9.1 by Longbridge, the application facade over the GPUI 0.3.x family and gpui-component 0.9.1. All three are Apache-2.0 licensed.
- **Rendering:** wgpu, the GPUI 0.3.x graphics backend. Spectrograms are rendered as RGBA textures through image elements; waveforms use paths and quads. GPUI does not expose a public hook for a custom shader or wgpu pass within a frame; see [Rendering boundary](analysis-rendering.md#rendering-boundary).
- **DSP:** rustfft and realfft.
- **Configuration:** serde and TOML.
- **Logging:** tracing.

## Core requirements and invariants

1. Support Linux, Windows, and macOS.
2. Ship one binary plus configuration with minimal external dependencies. Take the GUI toolkit as one crates.io dependency, `gpui-kit`, and let `Cargo.lock` fix the graph.
3. Keep cold startup and the first waveform or spectrogram frame fast.
4. Provide the core display and editing operations available in ocenaudio.
5. Provide a separate detailed spectrum window.
6. A plugin system for formats and processing, with or without visualization, is planned around a separate worker process. It is currently deferred; see the [roadmap](../plans/IMPLEMENTATION_PLAN.md).

## Cargo workspace architecture

- **argand-core:** domain types such as `Signal`, real or complex sample metadata, `Selection`, and units. It also owns toolkit-independent render view models such as `WaveformEnvelope`, `DbGrid`, `SpectrogramImage`, and primitive lists, and the axis tick layout in `argand-core::axis`, which measures candidate labels through the `LabelMeasure` trait so that every front end places its marks by one policy. A view model's shape fields are the caller's to set, so its accessors check every coordinate against the declared shape and settle the product with `checked_mul`; one index past the end of a row is a valid offset into the next one, so an unchecked read answers with a neighbour's data that looks like a picture. The accessors differ in how strict they are, and the difference is deliberate. `SpectrogramImage::get` and `put` settle the whole buffer through `shape()` and answer nothing until it matches. `DbGrid::column`, `DbGrid::value` and `WaveformEnvelope::column` do not consult `shape()`; they check the one coordinate, settle that offset with `checked_mul` and take the cell from the slice, so a cell inside the declared shape stays readable where the buffer runs longer than that shape. It also owns `Adjustment`, the spectral inversion and gain a capture applies to its samples as it is read (see [Inverting and normalizing (#188, #190)](processing.md#inverting-and-normalizing-188-190)). It must not depend on GUI or heavy DSP code.
- **argand-dsp:** STFT and spectrogram generation, Welch PSD, window functions, min/max pyramid construction, and band extraction (`extract`: frequency shift, low-pass filtering integer decimation and fractional-rate reconstruction, see [Band and rectangle selection (#202)](processing.md#band-and-rectangle-selection-202)) and band suppression (`suppress`, see [Band suppression (#212)](processing.md#band-suppression-212)). Shading is a separate public step over a `DbGrid`, so changing the colour scheme or the dynamic range recolours values a caller already holds instead of running the transform again. It depends on rustfft and must not depend on GUI code.
- **argand-io:** WAV and other format readers behind the `FormatReader` interface: probe, open, and read through a lazy sample source, and `write::save`, which writes a capture or a span of it to a new file (see [Saving (#186)](editing-io.md#saving-186)). A plain RIFF WAVE may split its samples over several `data` chunks, as KiwiSDR recorders do with a `kiwi` GNSS chunk before each (#204): `riff::WavLayout::runs` walks the chunks after the first `data` up to the stated RIFF end and returns each body as a `DataRun`, `MmapSource` reads the runs as one array and the writer copies them into one `data` chunk. `write::FloatWave` writes computed real or I/Q `f32` samples, and a `SourceFile` with an `Encoding` is such a file that a save encodes into the capture's own format (#212). RF64/BW64 and a file with nothing after its first `data` chunk are one run, without a walk. This is the future connection point for custom formats through the worker.
- **argand-edit:** the editing engine: an immutable piece table (`Capture`) over the files a capture reads, `Clip`s of it, a `History` of versions for undo and redo, and `EditedSource`, a `SampleSource` reading a capture through one opened source per `SourceId` (see [Editing (#195, #196)](editing-io.md#editing-195-196)). It depends on `argand-core` only.
- **argand-app:** the application binary, `argand`, using the GPUI Kit facade over GPUI and gpui-component. It opens a signal file, analyses it on a thread of its own, and draws the spectrogram with axes. Its modules divide by whether they name a toolkit: `config.rs`, `session.rs`, `document.rs` and `analysis.rs` do not and are tested without a window, while `shell.rs`, `chrome.rs`, `axes.rs` and `spectrogram.rs` do. It owns cursors, selections and scrolling; the detailed spectrum window remains planned. The application binary is shipped alongside `aspec`.
- **argand-worker:** a planned processing worker binary that loads C ABI libraries and communicates through a stdio protocol. It is deferred.
- **argand-abi:** planned C ABI contract and protocol types. It is deferred.

## Configuration and session

- Read `argand.toml` at startup from beside the binary or from the platform-specific configuration directory.

- `crates/app/assets/argand.toml` is the single complete, commented default configuration for distribution. Keep it synchronized with `Config::default()` and all supported keys; its test parses without repairs. Package delivery is tracked in #86. Installations must preserve existing user configuration and must not place a default beside a system-installed executable, which would shadow the user file under the portable-first search policy.

`session.toml` is shared by every running instance (#43). `session::Writer` writes it as a read-modify-write under `session.toml.lock`, taken with `File::try_lock` and never waited for except for a bounded retry at the final flush; a filesystem without locks still gets the merge. The writer runs on its own thread (`session::Saver`, #175): the shell offers the session through a one-slot mailbox without waiting, a held-back offer is written when its interval ends, and dropping the `Saver` waits for the final flush at most one second. `session::merge` takes a setting from this instance only where it changed it since its last read or write, keeps the window rectangle and state together, and puts the files this instance opened at the head of the file's recent list. A newer-version file found at write time stops this run's writes. A running instance does not show files another opened until it restarts.

## Application identity and delivery

The B0 application icon uses closed cyan/coral I/Q ribbons. The selected full-size raster and 16/24-pixel optical SVG sources live in `crates/app/assets/icons/source/`. `crates/app/assets/icons/generate.py` reproduces PNG, ICO, ICNS, scalable SVG, monochrome SVG and tray PNG assets using Pillow and rsvg-convert. The toolbar embeds the 24-pixel PNG for its 22-pixel slot. The scalable SVG embeds the approved raster; it is intentionally not a vector tracing. Monochrome/tray artwork derives from the same optical SVG paths with transparent crossing gaps. The desktop installer includes the 24-pixel theme directory. Windows resource/macOS bundle packaging is tracked separately in #40.

The name and icon assets live in `crates/app/assets/icons/`, beside the desktop entry that names them. The set includes `argand.svg`, `argand.ico`, `argand.icns`, PNG sizes, and a monochrome glyph. A desktop environment learns the application's name and icon from `crates/app/assets/io.github.o_kos.argand.desktop`, matched against the window's application id; `install-desktop-entry.sh` beside it installs both into the user's own directories for a development build.

The window's size and state are restored wherever the toolkit reports the window state reliably; X11 and macOS both report a window as ordinary while it is not, which can cost the size a maximized window returns to (Issue #38). Its position is restored only where the toolkit reports enough to restore it: not on Wayland, where plain xdg-shell gives a client no absolute position, and not across displays on macOS, where gpui reports every display's origin as zero and opens on the primary one (Issue #37; its macOS half needs nothing gpui does not already provide).

The current implementation focus is display, editing, and spectrum analysis: phases 0 through 6 in [roadmap](../plans/IMPLEMENTATION_PLAN.md).

The plugin and worker design is deferred to the final section of [roadmap](../plans/IMPLEMENTATION_PLAN.md).
