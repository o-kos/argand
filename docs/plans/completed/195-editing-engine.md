# Issues #195 and #196: The editing engine, delete, and copy, cut and paste

Resolves #195 and #196.

Class A (a new public crate, analysis and minimap caches, data safety of saved edits, an expected diff well above 400 lines across more than five code files). Implementer: Claude in the session. Reviewer: `gpt-6.1-sol` high, agreed with the owner.

## Overview

Step 3 of phase 5 in `IMPLEMENTATION_PLAN.md`. A new crate, `argand-edit`, keeps an edited capture as a piece table over the files it came from, with undo and redo. The application gains delete, cut, copy and paste of time ranges, an Edit menu, a context menu on the spectrogram, unsaved-change tracking, and Save as of the edited capture. Saving over the open file (Ctrl+S) is #193, a separate Pull Request.

## Context

- `analysis.rs`: one worker per document opens the file with `argand_io::open` and, per request, computes an `Overview` for a sample range through its own `SampleSource`. Range changes are new analysis generations; the worker is cancellable between batches.
- `minimap.rs`: an independent task reopens the file (`argand_io::reopen`) and builds a full-capture `WaveformEnvelope` of at most 65536 cells, preview first, then a sequential scan.
- `argand_io::write::save` writes one span of one source, byte for byte for linear WAVE and headerless layouts, decoded and re-encoded for FLAC, checked against a `SourceStamp`.
- `Shell` holds the document (`OpenFile`, `state.document`), the time `View { start, len }`, the `selection`, and resets view and selection on every file opening. `meta.len_samples` is the capture length everywhere.
- Context menus are gpui-component's `ContextMenuExt::context_menu` on an element, as the time ruler does (`navigation_ui::time_context_menu`); the menu cannot be opened in a headless test (#144).
- GPUI 0.3.6 offers `Window::prompt(level, message, detail, answers)` for a native question and `Window::on_window_should_close`, whose callback answers synchronously, so a close that needs a question is refused and the window closed after the answer.

## Decisions

Agreed with the owner:

1. **Scope**: the engine, delete (#195), copy, cut and paste (#196), Save as of the edited capture. Ctrl+S is #193.
2. **No cursor.** There is no playback here, so no insertion cursor. A right click on the spectrogram opens a context menu: Cut, Copy, Paste here (inserting at the clicked sample, marked by a line while the menu is open), Replace selection, Delete. Ctrl+V replaces the selection and is disabled without one.
3. **Clipboard**: Argand's own, never the system's. It holds a reference (file, hints, stamp, ranges), not samples, so copying an hour is immediate, and it outlives opening another file. Pasting into another capture needs the same sample rate and sample type (I/Q or real included); otherwise it is refused with the reason, conversion being #189. A source that changed on disk since it was copied is refused when the capture is saved, by the writer's stamp check; pasting does not read metadata on the window's thread.
4. **Keys and menu**: an Edit menu between File and View. Undo Ctrl+Z, Redo Ctrl+Shift+Z (Ctrl+Y as well on Windows), Cut Ctrl+X, Copy Ctrl+C, Paste Ctrl+V, Delete the Delete key; Cmd on macOS.
5. **Unsaved edits**: a `•` before the name in both titles. Opening another file or closing the window first asks natively: Save as…, Discard, Cancel.
6. **Save as of the whole edited capture** turns the window to the saved file: no edits, an empty undo history, its name in the titles; the view and the selection stay. Save selection as leaves the document as it was.

Derived here:

- **`argand-edit`** depends on `argand-core` only. `SourceId` indexes a source table the application owns. `Piece { source, start, len }`; `Capture` is an immutable piece list with its length and prefix sums, built by `delete`, `insert`, `replace`, and read by `copy` (a `Clip` of pieces) and `segments(span)` (the source ranges a span maps to). Adjacent pieces of the same source merge. `History` keeps capture versions with the selection each left, undo and redo, and the version last saved, which decides the unsaved state. Versions share nothing mutable, so a worker holding an old one is never disturbed.
- **`EditedSource`** implements `SampleSource` over a `Capture` and one opened source per `SourceId`, seeking by binary search over the prefix sums and reading across piece boundaries. Each consumer opens its own (the analysis worker, the writer), so no cursor is shared.
- **Analysis**: a request carries the capture version; a new version is a new generation, re-analysing the visible range through the existing cancellable worker. Picture and backdrop are dropped on an edit until the new version's picture lands: unlike navigation, an edit moves samples under the old picture's coordinates, so selecting on it would select other samples (changed after review).
- **Minimap**: envelopes stay per source and are never rescanned for an edit. The minimap of the edited capture is composed by mapping each display cell through the capture onto the source envelopes' cells, conservatively at their resolution, so an edit shows at once. A source pasted from another file gets its envelope built once in the background; until then its stretch is drawn empty.
- **Paste compatibility and saving**: the writer takes segments of possibly several sources. Byte copy needs the same storage layout, not only the same `SampleType` (a 24-bit WAVE reports `I32` as a raw 32-bit file does), so compatibility is checked on the storage layout (`argand_io::write::Storage`: linear block layout, or FLAC bit depth) and the sample rate. The output format is the original's; the reference frequency is the document's.
- **Selection after an edit**: none after delete and cut; the inserted range after paste; undo and redo restore the selection the version had.
- **Levels**: each source keeps its own normalization when displayed; saved files keep stored values, as in #186.

## Rejected alternatives

- An insertion cursor: implies playback, which Argand does not have.
- The system clipboard: large ranges would have to be materialized, and no other application would read them.
- Rescanning the minimap after an edit: minutes on multi-hour captures, against "edits are immediate".
- Storing edits as copied samples: memory grows with every paste.

## Implementation steps

- [x] `argand-edit`: `Capture`, `Piece`, `Clip`, `History`, with property tests (random edit sequences against a plain vector model).
- [x] `EditedSource` over a capture and opened sources, tested against the same model through `SampleSource`.
- [x] `argand-io::write`: segments of several sources, `Storage` and compatibility, FLAC and WAVE outputs over several segments, tests.
- [x] Analysis worker and minimap: capture versions, per-source envelopes and composition, the backdrop on an edit.
- [x] Shell: source table, history, delete, cut, copy, paste, undo, redo, selection after each, unsaved state and titles.
- [x] Edit menu and the spectrogram context menu with the paste line; key bindings.
- [x] Questions before opening another file and closing the window; Save as of an edited capture turning the window to the saved file.
- [x] Headless tests for commands, availability, history and unsaved state; the context menu cannot open headless (#144), so it is a native check.
- [x] Update `AGENTS.md`, `IMPLEMENTATION_PLAN.md` and `CHANGELOG.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The owner checks the release binary: delete, cut, copy and paste on a multi-hour capture, across two files, undo and redo, the questions on unsaved edits, Save as of an edited capture.
