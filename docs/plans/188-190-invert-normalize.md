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

### The adjustment

- `argand_core::Adjustment { invert: bool, gain: f64 }`, identity `{ false, 1.0 }`, lives in `argand-core` because both `argand-edit` (reading) and `argand-io` (writing) apply it. It is not called a transform, which in this project means the FFT settings. `Adjustment::apply(values, channels, first)` changes interleaved read values in place, where `first` is the index of the first sample in its own file. ➕ The sign of `(−1)^n` follows that index, not the position in the capture, so a sample reads the same wherever an edit moves it (review round 2).
- The adjustment belongs to the capture version: `Capture` carries one, kept by `delete`, `insert` and `replace`, and two captures differ when their adjustments do. A version that only inverts or only changes the gain has the pieces of the version before it, so `Capture::position_in` keeps the picture where it is until the new analysis lands, and `Editing::is_untouched` is false while an adjustment is on.
- Pieces always hold the sources' own samples: `EditedSource` applies the capture's adjustment to each piece as it reads it.
- ➕ Suppression (#212) computes on the adjusted capture, as shown, with the band as selected, and stores its scratch samples through `Adjustment::inverse` at their own indices, so reading them through the adjustment gives back what was computed (review round 2).
- Extraction reads the adjusted capture, so a band saved as a file is what the window shows.
- The selection, time span and band, is kept as it is in physical hertz by both commands, and the version before is given the same selection (`History::set_state`), as suppression does.

### Normalization

- The peak is measured on the unadjusted capture by a job on its own thread (`levelling.rs`, through `saving::spawn`), reading every sample through `EditedSource` with progress, a cancel and the status-bar item `Measuring the level…`. The gain is `10^(target/20) · full_scale / peak`, replacing any gain before it, so normalizing twice gives the same result as once. Inversion does not change the peak. ➕ The peak is taken piece by piece in stored units, each source through its own read scale, since pasted files may be read on other scales.
- dBFS is relative to the stored format's full scale (128 for 8-bit, `2^(bits−1)` for integers, 1.0 for float), not to the read unit scale that the normalization and gain hints move, because the gain changes what is saved. The read value of full scale is that stored value through the opened file's `Encoding`, so normalization, like suppression, needs the file described and a reader with `original_sample_units`.
- The target comes from a standard gpui-component `Dialog` with an `Input`, −1 dBFS at first and the last target used in this run after that, accepted from −60 to 0 dBFS. A silent capture (peak 0) is refused with a message. A result within 0.001 dB of the gain already in force adds no version.
- The measurement applies only to the document and the version it started from, like suppression; edits and saves wait for it, and opening another file cancels it.

### Saving

- `SaveRequest` gains `adjustment: Adjustment`, whose parity each segment takes from its own source indices. With the identity adjustment every path is unchanged. Otherwise every sample is encoded again: a linear source's stored bytes are decoded to stored values, adjusted around the format's zero (128 for 8-bit offset binary, 0 otherwise) and pushed through `Linear::push`; computed sources are encoded first and adjusted the same way; FLAC (and FLAC written as WAVE) adjustments its exact integers. Integers round and clamp without a warning, as agreed (so inverting −32768 gives 32767). The linear formats `Linear::of` refuses (an extensible header with fewer valid bits than its container) are refused while an adjustment is on.
- After Save over or a whole Save as, the document reads the saved file, which already holds the adjusted samples, so the rebound history starts with the identity adjustment. The shown picture and minimap stay, as for any rebind; only another sample scale draws again.
- The clipboard holds unadjusted pieces and remembers the adjustment of the capture it was copied from. A paste into a capture with another adjustment is refused (`PasteError::Adjustment`), so a paste never silently undoes an inversion or a level. After a save over the file with an adjustment, copies of the replaced file cannot move onto the saved one and are cleared with the existing notice.

### The minimap

- `Editing::minimap` adjustments the composed envelope: the gain scales every cell, a complex inversion turns the Q channel's `(min, max)` into `(−max, −min)`, and a real inversion takes the symmetric hull `(−m, m)` with `m = max(|min|, |max|)`, which errs outward like the rest of the composition, because the parity of each sample is not in the envelope. Nothing is scanned again.

### Commands and display

- Edit → Invert spectrum, a checked row, Ctrl+Shift+I in the `Plot` context (Cmd+Shift+I on macOS), and Edit → Normalize level…, ➕ Ctrl+Shift+N (Cmd+Shift+N), as the owner asked. Both are enabled like suppression: a described capture, not changed on disk, no save, suppression or measurement running.
- ➕ Both commands are disabled while anything is selected (a time span, a band or a rectangle), as the owner decided after the review rounds: with a selection they would read as acting on it alone, which are different, computed operations. They therefore leave nothing selected.
- While an adjustment is on, the status bar shows an item after the file group: `Inverted`, `+3.0 dB`, or both, with a hint saying that the spectrum is inverted or the level changed while reading, and that saving writes it into the file.

## Rejected alternatives

- Normalization as a computed edit (#212): the whole capture would be written to a scratch file in `f32`, gigabytes and minutes for a multi-hour capture.
- Normalization only through Save as: the open capture would never show the new level, and a later edit could not build on it.
- A view setting outside the history, remembered in the session: saving changes the samples, so the state is an edit and belongs where undo and the unsaved question see it.
- An adjustment per piece: a pasted piece would need its own phase for `(−1)^n`, and every piece would have to change on each toggle; the owner decided both operations are whole-capture.
- Mirroring the shown picture at once: the picture held may cover only part of the frequency axis, and the mirrored part is not held; the picture is replaced when the new analysis lands, as after every edit.
- Rescanning the minimap for a real inversion: exact, but a full read of a multi-hour file for a waveform outline; the hull errs outward as the composition already does.
- Pasting unadjusted samples into a capture with another adjustment: what lands would differ silently from what was copied.

## Implementation steps

- [x] `argand_core::Adjustment` and its tests: conjugation, `(−1)^n` from an odd and an even first index, gain.
- [x] `argand_edit`: `Capture` carries an adjustment (`with_adjustment`, `adjustment`), kept by its edits; `EditedSource` applies it; tests across piece boundaries and seeks.
- [x] `argand_io::write`: `SaveRequest::adjustment` and `first`, re-encoding of linear (8, 16, 24 and 32-bit PCM, float, unscaled float), headerless, computed and FLAC samples; tests for each, clamping, a span starting at an odd index, and byte-exact copies with the identity.
- [x] `editing.rs`: invert and gain versions, selection kept, clipboard adjustment and `PasteError::Adjustment`, adjusted minimap, `save_request` with the adjustment, the rebound history's identity, the full-scale value for dBFS.
- [x] `suppression.rs`: read unadjusted and mirror the band through the inversion.
- [x] `levelling.rs`: the peak measurement job and its tests.
- [x] `editing_ui.rs`, `app_menu_ui.rs` and the status bar: the actions, the checked row, the key, the dialog, progress and cancel, the status item and its hint.
- [x] App tests: invert and normalize add one version each, undo and redo, selection kept, a stale measurement dropped, a refused paste, suppression of a band under inversion suppresses the band shown, saving an inverted and a normalized capture writes the adjusted samples.
- [x] `AGENTS.md`, `IMPLEMENTATION_PLAN.md` and `CHANGELOG.md`.
- [x] Review round 1 (`gpt-6.1-sol` high): four findings, two accepted. The peak was measured in read values while the gain is applied in stored units, so a capture pasted from files opened with different normalizations could clip when saved; the peak is now taken per piece in stored units, each source through its own `original_sample_units` (or its encoding when computed), against the stored full scale. The minimap cast the gain to `f32` before multiplying, which overflows for a gain above `f32::MAX` on a tiny float signal; it now multiplies in `f64`. Declined: a real piece pasted at a position of the other parity under an inversion reads negated, since the sign of `(−1)^n` follows the capture position. That is a constant phase of π over the piece, which leaves its spectrum and the inversion itself intact, while refusing such pastes would refuse half of all positions. Declined: the comment rule covers code comments, and the flagged text is in doc comments, which the rule leaves out. Each accepted fix has a test that fails without it.
- [x] Review round 2 (same reviewer): both fixes confirmed and the comment finding withdrawn, but the reviewer challenged the declined parity finding with a concrete case: under a real inversion, copying one sample over its neighbour changes the shown signal's spectrum, because the moved sample's sign changes against everything around it. Accepted, and it is wider than a paste, since a delete of odd length negated everything after it. The parity of `(−1)^n` now counts each sample's index in its own source (`EditedSource` per piece, the writer per segment), so edits move samples as shown. Suppression, whose scratch samples are a source of their own, now computes on the shown capture and stores through the inverse adjustment instead of mirroring the band, and `SaveRequest::first` is gone. Tests cover a delete across the parity, a saved request of two segments of opposite parity, and a rectangle suppressed from an odd sample of an inverted real capture with a gain.
- [x] Review round 3 (same reviewer): two findings, both accepted, both about the parity of a real inversion. A rectangle suppressed from a sample of odd index wrote its scratch samples from index 0, so once the inversion was undone the kept frequencies of that span turned over against their neighbours; the scratch file now starts with one silent sample whenever that aligns the parity (`Suppressed::offset`, `replace_computed` taking the offset). Where the span crosses pieces of different alignment, made by edits under the inversion, the later pieces cannot share the first one's parity; the span reads as computed under the adjustment it was computed with, and its edges against those pieces turn over under the other one, as the pieces' own edges already did. A copy made under a real inversion that a save over the file would move to the other parity is now cleared rather than moved, since it would read turned over (`Clipboard::moved_onto`). Each fix has a test that fails without it. The round limit is reached, so further review of these fixes is the owner's call.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Manual, release binary: invert an I/Q capture with an offset tone and a real one, the tone moves to the mirrored frequency; normalize to −1 dBFS and to −20 dBFS, the status item and the picture's level follow; undo and redo; Save and Save as reopen inverted and at the new level; a 16-bit capture saves as 16-bit; suppress a band of an inverted capture.

## Post-completion

None.
