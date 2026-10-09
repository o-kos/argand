# Issue #204: Read WAV files whose samples span several data chunks (KiwiSDR)

Resolves #204.

Class A (data safety in the save path, `argand-io` public API). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

KiwiSDR recordings put a `kiwi` chunk with GNSS time before every `data` chunk of 512 samples. The reader stopped at the first `data` chunk, so a 146 s recording opened as 0.04 s.

## Decisions

- `riff::WavLayout::runs` and `Chunks::runs` walk a plain RIFF past the first `data` chunk up to the stated RIFF end (bounded by the file) and return every `data` body as a `DataRun`. RF64/BW64, a first chunk without a usable length and a file with nothing after its first chunk are one run, without a walk.
- `MmapSource::from_wave` reads the runs as one array; each run is cut to whole samples, `read` continues across runs, seek/prefetch/release map samples to bytes by binary search.
- `normalize::resolve_divisor_over` gives each run its share of the level-scan budget; one run behaves as before.
- Saving copies into one `data` chunk; `kiwi` chunks are dropped. One read spans as many runs as fit the 4 MiB buffer, and the samples go out in one write.
- A WAVE layout the native reader declines (24-bit) is read by the decoder, which sees only the first `data` chunk, so the writer copies only that run too; #192 brings 24-bit to the native reader and its runs.

## Implementation steps

- [x] Walk later `data` chunks in `riff`.
- [x] Read runs in `MmapSource`; level scan over runs.
- [x] Copy runs when saving.
- [x] Tests: walk, truncated last chunk, chunks past the RIFF end, reading and seeking across chunks, auto levels, save.
- [x] AGENTS.md note on the WAVE reader.
- [x] Review round 1 (`gpt-6.1-sol` high): accepted the 24-bit length mismatch, the level-scan budget rounding to zero values and the per-run system calls on save. The missing re-check of source stamps after copying exists on `main` and went to #206.
- [x] Review round 2: clean, and the reviewer agreed that #206 belongs outside this change.
- [x] Complete validation and review.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test -p argand-io --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`
- [x] The eight corpus recordings open at full length in `aspec` (22052: 2m26.146 at the stated 11 999 Hz).
