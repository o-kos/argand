# Issue #186: Save a time selection or the whole capture as a new file

Resolves #186.

Class A (data safety, a new public `argand-io` API and dependency, an expected diff well above 400 lines across more than five code files). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

Step 2 of phase 5 in `IMPLEMENTATION_PLAN.md`. `argand-io` learns to write, and the application gains File → Save as… for the whole capture and File → Save selection as… for a time selection. The source file is never modified, and the target is replaced only by a complete file.

## Context

- `argand_io::open` probes the head (64 KiB), then reads WAVE/RF64/BW64 layouts it understands through `MmapSource`, headerless files through `MmapSource` with `OpenHints::raw` and `byte_offset`, and FLAC plus WAVE layouts it declines (24-bit, for one) through `DecodedSource` (symphonia).
- `riff::parse` walks chunks up to `data` and ignores everything but `fmt `, `ds64` and `data`.
- `SignalMeta::center_freq` comes only from `--center` (`cli.rs`) and the recent list's hints (`session::Hints`). For both signal kinds it is the physical frequency of baseband 0 Hz: the centre of a complex band, the lower edge of a real one (#185 records this).
- `SampleSource::original_sample_units` gives the affine map from decoded values back to stored ones; `DecodedSource` reports it for FLAC.
- `Shell::selection` holds `Selection { time: Option<SampleSpan>, .. }`, cleared on file opening.
- GPUI 0.3.6 `App::prompt_for_new_path(directory, suggested_name)` returns `Result<Option<PathBuf>>`: the file chooser portal on Linux, `IFileSaveDialog` on Windows (no type filter beyond "All files") and `NSSavePanel` on macOS. The native dialogs confirm overwriting an existing file themselves.
- `minimap.rs` is the model for a separately cancellable worker with a bounded reply channel.
- `flacenc` 0.5.1 (Apache-2.0) encodes frame by frame with `encode_fixed_size_frame`; STREAMINFO (MD5, total samples) is known only at the end.

## Decisions

Agreed with the owner:

1. **Format follows the source.** WAVE stays WAVE, same sample type and bit depth, sample bytes copied unchanged. A headerless capture becomes WAVE with its own sample type (all ten raw types, including `f16x8`, have a WAVE form). FLAC stays FLAC, re-encoded losslessly with `flacenc`. A WAVE output whose RIFF size would exceed `u32` is written as RF64 with `ds64`.
2. **Reference frequency in the file.** WAVE carries an `auxi` chunk in the layout SDR#, HDSDR and SDRuno write (centre frequency as integral `u32` hertz, written only when it fits) and an Argand chunk `argd` with the exact `f64` frequency and sample rate. FLAC carries a Vorbis comment. Opening reads `argd`, then `auxi`, then the FLAC comment; an explicit `--center` overrides the file.
3. **Original units.** Stored sample values are written, never normalized or gain-adjusted ones.
4. **Two commands.** Save as… (Ctrl+Shift+S, Cmd+Shift+S) saves the whole capture; Save selection as… (Ctrl+Alt+S, Cmd+Option+S) saves the time selection and is disabled without one. The open file is never a valid target.
5. **Background, atomic, cancellable.** One save at a time on its own thread, bounded 4 MiB buffers, a temporary file beside the target, sync and rename on success. Status-bar progress with a cancel button; a failure or cancellation removes the temporary file and leaves the target as it was. Viewing and navigation stay available; opening another file does not stop the save.

Derived here:

