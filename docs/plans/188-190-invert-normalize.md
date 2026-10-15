# Issues #188 and #190: Invert the spectrum and normalize the level of a capture

Resolves #188. Resolves #190.

Class A (DSP correctness, public `argand-core` API, data safety of saving, more than five code files). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

Two whole-capture operations that change what the capture's samples are without rewriting anything until it is saved:

- Edit → Invert spectrum mirrors the spectrum: a complex capture is conjugated (`Q → −Q`), mirroring its band about the reference frequency; a real one is multiplied by `(−1)^n`, mirroring `0 .. Fs/2` about `Fs/4`.
- Edit → Normalize level… scales the capture so that its peak lands on a chosen level in dBFS, I and Q by one common factor. The peak is the largest `|I + jQ|` of a complex capture and the largest `|x|` of a real one, measured over the whole capture.

Both are applied on the fly while the capture is read, as the owner decided for #188 and chose for #190, and both are versions in the edit history: undone and redone, marking the capture unsaved and asking before it is closed. A save writes them into the file by encoding every sample again; nothing is copied byte for byte while either is on.

Out of scope: inverting or normalizing a time selection (the owner decided whole capture for both), RMS or mean-power targets, DC (I/Q offset) removal, remembering either state in the session.

## Context

- `argand_edit::Capture` is the piece table, `History<Selection>` its versions, and `EditedSource` reads a capture through one opened source per `SourceId`. The analysis worker (`analysis.rs`), extraction (`extraction.rs`) and suppression (`suppression.rs`) read through `EditedSource`; the minimap of an edited capture is composed from per-source envelopes (`minimap::compose`) without reading a sample.
- `argand_io::write::stage` copies linear samples byte for byte and decodes FLAC to exact integers. Computed sources (#212) carry an `Encoding` from read values to stored units and are encoded through `Linear::push` or the FLAC integer path, rounded and clamped.
- `SampleSource::original_sample_units` maps a read value to stored units as `value * factor + offset` and already includes the read-time normalization divisor and gain hint (`Editing::describe_units`). Those only bring stored values onto the unit scale for display; they never change what is saved.
- `editing_ui.rs` owns the edit actions and `Shell::edited` follows every version: the document length, `Analyst::set_edit` (a final-only replacement), the picture held through `Editing::repicture`, the composed minimap and the titles.
- `saving::spawn` runs a job on its own thread with progress and a cancel; suppression uses it with its own status-bar item.

## Decisions

### The transform

- `argand_core::Transform { invert: bool, gain: f64 }`, identity `{ false, 1.0 }`, lives in `argand-core` because both `argand-edit` (reading) and `argand-io` (writing) apply it. `Transform::apply(values, channels, first)` changes interleaved read values in place, where `first` is the capture index of the first sample, so the sign of `(−1)^n` follows the sample's position in the capture.
- The transform belongs to the capture version: `Capture` carries one, kept by `delete`, `insert` and `replace`, and two captures differ when their transforms do. A version that only inverts or only changes the gain has the pieces of the version before it, so `Capture::position_in` keeps the picture where it is until the new analysis lands, and `Editing::is_untouched` is false while a transform is on.
- Pieces always hold the sources' own samples: `EditedSource` applies the capture's transform to whatever it reads, after the pieces are put together. Computed samples are therefore computed in the untransformed domain.
- Suppression (#212) reads the capture with the identity transform and maps the selected band through the inversion to the frequencies it holds there: `[−high, −low]` around the reference for a complex capture, `[Fs/2 − high, Fs/2 − low]` above it for a real one. The scratch samples it writes are then transformed on reading like the rest, and the noise level, measured untransformed, is scaled by the gain on reading too.
- Extraction reads the transformed capture, so a band saved as a file is what the window shows.
- The selection, time span and band, is kept as it is in physical hertz by both commands, and the version before is given the same selection (`History::set_state`), as suppression does.

### Normalization

- The peak is measured on the untransformed capture by a job on its own thread (`levelling.rs`, through `saving::spawn`), reading every sample through `EditedSource` with progress, a cancel and the status-bar item `Measuring the level…`. The gain is `10^(target/20) · full_scale / peak`, replacing any gain before it, so normalizing twice gives the same result as once. Inversion does not change the peak.
- dBFS is relative to the stored format's full scale (128 for 8-bit, `2^(bits−1)` for integers, 1.0 for float), not to the read unit scale that the normalization and gain hints move, because the gain changes what is saved. The read value of full scale is that stored value through the opened file's `Encoding`, so normalization, like suppression, needs the file described and a reader with `original_sample_units`.
- The target comes from a standard gpui-component `Dialog` with an `Input`, −1 dBFS at first and the last target used in this run after that, accepted from −60 to 0 dBFS. A silent capture (peak 0) is refused with a message. A result within 0.001 dB of the gain already in force adds no version.
- The measurement applies only to the document and the version it started from, like suppression; edits and saves wait for it, and opening another file cancels it.

### Saving

- `SaveRequest` gains `transform: Transform` and `first: u64`, the capture index of its first written sample. With the identity transform every path is unchanged. Otherwise every sample is encoded again: a linear source's stored bytes are decoded to stored values, transformed around the format's zero (128 for 8-bit offset binary, 0 otherwise) and pushed through `Linear::push`; computed sources are encoded first and transformed the same way; FLAC (and FLAC written as WAVE) transforms its exact integers. Integers round and clamp without a warning, as agreed (so inverting −32768 gives 32767). The linear formats `Linear::of` refuses (an extensible header with fewer valid bits than its container) are refused while a transform is on.
- After Save over or a whole Save as, the document reads the saved file, which already holds the transformed samples, so the rebound history starts with the identity transform. The shown picture and minimap stay, as for any rebind; only another sample scale draws again.
- The clipboard holds untransformed pieces and remembers the transform of the capture it was copied from. A paste into a capture with another transform is refused (`PasteError::Transform`), so a paste never silently undoes an inversion or a level. After a save over the file with a transform, copies of the replaced file cannot move onto the saved one and are cleared with the existing notice.

### The minimap

- `Editing::minimap` transforms the composed envelope: the gain scales every cell, a complex inversion turns the Q channel's `(min, max)` into `(−max, −min)`, and a real inversion takes the symmetric hull `(−m, m)` with `m = max(|min|, |max|)`, which errs outward like the rest of the composition, because the parity of each sample is not in the envelope. Nothing is scanned again.

### Commands and display

- Edit → Invert spectrum, a checked row, Ctrl+Shift+I in the `Plot` context (Cmd+Shift+I on macOS), and Edit → Normalize level…, without a key. Both are enabled like suppression: a described capture, not changed on disk, no save, suppression or measurement running.
- While a transform is on, the status bar shows an item after the file group: `Inverted`, `+3.0 dB`, or both, with a hint saying that the spectrum is inverted or the level changed while reading, and that saving writes it into the file.

## Rejected alternatives

- Normalization as a computed edit (#212): the whole capture would be written to a scratch file in `f32`, gigabytes and minutes for a multi-hour capture.
- Normalization only through Save as: the open capture would never show the new level, and a later edit could not build on it.
- A view setting outside the history, remembered in the session: saving changes the samples, so the state is an edit and belongs where undo and the unsaved question see it.
- A transform per piece: a pasted piece would need its own phase for `(−1)^n`, and every piece would have to change on each toggle; the owner decided both operations are whole-capture.
- Mirroring the shown picture at once: the picture held may cover only part of the frequency axis, and the mirrored part is not held; the picture is replaced when the new analysis lands, as after every edit.
- Rescanning the minimap for a real inversion: exact, but a full read of a multi-hour file for a waveform outline; the hull errs outward as the composition already does.
- Pasting untransformed samples into a capture with another transform: what lands would differ silently from what was copied.

## Implementation steps

- [ ] `argand_core::Transform` and its tests: conjugation, `(−1)^n` from an odd and an even first index, gain.
- [ ] `argand_edit`: `Capture` carries a transform (`with_transform`, `transform`), kept by its edits; `EditedSource` applies it; tests across piece boundaries and seeks.
- [ ] `argand_io::write`: `SaveRequest::transform` and `first`, re-encoding of linear (8, 16, 24 and 32-bit PCM, float, unscaled float), headerless, computed and FLAC samples; tests for each, clamping, a span starting at an odd index, and byte-exact copies with the identity.
- [ ] `editing.rs`: invert and gain versions, selection kept, clipboard transform and `PasteError::Transform`, transformed minimap, `save_request` with the transform, the rebound history's identity, the full-scale value for dBFS.
- [ ] `suppression.rs`: read untransformed and mirror the band through the inversion.
- [ ] `levelling.rs`: the peak measurement job and its tests.
- [ ] `editing_ui.rs`, `app_menu_ui.rs` and the status bar: the actions, the checked row, the key, the dialog, progress and cancel, the status item and its hint.
- [ ] App tests: invert and normalize add one version each, undo and redo, selection kept, a stale measurement dropped, a refused paste, suppression of a band under inversion suppresses the band shown, saving an inverted and a normalized capture writes the transformed samples.
- [ ] `AGENTS.md`, `IMPLEMENTATION_PLAN.md` and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Manual, release binary: invert an I/Q capture with an offset tone and a real one, the tone moves to the mirrored frequency; normalize to −1 dBFS and to −20 dBFS, the status item and the picture's level follow; undo and redo; Save and Save as reopen inverted and at the new level; a 16-bit capture saves as 16-bit; suppress a band of an inverted capture.

## Post-completion

None.
