# Issue #43: Two running instances lose each other's session

Resolves #43.

Class A (cross-process concurrency, data safety). Implementer: Claude in the
session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

`session.toml` is read once at start-up and written whole, so two running
instances keep whatever the last one wrote. Since #28 that includes recent
entries and the only record of the hints that open a headerless capture.

Each write becomes a read-modify-write under an advisory cross-process lock:
the writer re-reads the file and merges its own changes into it. A single
instance behaves as before: writes stay atomic and advisory, and nothing about
the lock can stop the application starting or block the UI thread.

## Context

- `crates/app/src/session.rs`: `Session::load`, `Session::save`,
  `write_atomically` (staging file and rename), `Writer` (throttled offers,
  flush on close). The module documentation records the limitation.
- `crates/app/src/main.rs` creates the `Writer` only when the file is writable
  (not from a newer version) and seeds it with the loaded session.
- `crates/app/src/shell.rs` offers the whole session on window moves, file
  openings and setting changes, and flushes on close.
- Nothing removes recent entries except `Session::remember`'s move-to-head and
  truncation.
- The toolchain is Rust 1.97, so `std::fs::File::try_lock` is available without
  a new dependency.

## Decisions

- Three-way merge at write time. The base is the session this process last
  read or wrote (`Writer::stored`), mine is the pending offer, theirs is the
  file as read under the lock. A unit is taken from mine only when mine differs
  from the base, otherwise from the file.
- Merge units: `geometry` and `window_state` together (a rectangle belongs to
  its state), then `orientation`, `show_grid`, `show_scale_ui`, `time_ruler`,
  `theme` and `analysis_settings` each on its own.
- Recent list: the entries this process put at the head since the base are the
  shortest prefix of mine whose removal from the base, truncated, gives the
  rest of mine. That prefix goes at the head of the file's list, which loses
  those paths elsewhere, and the result is truncated to `RECENT_LIMIT`. When no
  prefix explains mine, the whole of mine counts as the prefix, which is a
  union rather than a loss.
- Lock: `session.toml.lock` beside the session, taken with `File::try_lock`.
  It covers read, merge, staging write and rename. A held lock leaves the offer
  pending for the next interval; the final flush retries a few times over a
  bounded wait of about 200 ms. A filesystem without locks (`Unsupported`) and
  an unopenable lock file fall back to the merged write without a lock, with
  one log line.
- A file that is missing, unreadable or corrupt when re-read merges as if it
  held the base, so this process's own view is written. A file from a newer
  version closes the writer for the rest of the run with one warning, the same
  rule `load` applies at start-up.
- The shell keeps its own in-memory session. A running instance does not show
  another's recent files until it restarts; the merge only decides what reaches
  the disk.

## Rejected alternatives

- Re-read and merge without a lock: leaves a window between read and rename in
  which two writers still lose one update.
- A separate append-only recent file: splits the format and its version gate,
  and still leaves geometry and settings as last-writer-wins.
- A blocking lock on the UI thread: a lock held by a hung process, or on a
  stalled network mount, would freeze the window.
- A lock crate (`fs4`, `fd-lock`): `std` already provides the call.

## Implementation steps

- [x] Merge function over `(base, mine, theirs)` with unit tests for every
  unit, the recent prefix rule, the fallback union and truncation.
- [x] Locked read-modify-write in `Writer::write`, with lock-busy, unsupported
  lock and newer-version outcomes, and tests with two writers on one file.
- [x] Bounded retry on the final flush.
- [x] Replace the limitation in the `session.rs` module documentation; update
  `AGENTS.md` and `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Two release instances open different files and both exit: the recent
  list holds both, with their hints.

## Post-completion

None.
