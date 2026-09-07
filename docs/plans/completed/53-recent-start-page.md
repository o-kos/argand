# Issue #53: Existing recent files on the start page

Resolves [#53](https://github.com/o-kos/argand/issues/53).

## Overview

Replace the passive empty-state instruction with a file-chooser link and verified
recent-file links. Number the first nine visible entries and bind Alt+1 through
Alt+9 to those entries while the start page is visible.

## Context

The session retains ten recent captures and their opening hints. `shell.rs`
already owns the platform chooser and background file opening. This branch builds
on Draft PR #50; its PR targets that branch until #50 is accepted and merged.
#45 retains Ctrl/Cmd+O and #33 retains automatic session restoration.

## Decisions

- Probe all candidate paths off the UI thread; network mounts cannot be reliably
  identified from path syntax across platforms.
- Probe candidates independently, with at most the session's ten entries, so a
  hung network stat cannot hold back another file. Do not join blocked workers
  during opening or shutdown; late results are ignored after leaving the page.
- Publish verified regular files in saved recent order. Derive both the labels
  and shortcuts from the same filtered snapshot; unavailable paths stay in history.
- Preserve original paths and opening hints. Keep the chooser link visible even
  while candidates are being checked and when none exist.

## Rejected alternatives

- Synchronous metadata calls can block the window on a network mount.
- Serial background probing lets one blocked path hide all later entries.
- Removing failed checks from history loses temporarily disconnected captures.

## Implementation steps

- [x] Add a toolkit-neutral model and independent background existence checks.
- [x] Render start-page links and route Alt+1 through Alt+9 to visible entries.
- [x] Cover filtering, blocked checks, ordering and saved hints with focused tests.
- [x] Update documentation and changelog.
- [x] Complete validation and move the plan to `docs/plans/completed/`.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after the local gate
- [x] Fresh-release checks: existing/missing paths, duplicate names, more than
  nine entries, click and Alt shortcuts, explicit file argument, chooser link,
  both themes and minimum window size.
- [x] Required external review returns no substantive findings.

## Validation evidence

- Formatting, strict Clippy and all 374 local tests pass; release was rebuilt
  after the gate. Five new tests cover actual file/directory/missing-path
  filtering, preservation of raw hints and history, completion order, bounded
  history and shortcut mapping. A gated probe reproduces a blocked first path
  while the second path completes independently.
- GPU-backed Wayland checks exercised all nine Alt shortcuts against distinct
  saved entries, including identical filenames in different directories and a
  long filename. The selected raw capture opens with its saved format/rate.
  Alt shortcuts no longer switch files after leaving the start page.
- Missing paths and directories are absent and the remaining links are numbered
  contiguously. An explicit CLI capture bypasses the page; an empty or wholly
  unavailable history leaves the chooser link usable.
- Mouse checks open the first entry and the unnumbered tenth entry after
  scrolling. In a 640x400 window the status bar and chooser stay visible;
  long labels are ellipsized and their path tooltips wrap inside the viewport.
- The chooser link opens the real GTK file dialog with and without recent files.
  Checks use an isolated compositor and portal session. Both themes are covered, and the minimum-size page and Alt+1 also pass at 200% DPI.
- Initial UI checks exposed missing startup focus and a list growing beyond its
  flex container; these were fixed before final validation. External review then
  found lost focus after dismissing File. The finding was accepted, reproduced
  via File/Escape/Alt+9, and fixed with the popup action context. Alt shortcuts
  now work after Escape, outside clicks, Tab traversal and cancellation of the
  chooser opened by either menu or start-page link. No findings were declined;
  the final focused review returned no substantive findings.
- Runtime validation is on Linux Wayland. Windows/macOS runtime behaviour and
  real offline network mounts are not exercised here; the blocked-probe test
  verifies independence without changing the host's network mounts.

## Post-completion

After #50 merges, retarget this PR to main and reconcile its base. Keep Draft
through owner feedback; request full CI only for final acceptance. Squash-merge
and clean the branch after acceptance and required checks.
