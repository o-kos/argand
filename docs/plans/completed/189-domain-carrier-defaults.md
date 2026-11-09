# Domain conversion carrier and source-derived defaults

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

## Classification and roles

Class A: signal-domain and physical-frequency correctness, conversion metadata.
Coordinator: gpt-6-astra medium; executor: gpt-6.1-sol medium;
independent reviewer: gpt-6.1-sol high, retaining the existing reviewer assignment.

## Approved behavior and invariants

- On a real/IQ domain transition, derive draft defaults from the SOURCE, never
  from a previous draft: IQ -> real doubles source Fs and sets carrier C=source
  Fs/2; real -> IQ halves source Fs and sets C=source Fs/4.
- Both target Fs and C remain editable before Convert. Editing target Fs does not
  move C. Numeric-encoding-only changes within one domain preserve drafts.
  Returning to the source domain restores source Fs and physical reference.
- The cross-domain Reference control represents the positive-frequency position
  C of IQ zero in the real array, distinct from physical metadata reference R.
  IQ -> real derives Rreal=Riq-C; real -> IQ derives Riq=Rreal+C. Same-domain
  requests keep original physical reference. Existing physical metadata semantics
  and the independently shiftable DSP API remain unchanged.
- The default IQ Fs=24000, R=0 -> real Fs=48000, C=12000 maps array frequencies
  [-12000,+12000] to [0,24000], with physical metadata Rreal=-12000. Reversing
  that result uses C=12000, IQ Fs=24000 and restores Riq=0. Physical tones do not
  change their physical frequencies. Carrier is not written as metadata origin.
- Custom Fs/C choices retain the existing intersection/filter rules. The warning
  appears for actual nominal input-band loss; the full-band defaults have none.
  Nonfinite rates/carriers/references and empty intersections fail before running.
- Existing finite filter transitions remain; nominal full-band coverage is not
  a promise of unity gain at every Nyquist edge. IQ endpoints coincide modulo
  its rate; exactly coincident boundaries cannot preserve two independently
  encoded real edge tones or arbitrary complex edge phase. Interior tones and
  existing unambiguous endpoint guarantees must not regress.
- Preserve existing adjusted-sample amplitude, atomic unsaved-document adoption,
  cancellation, source safety, no Undo, first Save behavior and bounded memory.
  No toolkit upgrade or new custom UI controls.

## Work and acceptance

- [x] Implement source-derived domain defaults using the existing Select event
      and NumericEditor, preserving same-domain drafts and resetting on return.
- [x] Convert the cross-domain carrier to physical output reference in the
      application request, with finite validation and concise field explanation.
- [x] Verify IQ -> real and reverse default Fs/carrier/metadata, positive and
      negative interior tones, amplitude, duration/count and roundtrip gain.
- [x] Verify custom Fs leaves C unchanged, custom C warns only on actual band
      loss, fractional rates, same-domain encoding changes and repeated toggles.
- [x] Update current architecture documentation without rewriting completed plans.
- [x] Freeze exact revision for independent review; address substantive findings.
- [x] Full formatting/Clippy/test gate and coordinator assessment.
- [x] Fresh release build.
- [ ] Native visual verification: not completed; invalid diagnostic capture,
      no further launch to avoid interfering with the owner's input.
- [x] Complete plan before final review; retain the existing Draft and required push hook.

## Numerical validation

Use independent expected tones in output array coordinates and metadata physical
coordinates, including input IQ Fs=24000/R=0 with tones -6000 and +6000 mapping to
real 6000 and 18000 at Fs=48000/R=-12000. Reverse conversion restores each IQ
frequency and reference. Test individual tones and superposition; compare interior
amplitudes after filter margins against the existing 0.1 dB passband criterion,
with roundtrip within 0.2 dB. Preserve output count ceil(N*Fsout/Fsin), exactly 2N
for doubling and ceil(N/2) for halving; do not claim exact duration for odd real N.
Retain existing anti-alias rejection tests and their measured quality floor rather
than claiming stronger rejection from a filter name. Include non-integer Fs and
nonzero physical references so carrier and physical origin cannot be conflated.

## Focused evidence

All 17 conversion-panel regressions pass, including opening another source while
an old carrier draft exists. The normal document transition closes the old panel;
the new real source derives fresh reverse defaults. Four focused carrier tests
pass, including actual encoded output and reopened metadata, independent complex
and real tone projections, custom partial-band warning, finite carrier bounds,
source-based toggles and current numeric-editor keyboard behavior. No DSP backend
semantics were changed. The existing NumericEditor composes the same standard
controls; no new custom-control exception is required.

## Accepted precision correction

The first independent review found that deriving the oscillator shift by subtracting
rounded absolute metadata references loses the requested carrier at large references
or fractional rates. Preserve the exact signed carrier delta separately in the app
request. A narrowly named DSP constructor accepts that explicit input-local delta;
the existing metadata-based constructor keeps its behavior. Local intersections,
filter construction and oscillator use only local bounds/delta. Warning decisions
compare native local bounds, not rounded physical labels. Output physical metadata
is the nearest representable f64; at very large references it cannot express an
arbitrarily small carrier exactly, and no stronger metadata precision is promised.
Add regressions at R=1e20 and fractional Fs=8000.15/R=1e10, including default full
coverage, independent array-frequency/phase/amplitude checks and roundtrip metadata.

## Review record

Round 1 accepted the carrier precision finding: subtracting rounded absolute
references could shift the requested carrier or report false band loss. The fix
retains exact carrier provenance, derives the signed local delta explicitly and
bases band-loss decisions on local edges. No findings were rejected. Five app
carrier regressions pass, including actual encoded/reopened high-reference and
fractional-rate roundtrips. The DSP regression checks full local coverage, the
existing metadata-based constructor and phase at absolute output index 2^40+13.
Round 2 was clean. The final tests-only delta extracts source setup to satisfy
Clippy without changing the runtime or assertions; that bounded delta also received
a clean independent review. The refreshed full local gate
passed: formatting, all-targets Clippy and 1134 tests, no failures, one ignored
manual benchmark. The production release rebuilt successfully after the gate.

The attempted actual diagnostic capture is invalid: its sample-rate field contains
unexpected text and does not show the intended valid defaults. No native visual
acceptance is claimed for this iteration. No further diagnostic launch or input
emulation is performed; supported GPUI integration tests verify the defaults and
absence of a warning. Native visual checking remains an explicit delivery limit.
