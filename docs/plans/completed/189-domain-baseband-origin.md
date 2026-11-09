# Domain conversion produces a zero-origin baseband document

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

## Classification and roles

Class A: DSP/application mapping and persisted signal metadata.
Coordinator: gpt-6-astra medium; executor: gpt-6.1-sol medium;
independent reviewer: gpt-6.1-sol high, retaining the existing assignment.

## Approved contract

- Real/IQ domain conversion produces output center_freq=0. The real result spans
  0..Fs/2; the IQ result spans -Fs/2..Fs/2. This changes actual output metadata,
  including scratch, adopted document and saved/reopened WAV/FLAC, not axis labels.
- Source physical reference does not affect sample-domain conversion. Exact signed
  carrier delta remains separate: IQ -> real uses -C; real -> IQ uses +C.
  Keep the existing metadata-based DSP constructor unchanged.
- Preserve source-derived rate/carrier defaults, explicit subsequent edits and
  source-domain return behavior. Same-domain encoding conversion keeps source R.
- Band-loss warnings describe retained SOURCE ARRAY frequencies, excluding old RF
  origin. Default full-band mappings do not warn; custom partial mappings do.
- Preserve gain, atomic unsaved adoption, no Undo, original disk safety, first Save,
  bounded streaming, filter/endpoint limits and current controls. No toolkit change.

## Work and acceptance

- [x] Separate zero output metadata from exact carrier validation/mapping.
- [x] Show local input band in loss warnings; update current architecture notes.
- [x] Verify IQ Fs12000 -> real Fs24000/C6000 and reverse, independent tones,
      counts/amplitude, custom real Fs8000/C1800, nonzero and huge source R,
      persisted zero metadata, same-domain reference retention, WAV and FLAC.
- [x] Complete plan before final independent review; address findings.
- [x] Run full formatting/Clippy/test gate, then build current release.
- [x] Update existing Draft PR and push through required hook after acceptance.

## Validation boundaries

Use independent expected array frequencies and metadata assertions, retaining
existing anti-alias and amplitude criteria. Do not claim lossless Nyquist edges.
No native GUI launch or input automation: automated toolkit coverage verifies
panel defaults/warnings; native visual validation remains unperformed.

## Focused evidence

Fourteen application conversion tests and eighteen conversion-panel tests pass.
Independent projections check passband amplitude within 0.1 dB forward and 0.2 dB
roundtrip for two-sided tones, including source Fs12000, fractional rates and huge
source physical references. A custom Fs8000/C1800 conversion places an input-array
200 Hz tone at real-array 2000 Hz and reopens with span 0..4000 Hz. Actual encoded
scratch and subsequent WAV/FLAC saves reopen with zero origin in both domains.
The panel warning is asserted in input-array coordinates despite source R100000;
same-domain/return-to-source reference retention remains covered.

The DSP algorithm is unchanged: only a read-only local-band reporting accessor is
added. Global physical axes and the metadata-based DSP constructor are unchanged.

## Authorized safety and presentation follow-up

The owner additionally requires the following in this iteration. The plan returns
active while this expanded work is implemented; historical completed plans stay
unchanged.

- Retain the source user file's parent as save-presentation provenance through
  repeated unsaved conversions. First Save starts there; a successful saved-file
  rebind updates the parent naturally. Validate directories off the UI thread.
- Place Convert to... last in Edit.
- Refuse document mutation/open/save races during conversion and prepared-result
  opening, retaining Cancel. Reuse computing() for shared edit/save execution
  guards and add explicit conversion guards to document replacement/panel entry
  and async chooser continuations. Hide coordinate readouts while conversion runs.
- Use the stock Select's grouped searchable delegate to distinguish Real and I/Q
  sections, retaining explicit domain in collapsed selection. Label arbitrary-scale
  IEEE f32 as Float 32-bit (unscaled), with arbitrary units and CoolEdit 16×8
  explained in the tooltip; distinguish Float 32-bit (normalized), whose nominal
  scale is ±1, without changing decoding or initial reader normalization.
- A click on the application caption close button with a Select open must enter
  the normal guarded close workflow once. Verify actual GPUI hit-tested pointer
  delivery and unsaved protection; no input replay or custom Select replacement.

### Toolkit and encoding evidence

Locked gpui-component 0.6.6 Select supports SearchableVec<SelectGroup<I>>; its
standard section delegate supplies named list headers and normal grouped keyboard
selection. f16x8 reads/writes IEEE f32 bytes with the CoolEdit WAVE marker, permits
arbitrary amplitude units and has full_scale=1, rather than imposing a 16-bit scale.
Existing Linux caption controls remain a recorded exception. Select's outside
Capture listener stops propagation while open. The title-bar capture synchronously
dispatches the standard Cancel through the saved Select trigger before dismissing
the panel; the old listener then propagates the original press. Caption click and
release-outside behavior remain unchanged. The pointer regression fails without
this composition and passes with it, including the normal unsaved dialog.

- [x] Retain and test source-parent save provenance through repeated conversion.
- [x] Move menu command; group sample types and clarify f16x8 label.
- [x] Cover busy action execution, chooser races, panel entry, coordinate hiding
      and safe cancellation/close behavior.
- [x] Reproduce and fix one-click caption close with Select open using supported
      event composition; preserve normal unsaved workflow.
- [x] Complete follow-up, independent review, full gate and fresh release.

### Expanded review dispositions

The first expanded review accepted a queued-success cancellation race: cancelling
the worker did not invalidate a success already waiting in the UI mailbox. A
single retained cancellation intent now discards every terminal result after
Cancel, then either retains the document or resumes the requested normal close
workflow. A deterministic queued-success regression checks unchanged document,
capture, version and unsaved state. No finding has been rejected.

The second expanded review and final test-only fixture delta were clean. The
full local formatting, all-targets Clippy and locked workspace test gate passed:
1,139 tests, zero failures, one ignored manual benchmark. The release build then
completed successfully. The normal production binary is checked separately from
test targets. The conversion UI suite has 21 passing tests, including queued
success cancellation, busy actions, source-parent provenance and caption pointer
delivery. The native GUI validation boundary above remains unchanged.
