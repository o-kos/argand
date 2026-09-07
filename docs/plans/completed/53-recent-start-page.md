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
#45 is completed by the owner-requested chooser shortcut refinement below;
#33 retains automatic session restoration.

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

## Owner feedback: recent rows and shortcut hints

- [x] Replace underlined links with compact ghost rows, a separate muted number
  column and hover background. Keep file opening and numbering unchanged.
- [x] Fit the recent list to its contents until it needs to scroll; put `or`
  and the chooser immediately below it instead of pinning the chooser at the bottom.
- [x] Add a shared tooltip layout with the action's registered keybinding in a
  distinct theme colour at the right edge, without a punctuation separator.
- [x] Give the chooser a working Ctrl+O / macOS Cmd+O action and tooltip; route
  File's Open item through the same action and show its accelerator. This owner
  request also completes the existing [#45](https://github.com/o-kos/argand/issues/45).
- [x] Validate layout, hover, shortcuts, chooser and menu focus in a fresh release;
  complete the local gate and focused external review.

Status metadata tooltip and compact duration changes are explicitly deferred
until the owner asks to proceed with them.

### Feedback validation evidence

- Formatting, strict Clippy and all 374 local tests pass on the refined code;
  the release binary was rebuilt after these checks.
- Fresh-release GPU-backed Wayland checks cover both themes with two, ten and
  zero recent files at 640x400, hover backgrounds, nearby chooser placement,
  right-aligned coloured shortcuts and long-path wrapping.
- Ctrl+O opens the real GTK chooser with empty history, recent files, an open
  document and an open File popup. The start-page button and File item also
  open it. Cancelling returns focus so Alt+2 opens its matching saved entry.
- All nine Alt shortcuts and mouse opening of the first and scrolled tenth
  entries pass again. File displays the registered Open accelerator.
- The initial focused feedback review and the follow-up covering global
  action dispatch, deferred window update, popup dismissal and shortcut lookup
  both returned no substantive findings. No review findings were declined.

## Owner feedback: content-sized highlights and hints

- [x] Center `or` above the chooser within the start-page block.
- [x] Size each recent row and the chooser to its text, keeping long rows
  constrained and ellipsized within the list width.
- [x] Size shortcut hints to their content, with a viewport-aware maximum;
  preserve the coloured shortcut on the right and wrapping for long paths.
- [x] Run the local gate, rebuild release, inspect both themes and narrow/long
  content, and complete the required focused external review.

Status metadata hint and duration redesign remains deferred.

### Content sizing validation evidence

- Formatting, strict Clippy and all 374 local tests pass; the release was rebuilt
  after the gate.
- GPU-backed Wayland inspection at 640x400 in both themes covers short rows,
  long ellipsized rows, content-sized short hints and wrapped long-path hints.
  The centered separator and chooser stay directly below the list.
- Clicking beyond a short row's right edge does not open a file; clicking its
  label does. The chooser works with empty history, and the tenth entry still
  opens by mouse after scrolling.
- Focused external review returned no substantive findings; none were declined.

## Owner feedback: align the chooser with the filename column

- [x] Align `or` with the left edge of `Recent files` and the chooser text with
  the filename column, retaining content-sized hover backgrounds and hints.
- [x] Keep the empty-history chooser centered; validate both states in a fresh
  release after the local gate and complete focused external review.

Validation: formatting, strict Clippy and all 374 tests pass; release rebuilt
following the gate. GPU-backed Wayland checks in both themes confirm the heading
and separator share a left edge, and the chooser text shares the filename left
edge. Two-entry, scrolling ten-entry and empty-history layouts retain compact
highlights and hints; the empty-history chooser still opens the platform dialog.
Focused external review returned no substantive findings; none were declined.
