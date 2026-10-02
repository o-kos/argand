# Issue #175: Session writes run synchronously on the UI thread

Resolves #175.

Class A (threads, data safety). Implementer: Claude in the session. Reviewer:
`gpt-6.1-sol` high, agreed with the owner.

## Overview

`session::Writer` opens the lock, reads, merges, writes and renames
`session.toml` on the UI thread, inside the shell's handlers. A stalled
filesystem stalls the window for as long as it does. The writes move to a
thread of their own; the shell hands it the latest session without waiting,
and the exit waits for the final write with a bound of its own.

## Context

- `crates/app/src/session.rs`: `Writer` throttles offers to one write per
  `Writer::INTERVAL`, merges under `session.toml.lock` (#43) and retries a held
  lock briefly in `flush`.
- `crates/app/src/main.rs` builds the `Writer`; `crates/app/src/shell.rs` keeps
  it in `Shell::writer`, offers in `Shell::save` and flushes in `Drop for Shell`.
- A throttled offer is written only by the next offer or the flush, so a run
  killed after an idle change loses it.

## Decisions

- `Writer` stays the synchronous core with its time-injected tests. It gains
  `due_at` and `tick`, so a throttled offer is written when its interval ends
  without waiting for another offer.
- A new `session::Saver` owns the thread. Offers go through a one-slot mailbox
  (mutex and condition variable) where the newest replaces the older, so a
  stalled thread holds one session, never a queue. The thread waits for an
  offer or the next due time, and never holds the mailbox lock during I/O.
- The shell keeps an `Option<Saver>` in place of the `Writer`. Dropping the
  `Saver` asks the thread for a final flush and waits for it at most
  `Saver::EXIT_WAIT` (1 s). A thread still stuck after that is left behind and
  ends with the process; the atomic rename keeps the file whole.
- A thread that cannot be spawned costs the session for that run with one
  warning, as a missing state directory already does.

## Rejected alternatives

- An unbounded channel of offers: a hung filesystem during a drag would queue
  every frame's session.
- Running the write on the GPUI background executor: the exit wait then
  depends on the executor still running while the application shuts down.
- Joining the thread at exit: unbounded on a stalled filesystem, which is
  the case this Issue exists for.

## Implementation steps

- [ ] `Writer::due_at` and `Writer::tick`, with tests.
- [ ] `Saver` with the mailbox, the thread loop and the bounded close, with
  tests for a write without a further offer, the final flush, and an offer and
  a close that do not wait on a stalled write.
- [ ] The shell and `main` use `Saver`; `Drop for Shell` goes.
- [ ] Update `AGENTS.md`, `CHANGELOG.md` and the `session.rs` documentation.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The release binary remembers a moved window and an opened file after a
  normal close.

## Post-completion

None.
