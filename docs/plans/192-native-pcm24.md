# Issue #192: Read packed 24-bit PCM natively

Resolves #192: <https://github.com/o-kos/argand/issues/192>.

## Overview

Read full-width signed little-endian 24-bit PCM through the bounded memory-mapped
reader in RIFF, RF64, BW64 and headerless files. Allow supported PCM24 saves to
use RF64 and reopen them with their original sample type and integer units.

## Roles and risk

- Class: A; public core sample types, numerical decoding and file safety.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Context and decisions

- The owner approved `rl_i24` and `iq_i24`, including raw inputs.
- The owner deferred reduced valid-bit support. Extensible PCM24 with fewer valid
  bits retains the existing decoder fallback and unsupported RF64 save protection.
  Do not alter the existing adjusted/computed-save rejection for these layouts.
- Reuse the native RF64/BW64 parser, mmap traversal, normalization and writer.
  Symphonia 0.5.5 already decodes PCM24 but its WAVE reader only accepts RIFF.
  A new container library or dependency solely for scalar decoding is unnecessary;
  use standard fixed-byte integer conversion consistent with existing formats.
- Existing standard sample-format Selects gain entries, without custom controls.
  FLAC conversion remains limited to the existing U8/I16 encoder support.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Correct packed scalar values and whole I/Q frames | Independent byte vectors, tails, offsets, seek and split RIFF data runs | Pending | Little-endian only, matching existing PCM types |
| Native RIFF/RF64/BW64 and raw input, including files over 4 GiB | Native type/divisor assertions and sparse 64-bit files with end sentinels | Pending | Sparse fixture does not scan every byte |
| Normalization, gain and original integer units agree | None/factor/auto/bounded scans, gain and inverse unit reconstruction | Pending | Float scaling tolerances stated in tests |
| Supported PCM24 saves can use RF64 and reopen | Forced small RIFF threshold, exact payload/metadata and reopened values | Pending | Unsupported layouts retain generic refusal |
| Reduced-valid PCM24 preserves existing behavior | Decoder fallback and copied-header/payload tests; computed-save refusal | Pending | New partial-valid support explicitly deferred |
| Sample-format UI offers Int 24-bit for real and I/Q WAVE conversion | Current production application: open File information, choose both entries with pointer/keyboard, dismiss/commit, save/reopen; compare Int 16-bit; verify FLAC filtering | Pending | Native input capability currently unavailable; headless evidence cannot establish UI acceptance |

Numerical reader reference: at Fs = 2.4 MHz and reference = 100 MHz, stored scalars
`[-8388608, -1, 0, 8388607]` decode exactly to
`[-1, -2^-23, 0, 1-2^-23]` with nominal normalization. Real length is four samples;
I/Q length is two samples (duration 2/Fs). Unadjusted save/reopen preserves bytes,
rate, reference and length. Gain and explicit normalization use the same inverse
original-unit affine mapping as other native integer formats.

## Implementation steps

- [ ] Add I24 sample metadata, parsing, packed decoding and peak scanning.
- [ ] Route full-valid WAVE PCM24 natively and preserve partial-valid fallback.
- [ ] Support I24 encoding and existing GUI/CLI sample-type presentation.
- [ ] Add numerical, mapping, large-file and save/reopen regression coverage.
- [ ] Update current architecture, supported-format documentation and changelog.
- [ ] Complete full local validation and independent review.
- [ ] Exercise native UI acceptance, or record the unavailable capability and retain Draft.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

Initial plan-only publication uses scoped diff/link validation; the code gate does
not apply until implementation. Subsequent code pushes require the full local gate.

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Independent read-only review of the exact submitted snapshot.
- [ ] Running-application UI acceptance, subject to the capability limit above.

## Post-completion

After owner acceptance and current full CI, follow the normal merge and conditional
branch cleanup policy. This implementation stays Draft while acceptance is incomplete.
