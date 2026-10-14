# Issue #200: Notice when an open capture changes on disk

Resolves #200.

Class A (data safety on every read path; public `argand-core` and `argand-io` API). Implementer: Claude in the session. Reviewer: to be agreed with the owner before the first round.

## Overview

Several read paths open a capture's file again by path and check at most its length:

- `DecodedSource::seek` builds a fresh FLAC parser from the path on every seek;
- `DecodedSource::open_file` counts an unknown length, and `apply_levels` measures an automatic level, through a second open by path;
- `argand_io::reopen`, which the minimap, the analysis worker (for pasted sources) and the envelopes of pasted files use, compares only `len_samples`.

A file replaced by another of the same length is then drawn while the window still describes the one it opened. After this change every re-open either reads through the handle the file was first opened with, or checks the `SourceStamp` taken from that handle, and fails with one recognisable error. The window also checks the stamps of the files a capture reads when it is activated. Either way the status bar says the capture changed on disk and offers to open it again.

Out of scope: an overwrite in place of a memory-mapped file while it is read, without a re-open. The activation check notices it afterwards, but reads in flight can see new bytes; that is the hazard every mmap reader accepts (see `MmapSource::map`). An overwrite that keeps the length and the modification time is the limit of `SourceStamp` itself (#206).

## Context

- `SourceStamp` (length, modification time, Unix device and inode) lives in `argand_io::write`. `open_stamped` returns it only when the header and the samples came from one file. `FileInfo::stamp` carries it to the document, and `editing::Source::stamp` keeps it per `SourceId`.
- Saving already refuses a changed source: `write::save` checks each stamp when it opens a source and again after the last sample (#198, #206). `extraction.rs` checks `reopen_stamped`'s stamp by hand.
- `Shell` refreshes recent-file availability on window activation (`observe_window_activation`) and does every metadata check off the UI thread. This check follows the same pattern.
- `Shell::ask_unsaved` asks Save, Discard or Cancel. Save cannot succeed once a source has changed, because the writer refuses it.

## Decisions

- `argand_core::SourceError::Changed` ("the file changed on disk since it was opened") is the one error for a source whose file is no longer the one opened. `argand_io::IoError::Source` carries it unchanged, so the app recognises it by downcasting the `anyhow` chain, not by matching text.
- The decoder keeps the opened `File` in an `Arc` and reads it through a `MediaSource` with its own position (`read_at` on Unix, `seek_read` on Windows). Every fresh parser (a FLAC seek, the length count, the automatic level scan) reads that handle, never the path. A renamed or replaced file therefore keeps being read as the file that was opened. Before a fresh parser starts, the handle is stamped again and compared with the opening stamp, so an overwrite in place is `Changed` rather than a mix.
- `argand_io::reopen` takes the expected `SourceStamp` and fails with `Changed` when the new handle's stamp differs or is unknown. The unchecked form goes away, so no caller can skip the check. `reopen_stamped` folds into it, and `extraction.rs` drops its own comparison.
- Every caller has the stamp it needs. The minimap uses the document's (`FileInfo::stamp`) or the editing source's. The analysis worker's edit state carries each source's stamp beside its meta and hints. Pasted envelopes take it from `editing::Source`. A source without a stamp cannot be reopened, which is `Changed` too: its stamp is absent only when its file changed while it was opened.
- On activation, a background task stamps by path every file the shown capture reads, at most once per pending check. The result is applied only when the document and the stamps it was started with are still current. A save that rebinds the file (#193) replaces the stamps first, so the application's own write is never reported. No check runs while `replacement_active` holds.
- Either signal sets `Document::is_changed`, naming the file found changed (the opened one or a pasted one) when it is known, so it goes with the document. While it is set:
  - the status bar shows `<name> changed on disk` with a standard `Button` labelled Reopen, in place of the analysis status. The shown picture, minimap, view and selection stay;
  - the document asks for no new analysis; navigation keeps mapping the held pictures, as it does while a re-analysis is pending;
  - edits are refused, as while the file is let go of (`Shell::editing_mut`).
- Reopen opens the capture's origin again, which starts a fresh document. With unsaved edits it first asks through the same dialog machinery as `ask_unsaved`, offering only Discard and Cancel, because Save would be refused.

## Rejected alternatives

- A file-system watcher: it needs a platform dependency and a thread per file, it misses network mounts, and the activation check catches the same edits whenever the user comes back to the window.
- Re-opening by path with a stamp check inside the decoder: a file rotated away from its name would fail although the original handle still reads it. Positioned reads on the original handle keep reading the file that was opened, with no extra open.
- `File::try_clone` for the decoder's parsers: a cloned handle shares the file position, and a counting or scanning parser would move the position under the main parser's buffered stream.
- A modal dialog as soon as the change is noticed (the owner chose the status bar).

## Implementation steps

- [x] `SourceError::Changed` in `argand-core`.
- [x] A positioned-read `MediaSource` over `Arc<File>` in `argand-io`. The decoder's seek, count and level scan use it, and each fresh parser re-checks the handle's stamp.
- [x] `argand_io::reopen(meta, hints, stamp)`, failing with `Changed`. Remove the unchecked form and `reopen_stamped`, and migrate the minimap, the analysis worker, the pasted envelopes and extraction.
- [x] I/O tests (each fails without the decoder change): a FLAC seek after the path is replaced by a same-length file still reads the original; a FLAC seek after an overwrite in place is `Changed`; `reopen` of a same-length replacement is `Changed`; `reopen` with the right stamp succeeds.
- [x] `Document::changed`: recognise `Changed` from analysis, minimap and envelope failures; stop analysis requests and refuse edits while it is set.
- [x] The activation check: background stamping of the capture's files, applied only to the current document and stamps.
- [x] Status bar notice with Reopen; Reopen with Discard/Cancel when edits are unsaved.
- [x] Tests: `disk_change` recognises `Changed` through an error chain and finds a rewritten or removed file. Headless GPUI tests: a stale check (another document, other stamps) is ignored; the activation check notices a rewrite, refuses edits, and Reopen asks first, Cancel keeps the document and Discard opens the file again; a minimap refused by a replaced file marks the capture.
- [x] AGENTS.md and CHANGELOG.md.
- [x] Review round 1 (`gpt-6.1-sol` high): accepted all six findings. `reopen` checks the stamp by path before parsing, so a removed, emptied or unparsable replacement is `Changed`. No disk check starts or applies while a saved file is rebound, and `swap_readers` drops a pending one, so the application's own Save is never reported. Analysis deliveries and minimap results are rejected once the capture changed, so a pass already running cannot replace the held picture, and a refused minimap keeps the one shown. A pasted file's late envelope failure counts only while the current version still reads that file under that stamp. The `SharedFile` documentation no longer claims `seek_read` leaves the handle's position alone. Each fix has a test; the rebinding one fails without its guard.
- [x] Review round 2 (same reviewer): accepted all six findings. `accept` rejected every delivery once changed, which stranded a Save as rebinding to its saved file; it now rejects only progress and pictures (`holds_picture`), and the rebinding clears the change. A late storage description from an earlier opening restored the old stamp after a save, then reported the save as a change; it now applies only to the opening with that stamp. The opened file's minimap failure counts only while the capture still reads that file. `reopen` re-checks the stamp when parsing fails, closing the race after its pre-check. A backdrop restyle in flight no longer lands over a held picture. An unnamed `Changed` from a reader is named by the minimap and, for a capture reading only the opened file, by the analysis worker; otherwise the notice names no file instead of the wrong one. Tests cover the rebinding (fails when every delivery is rejected), the late description, and naming.
- [x] Review round 3 (same reviewer): fixes 1, 2, 4, 5 and 6 of round 2 confirmed. One finding accepted: a late minimap failure of an opened file the capture no longer reads still cleared the composed minimap shown; it is now ignored entirely. No further findings. This is the third and last round; the fix was not reviewed again.
- [x] Windows CI fixes after round 3, reviewed by the same reviewer: the GUI tests no longer truncate a mapped file, and the same-length replacement in `argand-io` gets a later time on Windows, where stamps carry no file ID until #199. One finding accepted: removing the file by name gave it a new inode on Unix, so the GUI test no longer proved a rewrite in place was noticed by its time. It now rewrites the file in place without truncation and fails when the stamp ignores the time.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Manual, release binary: open a FLAC capture, replace it by rename with another of the same length, and pan. The original keeps being drawn. Switch away from the window and back: the status bar offers Reopen, and Reopen shows the new file. The same with a WAVE capture, and with unsaved edits, where only Discard and Cancel are offered.

The owner accepted the manual check as done on 2026-10-14. Individual results were not separately reported.
