# Issues #206, #207: Re-check source stamps after a save reads them

Resolves #206.
Resolves #207.

Class A (data safety in the save path). Implementer: Claude in the session. Reviewer: `gpt-6-sol` medium, agreed with the owner.

## Overview

`write::save` compared each source's `SourceStamp` only when it opened the source. A file overwritten by another process while the copy ran was saved as a mix of old and new samples. Separately, `a_rotated_source_is_neither_read_nor_saved_over` was flaky on Windows: without a file identity (#199) a renamed and replaced file is told apart only by its modification time, and the test wrote both files within one tick of the file clock.

## Decisions

- `Opened` keeps a clone of the handle the samples are read through (`File::try_clone`) beside the stamp it was opened with. The decoder owns the original handle for FLAC, so the clone is the only way to stamp that same handle again.
- After every sample is written and the temporary file is synced, `stage_with_limit` stamps each source again from its handle; a mismatch is `SourceChanged`, and the dropped `Partial` removes the temporary file. Staging is shared by Save, Save as and Save over the open file, so every path gets the check, a pasted second source included.
- The check compares with the stamp taken at opening, which equals `SourceFile::stamp` whenever one was given, so a source without a known stamp is checked too.
- The tests set the changed file's modification time explicitly (`File::set_modified`), so they test the stamp comparison rather than the clock's resolution.

## Rejected alternatives

- Re-stamping by path: it would miss a file rotated away from its name while it was read, which the handle still reads.
- Checking after every block: the copy is already refused at its end, and a per-block system call adds cost without changing the outcome.

## Implementation steps

- [x] Re-check every source's stamp from its own handle after the last sample is read.
- [x] Tests: a source changed after part of it was copied; the same for the second source of a joined request.
- [x] Give the rotated replacement in the #207 test a distinct modification time.
- [x] AGENTS.md and CHANGELOG.md.
- [x] Move the finished plans of #31, #112 and #156 to `docs/plans/completed/`.
- [x] Review round 1 (`gpt-6-sol` medium): declined that a same-length overwrite within one modification-time tick, or one that restores the old time, still passes; that is the limit of `SourceStamp` itself, which #206 asks to compare, and closing it needs content hashing or locking, not a re-check. Accepted narrowing the CHANGELOG entry to what the stamp detects.
- [x] Review round 2: clean; the reviewer found no cheap, reliable mechanism within this scope that would beat the stamp limit.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] The two new tests fail without the re-check.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`
