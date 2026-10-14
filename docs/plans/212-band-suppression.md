# Issue #212: Suppress a selected frequency band or rectangle as an undoable edit

Resolves #212.

Class A (DSP correctness, public `argand-dsp` and `argand-io` API, data safety of saving, more than five code files). Implementer: Claude in the session. Reviewer: to be agreed with the owner before the first round.

## Overview

Edit → Suppress band removes the selected frequencies from the capture and keeps everything else: its length, its rate, its reference frequency and the samples outside the band. A band (#202) is suppressed over the whole capture; a rectangle over its time span, with tapered edges outside the span. The computation runs on a thread of its own with progress and a cancel. The result is one edit in the history, undone and redone at once, and saved in the capture's own format.

This is the first edit that computes samples instead of rearranging them, so it adds three things the editing engine does not have yet: a source that holds computed samples, a temporary home for it, and a writer path that encodes those samples into the capture's storage.

Out of scope: a key binding (the owner chooses one at acceptance), suppressing several bands in one pass, and a band-stop with a user-chosen attenuation or transition.

## Context

- `argand_dsp::extract` (#202) moves a band to 0 Hz with an `f64` oscillator, filters it with a Kaiser windowed-sinc low-pass of 80 dB by overlap-save FFT convolution, and decimates. Its transition rules and the Fs/131072 floor bound the filter to about 660 k taps.
- `argand_edit::Capture` is a piece table over sources indexed by `SourceId`; `replace` with a `Clip` of equal length keeps the capture's length. `editing::Editing` keeps the source table, the `History` and one minimap envelope per source, and `Editing::repicture` maps the shown picture onto a new version through `Capture::position_in`.
- Every reader of an edited capture (analysis worker, minimap envelopes, extraction, saving) reopens each source with `argand_io::reopen(meta, hints, stamp)` (#200).
- `argand_io::write::stage` copies linear samples byte for byte and decodes FLAC to exact integers. Every source must share one `Storage` (`common_storage`). `SampleSource::original_sample_units` maps a read value back to stored units as `value * factor + offset`, including normalization, gain and codec depth.
- `saving.rs` runs a save on its own thread with progress at most every 100 ms and a cancel flag; `saving_ui.rs` shows it at the right end of the status bar.
- `Shell::band` holds the selected band, `Shell::selection` its time span; an edit clears the band.

## Decisions

### Filter (`argand_dsp::suppress`)

- The output is `y[n] = x[n] − w[n]·b[n]`, where `b` is the band-pass of `x` and `w` the time taper (1 for a band). Outside the band `b` is below −80 dB, so `y` equals `x` there to `f32` precision; inside it `x − b` leaves about −80 dB.
- The band-pass passes the selected band whole and has its transitions outside it, so the whole band is suppressed and only the transitions touch neighbouring frequencies. The transition is a quarter of the band's width, never narrower than Fs/131072 (as in #202, which bounds the filter to about 660 k taps) and never wider than the room left beside the band.
- Complex capture: the band-pass is the Kaiser low-pass of half the band's width modulated to the band centre, complex taps. A band within Fs/131072 of the whole capture suppresses everything. A band at ±Fs/2 has its transition wrap to the other edge, which is the circular spectrum's own neighbour.
- Real capture: the band-pass is the difference of two real low-passes, `LP(high) − LP(low)`, with the transitions above `high` and below `low`. A band starting at 0 Hz drops the second term and one ending at Fs/2 makes the first a unit impulse, so no transition reaches past the capture's edge and nothing is counted twice at DC or Nyquist.
- A band narrower than Fs/32768 is refused, as in #202.
- Convolution is overlap-save with an FFT four times the filter length rounded up to a power of two, clamped to 1024..2^22, as `Extractor` does; the filter delay is compensated, so output `n` corresponds to input `n`. `low_pass` and the Bessel and sinc helpers are shared with `extract`.
- Rectangle: the taper rises over `R` samples before the span and falls over `R` after it as a raised cosine, where `R` is the filter's half length, so the content of the band fades over the filter's own time resolution and the whole selected span is suppressed. The computed span is the selection widened by `R` on each side and clamped to the capture; no taper is needed at the capture's edges. A band is a rectangle over the whole capture.
- Input is read from the edited capture over the computed span plus the filter's margin on each side, with zeros past the capture's ends, the same feeding `Extractor` uses.

### Computed sources

- The suppressed span is written once, as real or I/Q `f32` on the capture's unit scale, through `argand_io::write::FloatWave` into a scratch folder, and becomes a new `editing::Source` with `computed: Some(Computed)`. The edit is `Capture::replace(span, clip of the new source)`; undo and redo only switch versions.
- `Computed` holds an `Arc<Scratch>` that removes the file when the last reference goes (document, history, clipboard), and the `Encoding` (`factor`, `offset`) of the opened file, taken from its reader's `original_sample_units` when the edit is made. A capture whose reader has no such mapping cannot be suppressed.
- Scratch files live in `<temp>/argand-scratch/<pid>/` while `<temp>/argand-scratch/<pid>.lock` is held with `File::try_lock`. At start-up, every `<pid>.lock` that can be locked belongs to a process that has gone, and its folder is removed. A crash therefore leaves files only until the next start.
- A computed source holds the capture's own rate and channels, so pasting it is allowed whatever the storage; the paste check compares storage only between files.
- `Editing::repicture` keeps the shown picture over the replaced span: a sample neither version holds elsewhere maps to its own position when either version holds a computed source there and the lengths agree. The picture is replaced as a whole once the new analysis completes, as after every other edit, so a suppression over the whole capture never blanks the spectrogram.
- The selection after the edit is the time span the rectangle had, or none for a band; the band is cleared as after every edit, and undo restores each version's selection.

### Saving computed samples (`argand_io::write`)

- `SourceFile` gains `encoding: Option<Encoding>`. A source with an encoding is computed: its `f32` samples are converted to stored units as `value * factor + offset` and encoded in the storage of the first source without one. Its own storage is never compared, and a request whose sources are all computed is refused. `Editing::save_request` therefore puts the opened file first whenever a computed source is read, so a capture suppressed whole still knows its format.
- Linear storage: integer formats (8-bit offset binary, 16, 24 and 32 bits) round to the nearest integer and clamp to the format's range without a warning, as agreed; float formats write `value * factor` as `f32`. Headerless output encodes the same way. FLAC (and FLAC written as WAVE) receives the clamped integers of its depth.
- The scratch file is read through its own handle with its stamp checked, like any other source, so it is never written over and never mixed with another file.

### Commands and the job

- `editing_ui` adds the `SuppressBand` action: Edit → Suppress band, and Suppress band in the spectrogram's context menu, enabled while a band is selected (a band or a rectangle), the capture has not changed on disk, no save or suppression runs, and its file is described. No key binding yet.
- `suppression.rs` (toolkit-neutral) runs the computation on its own thread with the save job's shape: progress every 100 ms through a bounded channel, a cancel flag, and a drop that cancels and waits at most one second for the scratch file to go. The edit is applied only when the document and its version are still the ones it started from.
- While it runs, edits, saving and opening another file wait: the status bar shows `Suppressing…` with progress and a cancelling ×, at the place of a save, and a failure stays there until closed. A cancellation leaves the capture as it was.

## Rejected alternatives

- Keeping computed samples in memory: a band suppressed over a multi-hour capture would hold gigabytes, and the editing engine never holds samples.
- Storing computed samples quantized to the capture's format at once (the owner chose `f32`): repeated edits would add quantization noise each time.
- Re-encoding every sample at save, including untouched ones: copying them byte for byte keeps them exact and keeps the existing writer paths.
- A real band-pass as a modulated low-pass for real captures: its mirror image overlaps the band near 0 Hz and Fs/2 and doubles the gain there; the difference of two low-passes has no mirror.
- Rectangular time edges: a step in the taper splatters the band's content over the whole spectrum as a click.

## Implementation steps

- [x] `argand_dsp::suppress`: `SuppressPlan` (band checks, taps, margin, taper length) and `Suppressor` (overlap-save band-stop with delay compensation and taper), sharing the filter helpers with `extract`.
- [x] DSP tests: a tone inside the band falls by at least 75 dB, samples of tones beyond the transition change by no more than the stopband ripple; real bands at 0 Hz and at Fs/2 and a complex band at ±Fs/2; a rectangle leaves samples before and after its widened span unchanged and suppresses the span itself; a too narrow band is refused; output length equals input length.
- [x] `argand_io::write`: `Encoding`, `SourceFile::encoding`, the storage of the first uncomputed source, encoding for linear (8, 16, 24 and 32-bit PCM, float, unscaled float), headerless and FLAC output with clamping.
- [x] Writer tests: a capture of copied and computed segments saves as 16-bit WAVE, 24-bit, float, headerless and FLAC, untouched samples byte for byte and computed ones encoded and clamped; a request of computed sources only is refused; a computed source whose stamp changed is refused.
- [ ] Scratch folder: lock, removal of folders of gone processes at start-up, `Scratch` removing its file on drop.
- [ ] `editing.rs`: `Computed`, `Editing::suppress`, paste of computed sources, `repicture` over computed spans, `save_request` with the opened file first.
- [ ] `suppression.rs`: the job, reading the edited capture through `EditedSource` and writing the scratch file.
- [ ] `editing_ui.rs` and the status bar: the action, menu rows, enabling, progress, cancel, failure, applying the edit.
- [ ] App tests: suppression of a band and of a rectangle applies one edit with the expected pieces and selection, undo and redo switch it, a stale result is dropped, the scratch file goes with the last reference, saving a suppressed capture writes the computed samples, start-up removes a gone process's folder.
- [ ] `AGENTS.md`, `IMPLEMENTATION_PLAN.md` (phase 5 step 5) and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Manual, release binary: suppress a band holding a strong tone in an I/Q capture and in a real one, and a rectangle over a burst; the spectrogram shows the band dark and nothing else changed; undo and redo; cut and paste across the suppressed span; Save and Save as reopen with the band suppressed; a 16-bit capture saves as 16-bit.
