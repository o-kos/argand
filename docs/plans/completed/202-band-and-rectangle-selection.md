# Issue #202: Select a frequency band or a rectangle and save it as a new file

Resolves #202. Step 4 of phase 5 in `IMPLEMENTATION_PLAN.md`.

Class A (DSP correctness, a public `argand-dsp` API, a new writer path). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

The selection model already holds a time span and a band (#178), but only a time span can be made. This step adds the band and the rectangle, shows them, and saves them as a complex baseband capture centred on the band, at a lower sample rate.

## Context

- `argand_core::Selection { time, band }`, with `FrequencyBand` in hertz. `Shell::selection` is still a bare `Option<SampleSpan>`.
- `PlotView::press` decides a gesture once per press:
  - a left press on the spectrum selects time;
  - Ctrl+left on the spectrum is reserved and does nothing;
  - every other press pans, a left drag on the frequency ruler included.
- Save selection as… (#186) copies stored samples through `write::save`. A band cannot be copied, because it has to be computed.
- The analysis worker reads the edited capture through `argand_edit::EditedSource` over `argand_io` sources. A save thread can build its own in the same way.
- `argand-dsp` has no mixer, filter or decimator yet. `argand-io` must not depend on `argand-dsp`, so the app joins the two.

## Decisions

Agreed with the owner:

1. **Gestures.** A left drag on the frequency ruler selects a band. The ruler then pans with the middle button, Space+drag and the wheel, as the spectrum does. A Ctrl+left drag on the spectrum selects a rectangle.
2. **Sample rate** of a saved band: Fs/D, with D the largest integer that keeps the band and its transition below the new Nyquist rate. There is no fractional resampling.
3. **Format**: I/Q float32 WAVE, RF64 past 4 GB.

Derived here:

- **Selections.** The shell keeps a band beside its time span (`Shell::band` next to `Shell::selection`, the two fields of `Selection`), so code that acts on time alone keeps reading the span.
  - Every gesture replaces the whole selection, and a click without a drag clears it.
  - A band is physical hertz, clamped to the capture's band: −Fs/2..+Fs/2 around the reference for complex, 0..Fs/2 for real.
  - Edit commands (cut, copy, delete, Replace selection) need a time-only selection; their menu rows and keys are disabled otherwise. Save selection as… works for every kind.
  - Undo and redo restore time selections as now; a band does not survive an edit.
- **Display.**
  - The band is tinted over the spectrum across the whole visible time, and the rectangle within its span, with the time selection's `blue_light` at 0.3. A time selection still tints the minimap; a band does not.
  - The status bar shows the band's low and high edges and its width, in the frequency ruler's unit; with a time span it adds start, end and length as now.
- **Extraction** (`argand_dsp::extract`):
  - **Mixer.** An NCO with an `f64` phase accumulator, wrapped every block, moves the band centre to 0 Hz. Real input enters as I with Q = 0.
  - **Rate.** D = ⌊Fs / (1.25·B)⌋, at least 1, so the new rate Fs' = Fs/D leaves at least a quarter of the band as transition.
  - **Filter.** Windowed-sinc low-pass with a Kaiser window, 80 dB stopband. The passband edge is B/2, and the tap count follows Kaiser's formula. The transition is the narrowest of three limits (changed after review):
    - **aliasing:** Fs' − B, so whatever aliases when decimating lands outside the band (B/4 at D = 1);
    - **wrapping:** a quarter of Fs − B, so the stopband stays below the Nyquist rate and half of what lies outside the band is stopped;
    - **mirror of a real capture:** twice the band's distance to 0 Hz or to Fs/2, but no less than Fs/2048.
  - **Mirror guard.** Within Fs/2048 of 0 Hz or Fs/2 a real capture's mirror cannot be told from the signal, so a band that close keeps that part of its mirror. Its DC is its own mirror and keeps the analytic gain of 2.
  - **Whole band.** A complex band within Fs/1024 of the whole capture is kept unfiltered.
  - **Narrowest band.** A band narrower than Fs/65536 is refused as too narrow to save. That bounds the filter to about 1.3 M taps.
  - **Decimator.** Polyphase: it computes only every D-th output, over a ring of past mixed samples.
  - **Delay.** The filter's (N−1)/2 input samples of delay are compensated. Output n is the filtered signal at input sample n·D of the span, and the output holds ⌈len/D⌉ samples.
  - **Edges.** The capture's own samples beyond the span feed the filter where they exist; zeros elsewhere.
  - **Streaming.** It works on bounded blocks (at most 1 Mi samples) and is pure, with no I/O. Cancellation and progress belong to the caller.
- **Writer** (`argand_io::write::FloatIq`):
  - It writes complex `f32` frames to the same `.argand-<pid>-<n>.part` temporary file and renames it over the target, under the same target rules as `save`. The target is none of the sources, protected or the open file.
  - The header carries the `fmt ` rate rounded, plus `argd` with the exact rate and the reference frequency, and `auxi` where its fields fit. It becomes RF64 when the data passes 4 GB.
  - Values stay on the unit scale: full scale is 1.0, as the reader normalizes.
- **Check on the window's thread.** The window checks only the band's edges and width before the dialog (`ExtractPlan::check`). The filter is built on the save thread (changed after review).
- **Save thread** (`saving.rs`):
  - It opens each source of the current version with `open_stamped` and refuses one whose stamp changed.
  - It reads the span plus the filter's margins through an `EditedSource`, extracts and writes.
  - Progress and cancel work as for a copied save.
- **Name.** `<stem>_<low>-<high>kHz.wav` for a band, and `<stem>_<start>-<end>s_<low>-<high>kHz.wav` for a rectangle. Frequencies use three decimals in the unit that keeps them readable (Hz, kHz or MHz).
- **Reference frequency** of the output: the band centre in physical hertz.

## Review

Reviewer `gpt-6.1-sol` high.

- **Round 1** (5 major, 3 minor, 1 nit).
  - **DSP:**
    - a real band near 0 Hz or Fs/2 kept its mirror in the transition;
    - a complex band wider than 0.8·Fs bypassed the filter;
    - a span too long for the extractor's arithmetic was not refused.
  - **Writing:** a cancel during the final sync could still replace the target.
  - **App:**
    - the window built the whole filter on its own thread to check a band;
    - a band clamped in physical hertz could be refused after the reference was taken away;
    - the vertical rectangle and Space+drag on the frequency ruler were not tested;
    - the plan's 25 % claim was wrong.

  All fixed: the mirror and wrapping limits, the whole-band threshold, `Extractor::new` returning an error, `FloatIq::finish` taking the cancel flag, `ExtractPlan::check` for the window, edges clamped after the reference is removed, and tests for each.

  Declined: the nit on colons in doc comments. The comment rule covers `//` comments, not docstrings, and no `//` comment in the diff has a colon or semicolon.

## Rejected alternatives

- **Fractional resampling to exactly the band width.** It is more code and more CPU. The integer rate is at least 1.25·B and below 1.25·B·(D+1)/D, so the saving is small once D is large. It can reach about 60 % at D = 1, for bands just under 0.8·Fs, which are rare to save (corrected after review).
- **Multistage decimation.** A single polyphase stage already costs about the same number of multiply-adds per input sample for any band, because the tap count grows as D does. Multistage would only save memory for very narrow bands, which the Fs/65536 bound already caps.
- **Writing in the source's sample type.** Filtered values are fractional, so quantising them back adds noise and risks clipping.

## Implementation steps

- [x] `argand-dsp::extract`: plan (D, taps, delay), NCO, polyphase decimator, streaming state; tests that a tone inside the band comes out at its offset frequency with unit gain, that a tone outside is at least 75 dB down, the output length and alignment, real input, and block-size independence.
- [x] `argand-io::write::FloatIq`: streaming float I/Q WAVE with RF64, `argd` and `auxi`, the target rules and the temporary file; tests that the file opens with the exact rate and reference frequency, and RF64 with a small limit.
- [x] `Selection` in the shell; band and rectangle gestures in `PlotView::press`; the frequency ruler's pan moved to the middle button and Space+drag; tests for every gesture and both orientations.
- [x] Band and rectangle tint; status-bar band readout; edit commands limited to time selections.
- [x] Save selection as… for a band or rectangle: the extraction job with progress and cancel, the name, and a headless test that saves a band and reopens it.
- [x] Update `AGENTS.md`, `CHANGELOG.md` and `IMPLEMENTATION_PLAN.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.