- **Byte copy for every WAVE source.** `riff` gains a layout scan that reports `fmt ` bytes, block alignment and the data range even for layouts the native reader declines, so 24-bit WAVE is copied rather than decoded. The written `fmt ` is the source's own chunk with its rate fields set to the effective rate. Only a sample type hint on a layout the native reader handles synthesizes `fmt ` from the effective `SampleType`, because the decoder reads the stored layout whatever the hint says.
- **Copy reads the file, not the mapping.** The writer opens its own `File`, reads the header and copies `data_offset + start × block .. data_offset + end × block` through that one handle, so it neither shares the document's cursor nor grows the mapping's resident set. The data length (the smaller of the declared and the present one) must still give the capture's sample count, otherwise the source changed since it was opened and the save is refused.
- **FLAC path.** One strict decoder resolves its own length, which must match the opened capture, seeks by exact timestamp and fails on a damaged packet or a gap in packet timestamps. Values go back to integers exactly and are encoded in 4096-sample blocks at the source's bit depth, up to 24 bits. STREAMINFO is a fixed 34-byte block written first as a placeholder and rewritten at the end; a span of 2^36 samples or more is refused. The Vorbis comment also carries the exact sample rate. `flacenc` 0.5.1 states only rates up to 96 kHz that frame headers can carry, so the header takes such a rate within 1 Hz of the exact one; where there is none, the owner chose WAVE of the same bit depth, named `.wav` in the dialog and reported in the status bar.
- **Metadata goes before `data`**, because `riff::parse` reads only the head and stops at `data`.
- **`OpenHints::center_freq` becomes `Option<f64>`**, so "not given" differs from 0 Hz. The CLI sets it only when `--center` is present; a session hint of 0 reads as absent, as it means today.
- **Container name**: RIFF when it fits, RF64 otherwise; a BW64 source is written as RF64.
- **Suggested name**: `<stem>_<start>-<end>s.<ext>` for a selection, seconds with millisecond precision, and `<stem>.<ext>` for Save as…; `.wav` for a headerless source, the source's own extension otherwise. The directory is the source's.
- **Target checks**: the target's canonical path (or its parent's plus the name, when it does not exist yet) must differ from the open file's; otherwise the save is refused with a status-bar error.
- **Shutdown**: dropping the save handle cancels it; the shell waits for the worker to remove its temporary file for at most one second. This bounds the wait, it does not guarantee the cleanup: a filesystem that hangs longer leaves the hidden temporary file.
- **Temporary file**: created with `create_new` under a fresh name, so no existing path is ever opened for writing. Cancelling is possible until the rename.
- **The file being written** cannot be opened until the save ends; the status bar says so.
- **API shape**: `argand_io::write::save(request, progress, cancel) -> Result<Saved, WriteError>`, where the request names the source path, its `OpenHints`, its resolved `SignalMeta`, the optional `SampleSpan` and the target. `WriteError` is a `thiserror` enum. No GPUI type crosses into `argand-io`.

## Rejected alternatives

- Always writing f32 WAVE: doubles i16 captures and loses exact equality with the source.
- Writing FLAC sources as WAVE: the owner chose format preservation.
- Copying through `MmapSource`: it hands out normalized `f32`, not stored bytes.
- A modal progress window: a new custom surface for no gain over the status bar.

## Implementation steps

- [x] Confirm the `auxi` layout (SDRangel's `wavfilerecord.h`, which reads and writes SDR# files); record it in a test.
- [x] `riff`: scan any WAVE layout (raw `fmt `, block alignment, data range); read `argd` and `auxi`.
- [x] `OpenHints::center_freq` as `Option<f64>`; CLI, session and callers; reading the frequency from files on opening.
- [x] `argand_io::write`: WAVE/RF64 writer with `fmt `, `auxi`, `argd`, byte copy from WAVE and headerless sources, progress and cancellation, temporary file and rename.
- [x] FLAC writer with `flacenc` (minimal features), Vorbis comment, STREAMINFO rewrite; FLAC comment read on opening.
- [x] Tests: round trips for every sample type and container, I/Q pairs intact, spans at both edges, RF64 above the threshold (threshold injectable in tests), cancellation and failure leave no file and an existing target unchanged, frequency round trip and precedence.
- [x] App: `SaveAs` and `SaveSelectionAs` actions, bindings, File menu rows with enabled state, dialog, open-file refusal.
- [x] App: save worker, status-bar progress with cancel, result and error, bounded wait on shutdown.
- [x] Headless tests: command availability and the status-bar notice; refusal of the open file and cancellation are covered in `argand-io`, where they are decided.
- [x] Update `AGENTS.md`, `IMPLEMENTATION_PLAN.md` and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The owner checks the release binary: saving a selection and the whole capture from WAVE, headerless and FLAC sources, reopening them with the frequency, cancelling a large save.
