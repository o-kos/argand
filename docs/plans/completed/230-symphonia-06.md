# Issue #230: Upgrade Symphonia to 0.6.1

Resolves #230: <https://github.com/o-kos/argand/issues/230>.

## Overview

Upgrade the existing WAVE/PCM/FLAC decoder graph from 0.5.5 to stable 0.6.1.
Adapt the breaking APIs while preserving Argand's existing reader and save contracts.
Native PCM24/raw formats and RF64 support remain separate work in #192. Tagged
Symphonia 0.6.1 still accepts only RIFF WAVE, so this upgrade does not solve #192.

## Roles and risk

- Class: A; decoder numerical behavior, bounded buffering and source/file safety.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Context and decisions

- The owner requested the dependency upgrade before continuing #192.
- Use the official 0.6 migration guide and tagged 0.6.1 source. Its MSRV 1.85
  fits the repository's Rust 1.97.1 toolchain.
- Preserve existing enabled formats and codecs; do not enable unrelated formats.
- Migrate typed audio codec parameters, track timing, explicit EOF, checked timestamp
  arithmetic, metadata and interleaved sample-copy APIs. Preserve gain, normalization,
  original units, immutable opened handles, source stamps and strict-save validation.
- Reduced-valid-bit feature support remains explicitly deferred by the owner.
- Only explicit `Ok(None)` ends packet reads normally. Unexpected EOF and a reset
  request propagate as errors rather than masquerading as a finished capture. The
  enabled RIFF/FLAC/PCM 0.6.1 sources contain no `ResetRequired` emission; unsupported
  format resets must not silently shorten a capture.
- No UI controls or application interaction contracts change. Native UI acceptance is
  not required for this decoder-only migration; production launch may be a smoke check.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Existing codecs use compatible Symphonia 0.6.1 | Targeted lock update and dependency graph inspection | Symphonia family at 0.6.1; only required new support crates | No unrelated broad update |
| Decoded values, levels and metadata retain their meaning | Decoder WAVE/FLAC unit and existing I/O tests | Standard PCM/float cross-reader and FLAC fixture tests passed | No new formats |
| Exact seek, reopen and source identity remain intact | Independent cursor, replacement/change and seek regressions | Seek/reopen/replacement and audio-track tests passed | Platform-specific identity limits unchanged |
| EOF and damaged frames preserve safe-save behavior | Full stream and strict corruption/timestamp tests | Clean EOF, final-frame truncation and strict-save regressions passed | Declared-length FLAC truncation covered; private captures unavailable |
| Current workspace passes required checks | fmt, Clippy, tests, then release build | All four checks passed; release build completed after tests | Local checks do not establish remote CI |
| Independent review covers exact current snapshot | Neutral packet outside worktree and separate reviewer | Round 1 finding accepted; round 2 pending | Coordinator assesses findings |

## Implementation steps

- [x] Upgrade only the Symphonia dependency family and adapt decoder APIs.
- [x] Verify explicit EOF, timestamps, sample-copy, metadata and reopen contracts.
- [x] Update relevant current documentation and user-visible changelog if warranted.
- [x] Complete local validation and build the current release binary.
- [ ] Complete independent review and record dispositions.
- [x] Move this plan to `docs/plans/completed/` before Ready/final review.

## Validation

Initial plan-only publication uses scoped diff/link checks and skips the code gate.
Code pushes require the complete local gate. Preserve the production feature set.

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after checks pass
- [x] Targeted decoder and safe-save regression evidence.
- [ ] Independent read-only review of the exact submitted state.

Workspace tests passed: 1,150 tests; one manual release-mode conversion cost
measurement is ignored. Tests use committed and synthetic FLAC/WAVE fixtures;
optional private captures are not available in this worktree. Native app interaction
and remote CI are not claimed by these automated results.

## Post-completion

Keep the PR Draft until owner acceptance. After current full CI and acceptance,
follow normal merge and conditional branch cleanup. Resume #192 on this validated
prerequisite with an explicit dependency if it is not yet merged.

## Review dispositions

Round 1 identified premature clean EOF when a known-length FLAC ends inside its
first audio frame, or its only remaining frame fails decoding. Accepted: retain
the checked decoded packet-end timestamp in normal and strict mode and reject EOF
before the known length. Unknown-length scans remain unconstrained; normal mode
still skips damaged interior packets. Added partial/corrupt-first-frame read,
repeated read, seek-zero, auto-normalization and strict-save regressions. Extended
valid final-block seeks and exact-endpoint seeks; the endpoint is handled locally
because the demuxer requires a frame to seek to. No findings were rejected.

Refreshed fmt, Clippy, all 1,150 workspace tests (one ignored manual benchmark),
and the ordered release build passed after freezing the final code/test snapshot.
Round 2 independent review is pending.
