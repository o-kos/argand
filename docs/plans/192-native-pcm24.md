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
  Symphonia 0.6.1 already decodes PCM24 but its WAVE reader only accepts RIFF.
  Dependency upgrade #230 is independently reviewed in Draft PR #231. This branch
  is stacked on `chore/230-symphonia-06`; PR #229 targets that prerequisite until
  it merges. Preserve the original PCM24 plan commit through the rebase.
  A new container library or dependency solely for scalar decoding is unnecessary;
  use standard fixed-byte integer conversion consistent with existing formats.
- Full-valid PCM24 now reports I24 instead of the decoder I32 representation.
  Explicit sample-type overrides use existing native byte reinterpretation.
  Session history retains original opening hints, not inferred decoder metadata,
  so auto-detected I32 is not persisted as an override.
- Existing standard sample-format Selects gain entries, without custom controls.
  FLAC conversion remains limited to the existing U8/I16 encoder support.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Correct packed scalar values and whole I/Q frames | Independent byte vectors, tails, offsets, seek and split RIFF data runs | Passed independent scalar, odd-buffer, offset and multi-run tests | Little-endian only, matching existing PCM types |
| Native RIFF/RF64/BW64 and raw input, including files over 4 GiB | Native type/divisor assertions and sparse 64-bit files with end sentinels | Passed native type/units and Unix64 sparse end-sentinel tests | Sparse fixture does not scan every byte; no >4GiB transfer |
| Normalization, gain and original integer units agree | None/factor/auto/bounded scans, gain and inverse unit reconstruction | Passed full/bounded scans and inverse unit reconstruction with gain | Float scaling tolerances stated in tests |
| Supported PCM24 saves can use RF64 and reopen | Forced small RIFF threshold, exact payload/metadata and reopened values | Passed actual writer RF64 promotion, ds64/payload and metadata/reopen assertions | Unsupported layouts retain generic refusal |
| Reduced-valid PCM24 preserves existing behavior | Decoder fallback and copied-header/payload tests; computed-save refusal | Passed fallback metadata, copied-payload and generic RF64/computed refusal tests | New partial-valid support explicitly deferred |
| Sample-format UI offers Int 24-bit for real and I/Q WAVE conversion | Current production application: open File information, choose both entries with pointer/keyboard, dismiss/commit, save/reopen; compare Int 16-bit; verify FLAC filtering | Incomplete: new options and FLAC fallback pass headless regressions; native scenario unperformed | Native input capability unavailable; headless evidence cannot establish UI acceptance |

Numerical reader reference: at Fs = 2.4 MHz and reference = 100 MHz, stored scalars
`[-8388608, -1, 0, 8388607]` decode exactly to
`[-1, -2^-23, 0, 1-2^-23]` with nominal normalization. Real length is four samples;
I/Q length is two samples (duration 2/Fs). Unadjusted save/reopen preserves bytes,
rate, reference and length. Gain and explicit normalization use the same inverse
original-unit affine mapping as other native integer formats.

## Implementation steps

- [x] Add I24 sample metadata, parsing, packed decoding and peak scanning.
- [x] Route full-valid WAVE PCM24 natively and preserve partial-valid fallback.
- [x] Support I24 encoding and existing GUI/CLI sample-type presentation.
- [x] Add numerical, mapping, large-file and save/reopen regression coverage.
- [x] Update current architecture, supported-format documentation and changelog.
- [x] Complete full local validation and independent review.
- [x] Exercise native UI acceptance, or record the unavailable capability and retain Draft.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

Initial plan-only publication uses scoped diff/link validation; the code gate does
not apply until implementation. Subsequent code pushes require the full local gate.

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Independent read-only review of the exact submitted snapshot.
- [ ] Running-application UI acceptance, subject to the capability limit above.

## Post-completion

After owner acceptance and current full CI, follow the normal merge and conditional
branch cleanup policy. This implementation stays Draft while acceptance is incomplete.

## Current validation and acceptance limits

The frozen implementation passes fmt, Clippy and 1,155 workspace tests; one manual
release-mode conversion-cost benchmark is ignored. The actual forced-small RF64
save test also passes independently. Source/test content is frozen before the gate;
no assertions are added during validation. The ordered production release build
passed. Round 1 independent review found no substantive code issues; one
documentation finding was accepted and corrected. Round 2 review was clean.
Optional private captures are unavailable; committed/synthetic fixtures run.

The UI acceptance skill's native scenario remains unperformed: available tools have
no native desktop input connector and the computer-use tool disables native apps.
No workaround was used. Pointer/keyboard choice, dismissal/commit and production
appearance of the new Int 24-bit rows still need native acceptance. Automated Select
and FLAC fallback results do not establish it. Keep this active plan unfinished and
the PR Draft while native acceptance is incomplete; move to completed before Ready.

## Independent review dispositions

Round 1 identified a stale paragraph in the current Editing and I/O architecture
that still described PCM24 as unsupported and RF64 support as future #192 work.
Accepted: update that paragraph to distinguish supported full-valid PCM24 from
unsupported layouts, including reduced-valid-bit PCM24. Neighboring copying,
FLAC-depth and RF64 statements were inspected for the same stale claim. No code
findings or rejected findings were reported.

This correction changes documentation only. Scoped diff/link checks and exact
Cargo/crates patch comparison preserve the existing fmt/Clippy/test/release evidence;
no code gate or release rebuild is needed for unchanged source/tests. The retained
`gpt-6.1-sol` high reviewer independently verified the exact documentation-only
delta and unchanged code/binaries; round 2 was clean. The coordinator accepted the
reviewed implementation and validation. One documentation finding was accepted and
corrected, none were rejected, and no new findings remained. Native UI acceptance
remains incomplete, so this plan stays active and the Pull Request stays Draft.
