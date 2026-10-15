# Issue #212: Suppress a selected frequency band or rectangle as an undoable edit

Resolves #212.

Class A (DSP correctness, public `argand-dsp` and `argand-io` API, data safety of saving, more than five code files). Implementer: Claude in the session. Reviewer: to be agreed with the owner before the first round.

## Overview

Edit → Suppress band removes the selected frequencies from the capture and keeps everything else: its length, its rate, its reference frequency and the samples outside the band. A band (#202) is suppressed over the whole capture; a rectangle over its time span, with tapered edges outside the span. The computation runs on a thread of its own with progress and a cancel. The result is one edit in the history, undone and redone at once, and saved in the capture's own format.

This is the first edit that computes samples instead of rearranging them, so it adds three things the editing engine does not have yet: a source that holds computed samples, a temporary home for it, and a writer path that encodes those samples into the capture's storage.

Out of scope: suppressing several bands in one pass, and a band-stop with a user-chosen attenuation or transition.

## Context

- `argand_dsp::extract` (#202) moves a band to 0 Hz with an `f64` oscillator, filters it with a Kaiser windowed-sinc low-pass of 80 dB by overlap-save FFT convolution, and decimates. Its transition rules and the Fs/131072 floor bound the filter to about 660 k taps.
- `argand_edit::Capture` is a piece table over sources indexed by `SourceId`; `replace` with a `Clip` of equal length keeps the capture's length. `editing::Editing` keeps the source table, the `History` and one minimap envelope per source, and `Editing::repicture` maps the shown picture onto a new version through `Capture::position_in`.
- Every reader of an edited capture (analysis worker, minimap envelopes, extraction, saving) reopens each source with `argand_io::reopen(meta, hints, stamp)` (#200).
- `argand_io::write::stage` copies linear samples byte for byte and decodes FLAC to exact integers. Every source must share one `Storage` (`common_storage`). `SampleSource::original_sample_units` maps a read value back to stored units as `value * factor + offset`, including normalization, gain and codec depth.
- `saving.rs` runs a save on its own thread with progress at most every 100 ms and a cancel flag; `saving_ui.rs` shows it at the right end of the status bar.
- `Shell::band` holds the selected band, `Shell::selection` its time span; an edit clears the band.

## Decisions

### Filter (`argand_dsp::suppress`)

- The output is `y[n] = x[n] − w[n]·(b[n] − σ[n]·u[n])`, where `b` is the band-pass of `x`, `u` the band-pass of unit Gaussian noise from a fixed seed, `σ` the noise amplitude of the frequencies beside the band and `w` the time taper (1 for a band). Outside the band `b` and `u` are below −80 dB, so `y` equals `x` there within the stopband ripple; inside it the band's content gives way to noise as loud as its neighbours'. The owner asked for this after the first manual check, where suppressing by 80 dB left a dark hole below the capture's own noise.
- ➕ The noise level is measured before filtering (`NoiseFloor`): the region is cut into at most 256 blocks, up to eight Hann-windowed segments of at most 4096 samples are read in each, and a block's level is the median bin power over the neighbouring bands (as wide as the band, at least 32 bins, starting 8 bins past the transitions so a Hann window's leakage of a tone at the band's edge stays out, and inside the capture), corrected from median to mean, so tones beside the band do not raise it.
- ➕ The residue and the fill cross as cosine and sine: with `A` the band-pass, the input loses its share through `1 − cos(πA/2)` and the fill passes `sin(πA/2)`, and in time the taper's `1 − cos(πw/2)` and `sin(πw/2)`. A noise background stays level through the transitions and the fade, and both functions are smooth in `A`, so the fill keeps `A`'s stopband. They are derived by frequency sampling and truncated to twice `A`'s length, and `A` still passes a fifth of each transition, so a tone at the band's edge stays suppressed after the derivation. For the narrowest band at 10 MHz this costs about a fifth more time than the earlier filters and no more memory. The fill interpolates the levels between block centres, so a background that changes over time is followed. A band without neighbours (the whole capture) is left empty.
- The band-pass passes the selected band whole and has its transitions outside it, so the whole band is suppressed and only the transitions touch neighbouring frequencies. The transition is a quarter of the band's width, never narrower than Fs/131072 (as in #202, which bounds the filter to about 660 k taps) and never wider than the room left beside the band.
- Complex capture: the band-pass is the Kaiser low-pass of half the band's width modulated to the band centre, complex taps. A band within Fs/131072 of the whole capture suppresses everything. A band at ±Fs/2 has its transition wrap to the other edge, which is the circular spectrum's own neighbour.
- Real capture: the band-pass is the difference of two real low-passes, `LP(high) − LP(low)`, with the transitions above `high` and below `low`. A band starting at 0 Hz drops the second term and one ending at Fs/2 makes the first a unit impulse, so no transition reaches past the capture's edge and nothing is counted twice at DC or Nyquist.
- A band narrower than Fs/32768 is refused, as in #202.
- Convolution is overlap-save with an FFT four times the filter length rounded up to a power of two, clamped to 1024..2^22, as `Extractor` does; the filter delay is compensated, so output `n` corresponds to input `n`. `low_pass` and the Bessel and sinc helpers are shared with `extract`.
- Rectangle: the taper rises over `R` samples before the span and falls over `R` after it as a raised cosine, where `R` is the filter's half length, so the content of the band fades over the filter's own time resolution and the whole selected span is suppressed. The computed span is the selection widened by `R` on each side and clamped to the capture; no taper is needed at the capture's edges. A band is a rectangle over the whole capture.
- Input is read from the edited capture over the computed span plus the filter's margin on each side, with zeros past the capture's ends, the same feeding `Extractor` uses.

### Computed sources

- The suppressed span is written once, as real or I/Q `f32` on the capture's unit scale, through `argand_io::write::FloatWave` into a scratch folder, and becomes a new `editing::Source` with `computed: Some(Computed)`. The edit is `Capture::replace(span, clip of the new source)`; undo and redo only switch versions.
- `Source::computed` holds an `Arc<Scratch>` that removes the file when the last reference goes (document, history, clipboard). The `Encoding` (`factor`, `offset`) is the saving document's own, taken from its reader's `original_sample_units` when it opens, so computed samples pasted into another capture are stored on that capture's scale. A capture whose reader has no such mapping cannot be suppressed.
- Scratch files live in `<cache>/argand/scratch/<pid>/` (`dirs::cache_dir`, mode 0700 on Unix) while `<pid>.lock` there is held with `File::try_lock`, never in the shared temporary folder, where other users could read them and which often lives in memory. At start-up, every `<pid>.lock` that can be locked belongs to a process that has gone, and its folder is removed. A crash therefore leaves files only until the next start.
- A computed source holds the capture's own rate and channels, so pasting it is allowed whatever the storage, refused only at another rate or kind. A paste checks only the sources its clip reads, so a clip of computed samples alone does not need the file it was computed from to match.
- `Editing::repicture` keeps the shown picture over the replaced span: a sample neither version holds elsewhere maps to its own position when either version holds a computed source there and the lengths agree. The picture is replaced as a whole once the new analysis completes, as after every other edit, so a suppression over the whole capture never blanks the spectrogram.
- ➕ What was selected stays selected after the edit, its undo and its redo, as the owner chose: every version keeps the band beside its time span (`History<Selection>`), and the edit sets the version before it to the same selection (`History::set_state`). Only the picture then changes, once its analysis lands.

### Saving computed samples (`argand_io::write`)

- `SourceFile` gains `encoding: Option<Encoding>`. A source with an encoding is computed: its `f32` samples are converted to stored units as `value * factor + offset` and encoded in the storage of the first source without one. Its own storage is never compared, and a request whose sources are all computed is refused. `Editing::save_request` therefore puts the opened file first whenever a computed source is read, so a capture suppressed whole still knows its format.
- Linear storage: integer formats (8-bit offset binary, 16, 24 and 32 bits) round to the nearest integer and clamp to the format's range without a warning, as agreed; float formats write `value * factor` as `f32`. Headerless output encodes the same way. FLAC (and FLAC written as WAVE) receives the clamped integers of its depth.
- The scratch file is read through its own handle with its stamp checked, like any other source, so it is never written over and never mixed with another file.

### Commands and the job

- `editing_ui` adds the `SuppressBand` action: Edit → Suppress band, and Suppress band in the spectrogram's context menu, enabled while a band is selected (a band or a rectangle), the capture has not changed on disk, no save or suppression runs, and its file is described. The owner chose Ctrl+Shift+M in the `Plot` context (Cmd+Shift+M on macOS).
- `suppression.rs` (toolkit-neutral) runs the computation on its own thread with the save job's shape: progress every 100 ms through a bounded channel, a cancel flag, and a drop that cancels and waits at most one second for the scratch file to go. The edit is applied only when the document and its version are still the ones it started from.
- While it runs, edits and saving wait: the status bar shows `Suppressing the band…` with progress and a cancelling ×, beside a save's place, and a failure stays there until closed. Opening another file cancels it. A cancellation leaves the capture as it was.

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
- [x] Scratch folder: lock, removal of folders of gone processes at start-up, `Scratch` removing its file on drop.
- [x] `editing.rs`: `Computed`, `Editing::suppress`, paste of computed sources, `repicture` over computed spans, `save_request` with the opened file first.
- [x] `suppression.rs`: the job, reading the edited capture through `EditedSource` and writing the scratch file.
- [x] `editing_ui.rs` and the status bar: the action, menu rows, enabling, progress, cancel, failure, applying the edit.
- [x] App tests: suppression of a band and of a rectangle applies one edit with the expected pieces and selection, undo and redo switch it, a stale result is dropped, the scratch file goes with the last reference, saving a suppressed capture writes the computed samples, start-up removes a gone process's folder.
- [x] Review round 1 (`gpt-6.1-sol` high): seven findings, six accepted. An extensible WAVE whose valid bits are fewer than its container's is refused for computed samples, which would otherwise be stored 16 times quieter. Removing gone instances' folders and taking one happen under `sweep.lock`, so a sweeper holding a stale lock file cannot remove a new instance's folder. A save keeps the scratch files it reads (`SaveOf::_scratch`) until it ends. Save over is busy while a suppression runs. The writer refuses computed samples of another kind than the capture. A suppression limited in time fades over at least 256 samples, so a whole band or a short filter does not cut. Declined: saving a fully computed capture without the opened file, which would need the format carried apart from the sources (header, `auxi`/`argd`, headerless preamble); the refusal is explicit and loses nothing.
- [x] Review round 2 (same reviewer): the round 1 fixes confirmed but one. The valid-bits refusal ran for every linear save, so an extensible WAVE with fewer valid bits could no longer be saved after a plain cut; it now runs only when computed samples are written, with a regression test. The reviewer judged the declined finding an acceptable limitation of this change.
- [x] Review round 3 (same reviewer): the round 2 fix and its test confirmed, no further findings.
- [x] Review round 4 (same reviewer, agreed by the owner after the manual check had changed the behaviour): two findings, both accepted. The neighbouring bins began right past the transition, so a strong tone at a narrow band's edge leaked into most of them through the Hann window and raised the fill by 5 to 10 dB; they now start 8 bins further out and span at least 32. The residue and the fill crossed with `1 − H` and `H`, a 3 dB dip in the middle of both transitions and of the fade in time; the fill now takes the power-complementary gains. Each fix has a test that fails without it.
- [x] Review round 5 (same reviewer, agreed by the owner): two findings, both accepted. The fill's `√(2|H| − |H|²)` lifted its stopband to about −20 dB just past the transition, so an empty part of the spectrum gained audible noise; the residue and the fill now cross as cosine and sine of the band-pass, which keeps the fill below −60 dB past the transitions and the power within 0.5 dB through them. Scratch files sat in the shared temporary folder, readable by other users; they now live in the user's cache folder, mode 0700. Each has a test that fails without it.
- [x] `AGENTS.md`, `IMPLEMENTATION_PLAN.md` (phase 5 step 5) and `CHANGELOG.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Manual, release binary: suppress a band holding a strong tone in an I/Q capture and in a real one, and a rectangle over a burst; the spectrogram shows the band dark and nothing else changed; undo and redo; cut and paste across the suppressed span; Save and Save as reopen with the band suppressed; a 16-bit capture saves as 16-bit.

The owner checked the release binary on 2026-10-15: after the first check the band was filled with its neighbours' noise and the selection kept, and the owner accepted the result. Individual results were not separately reported.
