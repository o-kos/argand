# Issue #189: Convert a capture

Resolves [#189](https://github.com/o-kos/argand/issues/189).

## Overview

Convert the whole current edited capture into a new WAVE/RF64 file and open it only
once publication succeeds. The source file and document history are unchanged.
There is no selection mode and no conversion Undo. PCM24 remains #192.

## Roles and risk

- Class: A: DSP correctness, public DSP API, asynchronous work and data safety.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Decisions

- The owner explicitly sets target sample rate and reference frequency. Changing
  real/IQ never changes either field. Physical frequency equals reference plus
  baseband frequency (#185). The retained band is the intersection of the input
  and target physical spans; show discarded edges and filter transitions.
- Real input is made analytic; IQ output translates that analytic band to its
  requested reference. Real output translates it above its requested reference
  and takes the real component. Do not conjugate, fold or alias discarded bands.
- Support all five scalar formats in both domains. `f16x8` is arbitrary-scale
  float32, not float16. Output floats retain finite values beyond unity.
- Preserve level is default. Normalize peak uses one gain for I and Q, targeting
  -0.1 dBFS after the transform; zero remains zero. Measure actual transformed
  samples in a bounded first pass, then encode in a second pass. Report gain and
  clipped scalar values. Round integer outputs to nearest, ties away from zero;
  saturate to the asymmetric signed range (u8 offset 128). No dither initially.
- Reject nonfinite samples/parameters. DSP uses f32: i32 conversion can lose low
  bits. A byte-preserving unchanged capture uses the existing save path when
  type/rate/reference and level mode are unchanged.
- Finite FIR transitions and zero padding at capture ends are unavoidable. Use
  1% transitions inside cropped band edges and compensate filter delay. The
  filter design parameter is 80 dB; measured acceptance is the 60 dB tone floor.
  Native real DC/Nyquist boundaries retain their analytic half-gain convention.
  Refuse unsupported narrow transitions/rates before publishing, never silently
  change the requested rate, reference or type.
- Keep one immutable source/version snapshot, source stamps and scratch ownership
  until completion. Cancellation/errors leave source and existing target alone.
- Use stock Dialog, Input, Select and Button, as already composed in
  `editing_ui.rs` and `settings_editor.rs`; no custom ordinary controls.

## Rejected alternatives

- In-place conversion: current History versions only Capture and Selection;
  changing Fs/domain would require a larger metadata and Undo redesign.
- Automatic Fs/4 reference changes: the owner explicitly rejected choosing
  reference frequencies for the user.
- Full-capture buffers: captures may span hours at RF sample rates.

## Implementation steps

- [x] Add validated physical-band conversion planning and bounded DSP streaming.
- [x] Add streaming encoded WAVE/RF64 output with clipping statistics and safe publication.
- [x] Add cancellable conversion orchestration over the complete edited capture;
      preserve level and two-pass peak normalization; exact output metadata.
- [x] Add standard conversion dialog, File command, progress/cancel and successful-result opening.
- [x] Add regression coverage and update architecture/user-facing documentation.
- [ ] Complete local validation and build the current release binary.
- [x] Move this plan to `docs/plans/completed/` before final external review.

## Validation

- Tone tests in both domains and both IQ frequency signs: in-band amplitude error
  below 0.01 (excluding documented transitions and capture-edge transients),
  phase/frequency alignment against analytic tones below 0.02 radians.
- Out-of-band tone amplitude below 0.001 of input away from transition bands.
- Output count ceil(input_count * target_rate / source_rate), duration error
  below one output sample, exact f64 rate/reference reopened from argd metadata.
- Chunk sizes 1, 17 and 4096 produce identical outputs. Bounded output batches
  and filter state independent of capture length; reject unrepresentable counts.
- All ten output types reopen with correct domain/type; integer endpoints,
  ties/offset/saturation/clipped counts; finite float overscale and f16x8 scaling.
- Normalize peak within 0.001 of -0.1 dBFS before integer quantization, one IQ gain;
  silence stays silent. Reject NaN/Inf rather than encoding silent corruption.
- Source mutation, protected source target, cancellation, partial writes and
  publication failures preserve original paths; RIFF/RF64 regression coverage.
- Headless and native UI checks where available: unchanged Fs/reference when
  domain changes, validation, narrowing summary, focus, Enter/Escape, lists,
  busy states, cancellation, and opening only a successfully published result.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after the checks pass
- [ ] Independent review with findings and dispositions recorded in the PR

## Implementation evidence and limits

- Preserve restores the decoded original scale through each file's affine factor,
  divided by that storage's full scale. It deliberately excludes u8's neutral DC
  offset. Computed pieces use their retained Encoding and the capture's full scale;
  Capture gain/inversion is applied once by EditedSource. Tests cover auto-normalized
  real/IQ sources in all ten types and mixed computed pieces.
- Peak normalization uses the existing Normalize command's IQ hypot/real absolute
  peak and -0.1 dBFS target, measured after all DSP in the first bounded pass.
- Full-band identity skips filtering. Compatible WAVE identity preserves raw bytes,
  including i32 low bits; storage depth must match the selected output type.
- Source stamps are checked on opening, after each pass and again after output sync
  before atomic publication. Cancellation drops the staged file. Successful opening
  uses the ordinary unsaved-document transition; Cancel retains the created file
  and original document, with the result path in the status bar.
- DSP release benchmark, one million input frames per case: IQ 1 MHz to 50 kHz
  417 ms (133 ms preparation); equal-rate IQ reference shift 554 ms; equal-rate
  real-to-IQ 392 ms. This excludes file IO and UI. Process high-water RSS 20,092 KiB
  across the three cases. This is measured typical-case memory, not a universal cap.
- Memory is independent of capture length: 4096-frame source/output blocks plus
  bounded FFT overlap-save and reconstruction state. Transitions narrower than
  Fs/131072 (including the 1% guard) are refused before run; the existing filter
  FFT and phase-bank limits bound worst-case storage. No full-capture allocation.
- Actual toolkit headless tests exercise editable rate, type changes, invalid bands,
  focus and dialog/list Escape. Native interactive coverage remains unavailable:
  the environment exposes no supported keyboard/pointer automation, and an existing
  owner's older application process was left untouched. Native UI acceptance must
  not be inferred from headless tests or a release build.
- The pre-existing FFT summary spacing defect is tracked separately in #225.

## Post-completion

Merge/release/tag require separate owner action. The initial plan-only push uses
scoped documentation validation (reviewed content and links); no code changes are
included and the existing AGENTS.md edit is excluded. Code pushes use the full gate.
