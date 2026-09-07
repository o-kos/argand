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

- [ ] Add a toolkit-neutral model and independent background existence checks.
- [ ] Render start-page links and route Alt+1 through Alt+9 to visible entries.
- [ ] Cover filtering, blocked checks, ordering and saved hints with focused tests.
- [ ] Update documentation and changelog.
- [ ] Complete validation and move the plan to `docs/plans/completed/`.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after the local gate
- [ ] Fresh-release checks: existing/missing paths, duplicate names, more than
  nine entries, click and Alt shortcuts, explicit file argument, chooser link,
  both themes and minimum window size.
- [ ] Required external review returns no substantive findings.

## Post-completion

After #50 merges, retarget this PR to main and reconcile its base. Keep Draft
through owner feedback; request full CI only for final acceptance. Squash-merge
and clean the branch after acceptance and required checks.
