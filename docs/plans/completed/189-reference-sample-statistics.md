# Conversion reference policy and whole-capture amplitude statistics

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

## Classification and roles

Class A: asynchronous statistics, source retention and numerical correctness.
Coordinator: gpt-6-astra medium; executor: gpt-6.1-sol medium;
independent reviewer: gpt-6.1-sol high.

## Approved contracts

- Reference is editable only across real/IQ domains. Returning to the source
  domain restores the original reference in the control and application request.
  Encoding changes do not unlock it; sample rate is always explicitly entered.
  The DSP API retains independent frequency-shift support.
- Replace channel extrema with Peak and RMS of the entire current edited capture.
  For IQ, peak is max hypot(I,Q), RMS is sqrt(sum(I²+Q²)/logical sample count).
  Real values use absolute magnitude. Empty/nonfinite/unrepresentable results are
  unavailable; pending work is shown concisely, never as old extrema.
- Units follow existing stored, centered amplitude scaling per CURRENT SourceFile
  encoding. U8's neutral 128 offset is not signal DC; no universal dBFS claim.
  Gain/inversion are applied analytically with presentation-level f32 tolerance.
- Accumulate at most 4096 source cells only during the minimap's full sequential
  pass, not overlapping sparse preview reads. Retain a bounded aggregate tree
  (~256 KiB/source) with existing retained sources, not a disk index.
- Edited-piece queries merge complete cells and read only included boundary
  portions on a worker. Coalesce/cache boundary ranges with a bounded cache.
  A boundary cell may cover N/4096 frames: very long captures can incur material
  boundary IO. Fragmented pieces can collectively reread all selected frames;
  there is no separate unconditional full-capture scan on each edit. Identical
  boundary intervals reuse summaries while duplicate pieces retain multiplicity.
- Keep source stamps, worker leases, document/capture-generation signatures and
  cancellation. Never publish a stale summary. Open File information updates
  when current statistics arrive.
- Reuse standard Input and Button disabled states in the shared NumericEditor;
  retain only its already approved press/repeat behavior exception.

## Work and acceptance

- [x] Implement reference enable/reset and application request enforcement.
- [x] Build sequential-only bounded source aggregates and background piece queries.
- [x] Publish guarded current metrics and live File information rows.
- [x] Verify independent expected metrics across chunks, partial cut/paste, mixed
      source scaling, gain/inversion, nonfinite/empty input and boundary IO budget.
- [x] Verify reference domain toggles, encoding-only changes, disabled interaction
      and explicit sample-rate preservation.
- [ ] Independent review; full format/Clippy/test gate; current release rebuild;
      actual diagnostic panel render on second monitor without input emulation.
- [ ] Complete this plan before final review and keep PR Draft.

## Toolkit and numerical evidence

The locked gpui-component 0.6.6 Input supports disabled editing, but its Tab
registration remains active (and its tab_index builder is unused by rendering).
The existing shared scoped traversal skips disabled numeric handles. This is
composition of supported focus movement, not a new control or exception.

Each aggregate node is 32 bytes, including energy, magnitude peak, component
peak and invalid flag. At most 8192 nodes cost 256 KiB per retained source.
The boundary cache holds at most 128 summaries with weak index identities, not
sample arrays or old source mappings. Worker read buffers contain 65536 f32s.
Sources/scratches and leases stay worker-owned until cancellation/completion.

Analytical gain statistics agree with per-component f32 adjusted samples within
2e-7 relative plus one minimum f32 subnormal in decoded units (scaled into stored
units for presentation). Component maxima detect f32 overflow before applying
stored-unit factors. Nonfinite flags are local to cells; excluded invalid samples
must not poison an otherwise valid edited capture. Empty RMS is unavailable.

Scheduling compares only document/capture versions, adjustment, changed status and
an Editing source-description revision. Creating/rebinding Editing gives a new
revision; source stamp/format/unit descriptions advance it. Completed renders,
FFT/viewport changes and draft typing do not walk or clone the piece/source table.

## Review record and validation

Round 1 accepted two substantive findings: a saved-source rebind reused a complete
waveform without building the new source statistics index; and the overflow
check rejected a value just beyond f32::MAX that rounds to a finite f32. Rebinding
now scans the new source independently of envelope reuse; overflow checks the
actual f32 cast. Both have independent regression tests. None were rejected.

The render-path performance observation was accepted: scheduling now compares a
fixed-size source-description/capture key, without source_files or piece sorting.
A discovered scan-failure gap reports Unavailable instead of permanent pending.
Reference fixtures deliver distinct domain transitions; the live panel regression
preserves draft text and focus when metrics update.

Focused metric regressions pass for all ten native types with initial normalization,
current gain/inversion, duplicate partial pieces and current computed encoding,
cached source mutation, source completion after editing, and same-format save/rebind.
Boundary counts measure requested decoded frames; codec seeks may decode extra packets.
Arbitrarily fragmented edits may collectively read their whole selected content.
Read-only amplitude presentation uses seven significant figures and scientific
notation for extremes; editable frequencies retain full precision.

Round 2 accepted a terminal scan-failure lifecycle gap: resetting a capture query
could lose Unavailable while a retained source could never deliver an index. Index
failure is now retained per source, survives capture and source-description changes,
and only affects captures still using that source. Removing it recovers; undo
restores the terminal state. A successful new complete index clears that failure.
Root and pasted-source regressions cover retention, removal, and scheduler status.

The full local gate passed before the final failure-lifecycle correction (1127 tests,
one manual benchmark ignored). Final follow-up review, refreshed gate, release
rebuild and actual current panel render remain pending before delivery.
