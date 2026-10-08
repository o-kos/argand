# Issue #193: Save the edited capture over the open file

Resolves #193.

Class A (data safety of the open file, file handles and worker lifetimes across threads, the public writer API). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

File → Save (Ctrl+S) writes the edited capture over the file it was opened from. Everything is written to a temporary file first, while the original is still being read; then every reader of the original lets it go, the original is checked to be unchanged since it was opened, the temporary file replaces it, and the window opens it again where it looked.

## Context

- `argand_io::write::save` writes a temporary file and renames it over the target at once, and refuses any source or protected file as the target (#186, #198).
- The open file is read by the analysis worker (`analysis.rs`, a thread that exits once its `Analyst` is dropped), by the minimap task (`minimap.rs`, a thread that exits once its receiver is dropped) and by the envelope tasks of pasted files. Each holds an `Mmap` or a decoder handle until its thread ends. Windows refuses to replace a file with a mapped section, so replacing the file needs every one of them gone first.
- The clipboard (#196) refers to source files by stamp; after the original is replaced, references to it would read other bytes.
- `Editing::save_request` builds segments of the current version; `SaveOf` ties a finished save to its document and version (#198).

## Decisions

Agreed with the owner:

1. **Format** stays the file's own: WAVE as Save as writes it, FLAC re-encoded, a headerless capture stays headerless, with the bytes before its samples (the `--offset` preamble) copied unchanged.
2. **FLAC the encoder cannot state** (above 96 kHz): Save is disabled; Ctrl+S says why in the status bar and points to Save as, since a stock menu row carries no hint.
3. **Clipboard**: before the file is replaced, copied ranges of it are moved to where those samples sit in the saved file; if any of them is not there any more, the clipboard is cleared with a notice.

Derived here:

- **Command**: File → Save, Ctrl+S (Cmd+S on macOS), enabled only with unsaved edits, a non-empty capture whose files are checked, and no save running. The unsaved-edits question offers Save, Discard and Cancel; Save is Save as when Save is disabled.
- **Writer**: `write::stage` writes the temporary file and returns a `Staged` that is committed later or dropped (removing it). `SaveRequest::replacing` names the file being replaced with its stamp; it may then be the target, and `Staged::commit` checks, just before the rename, that the file at the target still has that stamp. Every other target rule stays. A headerless output (`Output::Headerless { preamble }`) writes the preamble bytes and the samples with no header.
- **Releasing the file**: the analysis worker, the minimap task and the envelope tasks each hold a lease until their thread ends; `release::Lease` and `Released` (a channel whose senders are the leases) let the window wait until all have ended. Saving closes the document once the temporary file is complete (the status bar says the file is being replaced), waits for the leases, commits on a background thread, and opens the file again with the view and selection kept (`Reopening`).
- **When the commit fails** (another program holds the file, it changed on disk): the original is untouched, the written edits are kept beside it as `<stem>.unsaved-<n>.<ext>` so no work is lost, the notice says where, and the window opens the original again with its edit history restored when the file is still the one the edits refer to (changed after review).
- **While the file is replaced** nothing opens and the window does not close, so no other document or reader can appear between letting the file go and replacing it (added after review).
- **History** starts empty on the saved file, as after Save as.

## Review

Reviewer `gpt-6.1-sol` high.

- Round 1 (7 majors, 2 minors, a nit): edits were lost when the file changed after staging, the window could close or another file open while the file was let go of (with a clipboard paste adding a reader the wait did not cover, and a waiting action applied to the wrong document), the replacement mode was recomputed at commit so a hard-linked target could be overwritten unchecked, the moved clipboard had no stamp, cancelling during the wait was ignored, the restored minimap used the edited length, and the restored selection was overridden. All fixed. Declined: the comment nit on a module doc comment.

- Round 2 (4 P1, 1 P2): a failed keep removed the written file, two processes could take one kept name, the replacement mode was still recomputed after writing, the clipboard stamp was taken by path after the rename, and unfinished envelopes of pasted files were not restarted after a restore. All fixed. The comment nit was withdrawn.

## Rejected alternatives

- Writing in place: a failure would leave the original half written.
- Replacing the file while it is mapped: works on Unix, fails on Windows, and leaves readers on bytes that no longer exist.
- Keeping the document open and swapping its source underneath: every cache and worker would have to be told; reopening reuses the path that already works.

## Implementation steps

- [x] `argand-io::write`: `stage`, `Staged::commit`, `SaveRequest::replacing`, headerless output with preamble; tests including a target changed before commit and a failed commit.
- [x] `release::Lease` and `Released`; leases in the analysis worker, the minimap and the envelope tasks; tests that a released wait ends only after each thread does.
- [x] `Editing`: the clipboard moved onto the saved file's positions, or cleared; tests.
- [x] Shell: Save command, availability and reason, the save sequence (stage, close, wait, commit, reopen), failure recovery with the history restored, the question's Save button.
- [x] Headless tests for availability, the sequence on success and on failure.
- [x] Update `AGENTS.md` and `CHANGELOG.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The owner checks the release binary: Ctrl+S on WAVE, headerless and FLAC captures, the clipboard after a save, a save refused while another program holds the file.
