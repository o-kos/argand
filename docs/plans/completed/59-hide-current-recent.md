# Issue #59: Hide the currently loaded file from the Recent list

Resolves #59.

Complexity class **B**: implementer `gpt-6-astra` at high reasoning effort,
reviewer `gpt-5.6-sol` at high reasoning effort, per "Agent roles and model
selection" in `AGENTS.md`.

## Overview

The file on screen stays in the Recent list, where choosing it reopens what is
already open. Hide it while it is loaded, in the menu and on the start page, without
touching the saved history.

Out of scope: the order of the saved list, the availability probing, the opening
hints, and the native file chooser.

## Context

- `crates/app/src/recent.rs` owns the toolkit-neutral availability snapshot.
  `visible()` returns the entries whose probe confirmed a regular file, and
  `shortcut(index)` reads `visible()`, so both the numbered start-page links and
  Alt+1 through Alt+9 already come from one filtered order.
- `crates/app/src/shell.rs::recent_entries` (the menu) and the start page at
  `shell.rs:1190` both call `visible()`. Filtering inside `RecentFiles` therefore
  reaches every consumer, and `shortcut` cannot drift from the list that is drawn.
- `crates/app/src/shell.rs::remember_file` is the single place the history grows.
  It runs on `Effect::Opened`, which is after a successful open, and it already
  calls `RecentFiles::refresh`.
- `crates/app/src/session.rs::remember` stores `std::path::absolute(path)`, and
  deliberately not the canonical path -- a symlink is a name the person chose. A
  file opened as `argand dump.bin` is therefore remembered absolute while the
  `Origin` still holds the relative path it was given.
- `self.file` is only ever assigned, never cleared, so a loaded file is replaced
  rather than closed.

## Decisions

- **The current path lives in `RecentFiles`, not in each caller.** `visible()` then
  filters it out once and `shortcut()` follows automatically. Filtering at the two
  call sites instead would leave `shortcut` addressing the unfiltered order, so
  Alt+3 could open something other than the third row.
- **The stored path is normalised exactly as `session::remember` normalises it.**
  Comparing a raw `Origin::path` against history would fail to match whenever the
  file was opened by a relative path, and the entry would stay visible. Both sides
  must go through one normalisation, so it is extracted rather than repeated.
- **The current file is recorded where the history is updated**, in `remember_file`,
  which runs only after a successful open. `self.file` is assigned before the open
  is known to have succeeded, so keying off it would let a failed open hide an
  unrelated entry.
- **Saved history is untouched.** The filter applies to the snapshot `visible()`
  returns; `refresh` keeps receiving the full list and `session.recent` keeps every
  entry with its hints, so the file returns to its normal position on the next start
  and as soon as another file is opened.
- **Hiding one entry shortens the list to nine.** The current file is always the most
  recent, so the menu shows the nine behind it. Reading an eleventh entry from
  history to refill the tenth slot is not done: `RECENT_LIMIT` is what the session
  stores, and Alt+1 through Alt+9 addresses nine rows either way.

## Rejected alternatives

- **Dropping the file from `session.recent` while it is open.** It would need
  restoring on replacement and on shutdown, and a crash in between would lose the
  entry and its opening hints outright.
- **Filtering in `recent_entries` and at the start page.** Two places to keep in
  step, and `shortcut()` would still read the unfiltered order.
- **Comparing canonical paths.** It would resolve symlinks that `session::remember`
  intentionally preserves, so two different remembered names for one file would
  collapse, and it needs the file to still exist.

## Implementation steps

- [x] Extract the path normalisation `session::remember` performs so the same
      function can be applied to the currently loaded file.
- [x] Hold the current path in `RecentFiles` and exclude it from `visible()`.
- [x] Record it from `shell.rs::remember_file`, so it is set only after a
      successful open and replaced when another file is opened.
- [x] Cover in `crates/app/src/recent_tests.rs`: the loaded file is absent from
      `visible()` while the others keep their order and labels; opening another file
      brings the previous one back in its normal recent position; a file opened by a
      relative path is still matched; `entries` and the saved history are unchanged
      throughout; `shortcut` indices address the filtered list.
- [x] Update the recent-files paragraph of `AGENTS.md` to state that the loaded file
      is filtered from the shared snapshot.
- [x] ➕ Add an Unreleased changelog entry as required by `CONTRIBUTING.md`.
- [x] ➕ Clear the current path when `Shell::open` tears down the previous file,
      so pending or failed opens do not leave an unloaded file hidden.
- [x] ➕ Restore the `remember_file` doc comment to describe history promotion,
      hiding the loaded file and session persistence.
- [x] ➕ Cover clearing and replacing the current path without a window, including
      restored recent order, disambiguated labels and shortcut targets.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Owner validation on a real GPU session: open a file and confirm it is absent
      from File > Recent and from the start page; open a second file and confirm the
      first returns in its normal position; confirm Alt+1 opens the first row shown;
      restart and confirm the history is intact.

Automated validation passed on 2026-09-16 after the second-round fixes. The full
test suite passed 560 tests with no failures or ignored tests. Cargo reported a future-incompatibility notice
for the existing `proc-macro-error2 v2.0.1` dependency.

## Post-completion

- None.
