# Editing and I/O

Capture history, source identity and safe file publication. Read this before changing editing, readers, saving, replacement or changed-on-disk handling.

## Decoder boundary (#230)

Decoder-backed WAVE/PCM and FLAC use Symphonia 0.6.1, selecting the first non-null
**audio** track. Timing is read from the track, with WAVE/FLAC timestamps in sample
units; seeks and strict packet continuity use checked signed timestamps. Decoded
frames are copied interleaved directly into the reusable pending `f32` buffer.
Only explicit `Ok(None)` at or beyond the known stream length is normal packet EOF.
An unexpected EOF or unsupported decoder reset remains an error, including a truncated known-length FLAC with no complete first frame. Unknown-length
counting has no declared-length constraint. The checked decoded packet end is reset
to the actual landing timestamp on seek; seeking exactly to the endpoint establishes
a logical EOF without asking the demuxer to find a frame there. Strict saves still
refuse damaged packets,
missing timestamp spans and short input, and publish only complete output. The
native WAVE reader remains responsible for RF64/BW64; the decoder's WAVE reader
continues to accept RIFF only.

## Saving (#186)

The Save menu and app-level shortcut use `Shell::can_save_edits` for availability.
An unavailable shortcut is inert; internal save continuations retain their detailed
rejection and source/publication guards.

`argand_io::write::save` writes a capture, or a `SampleSpan` of it, to a new file in the
source's format. WAVE and headerless sources are copied byte for byte through one `File`
handle that also reads the header, never through the mapping, under the source's own
`fmt ` body with the effective rate; a headerless capture, or a sample type hint on a
layout the native reader handles, gets one synthesized from the effective `SampleType`.
Any linear PCM or float layout is copied this way, 24-bit included. The data length (the
smaller of declared and present) must give `SignalMeta::len_samples`, otherwise the
source changed since it was opened and the save is refused. FLAC is decoded by one strict
decoder over the handle whose stamp was checked, that resolves its own length (compared with the opened capture), seeks by exact
timestamp and fails on a damaged packet or a gap in packet timestamps, since Symphonia
drops frames with a bad CRC by itself. Values are checked to land on integers and encoded
again with `flacenc` 0.5 (default features off) at the source bit depth, up to 24 bits and
under 2^36 samples; STREAMINFO is rewritten at the end. That encoder states only rates up
to 96 kHz that frame headers can carry, so the header gets such a rate within 1 Hz of the
exact one; a FLAC source with none (192 kHz, an SDR rate) is written as WAVE of the same bit
depth instead, as the owner chose, and `write::writes_as_wave` gives it a `.wav` name;
an odd bit depth there is refused. A seek that lands after its sample fails, and a FLAC
stream without a length is counted by the strict decoder. A WAVE output that needs RF64
in a layout the native reader declines, including reduced-valid-bit PCM24, is refused
because Argand could not reopen it. Full-valid packed PCM24 is native and can use
RF64. A supported WAVE output whose RIFF size would exceed `u32` is RF64. Stored
values are written, never normalized ones. A `SaveRequest` names
`SourceFile`s and `Segment`s of them, written in order; every source must share one
`Storage` (`write::storage`: linear `fmt ` layout, or FLAC depth) and the request's sample
rate. An immutable `write::Format`, captured by the existing checked description reader,
retains the bounded linear `fmt ` body or FLAC storage independently of sample files (#214).
Document saves pass it explicitly; pure convenience constructors derive it from their first
checked non-computed source. `SaveRequest::meta` supplies the matching rate and channels and
the reference frequency the file states and `SaveRequest::protected` names files that must not
be written over though nothing is read from them (the open file), recognised by path and by
their stamp's identity, so a renamed one is still refused (on Unix only until #199 gives
Windows a file ID). Each `SourceFile::stamp`, a `SourceStamp` (length,
modification time, device and inode on Unix) taken off the window's thread after the file
describes itself, must match the handle read, both when it is opened and again from
that handle after the last sample is read (#206), and the target must be none of the files
read, also just before the rename. Output goes to
`.argand-<pid>-<n>.part` beside the target, created with `create_new` so no existing path
is ever opened for writing, then synced, checked for cancellation and renamed over the
target; any error or cancellation removes it. The source is never a valid target
(same device and inode on Unix, canonical path elsewhere, so a Windows hard link is not
detected).

The reference frequency is the physical frequency of baseband 0 Hz for both signal kinds
(#185). WAVE/IQW writing puts an `auxi` chunk (164 bytes, `CenterFreq` at 32
and `ADFrequency` at 36) before `data`. Its reference is rounded to whole Hz,
including zero; public saves refuse references outside the unsigned 32-bit range.
No `argd` chunk is written or interpreted. WAVE sample rates use the rounded
integer Hz in the standard header; standalone reopening uses that declared rate.
Private DSP backing retains exact rate/reference through explicit opening hints.
FLAC gets the Vorbis comment `AUXI` with a whole-Hz reference frequency and
`ARGAND_SAMPLE_RATE` with the exact sample rate. Opening reads `auxi` for WAVE,
including decoder-backed layouts, and `AUXI` for FLAC.
`OpenHints::center_freq` is `Option<f64>`: an explicit `--center` wins at opening.
Recent retains the pre-existing CLI hints, where stored zero means no override.
Session version 12 adds only the baseband presentation preference.

`argand_io::open_described` also returns the optional reference declared by the
file before opening overrides, alongside the reader and its existing checked
stamp. Native/fallback WAVE reuse the probed header; FLAC retains the tag already
read by its decoder. RAW has no declared reference. Existing `open`, `open_stamped`
and `reopen` remain compatible. `analysis::FileInfo` carries this provenance into
the document for read-only File information. History changes do not alter it;
successful Save/rebind supplies the newly published file's declaration. No session
field or extra reference probe is introduced.

Reference frequency is versioned document metadata (#185), alongside the
selection in the existing capture history. Metadata edits advance the same serial
used by dirty state and asynchronous save guards, without modifying the immutable
source description or requesting an FFT. Undo/Redo over unchanged captures updates
metadata and presentation only. Save requests use the effective reference;
source metadata, source stamps and source reading hints remain unchanged.
Reference edits, Undo/Redo and Discard do not update Recent. After a successful
WAVE/FLAC save, the published source and Recent have no reference override:
reopening reads the reference from the file, including zero. Unchanged headerless
saves retain their original opening hints. Private DSP hints remain in memory.

`processing/saving.rs` runs one save on its own thread with 4 MiB buffers, progress at most every
100 ms through a bounded channel and a cancel flag; dropping the `Job` cancels it and waits
at most one second for the temporary file to go. `shell/saving_ui.rs` owns File → Save as…
(Ctrl+Shift+S, Cmd+Shift+S) and Save selection as… (Ctrl+Alt+S, Cmd+Option+S), disabled
without a described capture, during a save, and for the second without a time selection.
The dialog is `App::prompt_for_new_path` in the source folder with
`<stem>_<start>-<end>s.<ext>` (seconds to the millisecond) or `<stem>.<ext>`, `.wav` for
a headerless source. The status bar shows the save last, at its right end, so it keeps its place: progress with a
cancelling ×, or an error that stays until its × is pressed or another save starts. A
finished save is said over the picture instead, `Saved <name>` in the spectrum's
bottom-right corner (`PlotView::saved_notice`, framed like the zoom pairs and free of
them in both orientations, taking no input), saying when a FLAC source was saved as WAV,
until the next mouse press, key, wheel or mouse move. A cancellation shows nothing. Each save
start is logged at info with its source, target and span. A save
outlives opening another file, but the file it is writing cannot be opened until it ends.

## Editing (#195, #196)

`editing.rs` (toolkit-neutral) keeps one document's `Editing`: a source table indexed by
`SourceId` (0 is the opened file, each with its `SignalMeta`, hints, the stamp the analysis
thread gets from `argand_io::open_stamped`, taken from the very handles that read the header
and the samples and kept only when they agree (`FileInfo::stamp`), and the `Storage` a background task reads afterwards, applied only to the
document that asked by its id and to clipboard copies with the same stamp), a `History` whose state is the selection
each version left, and one minimap envelope per source. The `Clipboard` lives on the shell,
outlives the document, and holds a `Clip` with the files it reads, never samples. Pasting
reuses a source with the same path, stamp, hints, sample type and rate and otherwise adds one, refusing a file whose
`Storage` or sample rate differs, or whose storage is not known yet; a source changed on
disk since is refused when saved, by the writer's stamp check. After delete and cut nothing
is selected; after paste the pasted samples are; undo and redo restore each version's
selection.

`shell/editing_ui.rs` owns the commands. There is no insertion cursor: a right click on the
spectrogram (`PlotView::spectrum_context_menu`) offers Cut, Copy, Paste here, Replace
selection and Delete, Paste here going before the clicked sample (`PlotView::paste_point`,
kept past the menu's dismissal because its action arrives after it) and marked by a line
while the menu is open. The Edit menu sits between File and View. Keys, in the `Plot`
context and Cmd on macOS: Ctrl+Z, Ctrl+Shift+Z (and Ctrl+Y on Windows), Ctrl+X, Ctrl+C,
Ctrl+V (replace the selection, disabled without one) and Delete; Undo and Redo are bound in
the `Shell` context as well (Ctrl+Y on Windows too), for a capture edited down to no plot. `Shell::edited` follows
every version: the document's length, `Analyst::set_edit` (a new edit version is a new
analysis generation delivered whole, as a final-only replacement, and the worker reads every capture through an `EditedSource` that opens
pasted files with `argand_io::reopen`), the selection, the view through `bound_view`, the
picture moved with its samples at once (`Editing::repicture` maps each sample of the new
version to where the version pictured held it, `Capture::position_in`, and
`Document::follow_edit` moves the grid and image columns accordingly, leaving columns of
samples never pictured empty, `NaN` in the grid, so they report no level), the backdrop
dropped until the new picture lands, the minimap composed (`minimap::compose`, mapping each cell through the
capture onto the source envelopes' cells, so no edit rescans; a pasted file gets its own
envelope built once), and both titles, which carry `•` while edits are unsaved. Opening
another file or closing the window with unsaved edits asks (`Shell::ask_unsaved`, the
close refused in `on_window_should_close` and carried out after the answer): the system's
own dialog through `Window::prompt` on macOS and Windows, and gpui-component's `Dialog`
through `Root` elsewhere, because GPUI's fallback prompt is unstyled; Enter saves and Escape
cancels. Save, Discard, Cancel (Save writes over the file, #193); the Linux close button goes through the same question (`chrome::CloseWindow`).
A save remembers its document and version (`SaveOf`): when it finishes, a whole save marks
that version saved, and only if the document is still that version does it read the edited
capture from the saved file from then on (`Shell::rebind_saved_as`, see #193 below) or do
what the question left waiting. A save needs a non-empty capture whose files all have their stamp. Saving over the open file is #193.

## Saving over the open file (#193)

File → Save (Ctrl+S, Cmd+S) writes the edited capture over its own file (`shell/replace_ui.rs`).
`write::stage` writes the temporary file while the original is still read and returns a
`Staged` that `commit` moves into place or, dropped, removes; `SaveRequest::replacing` lets
the target be the replaced file, whose stamp `commit` checks just before the rename, and a
headerless capture with unchanged reference is written headerless after its preamble
(`Output::Headerless`). When its effective reference differs from the opened source
baseline, ordinary Save instead opens the existing Save As chooser proposing WAVE,
because RAW cannot store reference metadata. The unsaved prompt follows the same
route; Cancel retains the dirty document and the original RAW. Undo restoring the
baseline reference restores ordinary headerless Save-over. Every
thread reading the open file (the analysis worker, the minimap and the envelopes of pasted
files) holds a `release::Lease`; once the file is staged the window puts unstarted readers of
the same path in their place (`Shell::swap_readers`), waits on `Released` for the old ones to
end, because Windows refuses to replace a mapped file, and commits on a background thread.
The document, plot, picture, minimap, view and selection are never let go of, so nothing on
screen changes. The new readers open the file under the shown picture (`OpenFile::rebinding`):
`Effect::Opened` then keeps the shown status and requested range (`Document::shown`), starts a
fresh history on the saved file and gives it the composed minimap as its envelope, and asks
for no picture when the one shown was finished. Only samples stored on another scale (another
sample type or normalization divisor, `Editing::scaled_as`) are drawn again, as a final-only
replacement, with a minimap scan that shows no previews. Save as of the whole capture goes on
from the saved file the same way, without the wait. Edits are refused while the file is let go
of (`Shell::editing_mut`).
The clipboard is moved onto the saved file's positions only when the commit succeeded, or
cleared with the notice when some copied samples are gone. If the commit fails the original is
untouched, the written edits are kept beside it as `<stem>.unsaved-<n>.<ext>`
(`Refused::keep_beside`, taking the name by a hard link that never replaces another file, and
leaving the temporary file in place when no name can be taken), and the document keeps its edits on it, the readers opening it again under the picture; a file
whose stamp changed is opened afresh, since the edits no longer say where its samples are. While the file is let go of and replaced
(`Shell::replacement_active`) nothing opens and the window does not close; the status bar
says so and offers no cancel. `stage` fixes whether it replaces the target before writing, `Saved::stamp`
is taken from the written file's own handle and gives the clipboard its stamp, and kept
edits ask again for pasted envelopes that never finished (`Editing::restart_envelopes`). Save needs unsaved edits, a non-empty checked capture and no save running,
and is refused for FLAC above the encoder's rates; the unsaved-edits question's Save uses it,
falling back to Save as.

## Changed on disk (#200)

No read path opens a capture's file again by path without proving it is the file opened.
`DecodedSource` keeps the opened handle in an `Arc<File>` and every fresh parser (a FLAC seek,
the length count, the automatic level scan) reads it through `shared_file::SharedFile`, a
cursor of positioned reads (`read_at`, `seek_read` on Windows) that keeps its own offset and
never relies on the handle's position (which `seek_read` moves), after stamping the handle again (`DecodedSource::fresh`). `argand_io::reopen`
takes the stamp the file was first opened with and fails unless the path has it before parsing
(so a removed, emptied or unreadable replacement is a change too, also when it is replaced
between that check and the parse) and the new handles have it.
Both failures are `argand_core::SourceError::Changed`, which `IoError::is_changed` and
`disk_change::changed_file` recognise through an `anyhow` chain. The minimap, the analysis
worker's pasted sources (`EditState::sources` carries each `SourceFile`), pasted envelopes and
extraction all reopen with a stamp.

A reader cannot name the file it found changed, so `disk_change::naming` names it where it is
known: the minimap names its file, and the analysis worker names the opened file while the
capture reads nothing else; otherwise the notice says the capture changed. `Document::is_changed`
holds from a failed analysis, minimap or envelope (`Document::note_failure`; the opened file's
minimap and a pasted file's envelope only while the current version still reads that file), or from the activation check (`shell/disk_change_ui.rs`), which
stamps the files the current version reads by path on a thread of its own, at most one at a
time per document, and applies the result only to the same document and the same stamps. No
check starts or applies while `replacement_active` or while a saved file is being rebound
(`OpenFile::rebinding`, when the document still holds the old stamp), and `swap_readers` drops
a pending one. While it is set, `ask_for_a_picture` asks for nothing,
`OpenFileState::holds_picture` rejects progress and pictures (including a backdrop restyle in
flight) while an opening or a failure still lands, and the minimap takes no snapshot or
failure, so the held picture and minimap stay. A Save as that rebinds the capture to its saved
file clears it (`Rebind::Fresh`), and a late storage description applies only to the opening
with its stamp (`Shell::file_stored`), edits are refused (`editing_mut`, `edit_commands_available`), and the status bar
shows `<name> changed on disk` with a Reopen button in place of the analysis status. Reopen
opens the origin again; the unsaved-edits question then offers only Discard and Cancel
(`ask_unsaved` without `savable`), because the writer refuses a changed source. An overwrite
in place of a mapped file is seen only by the activation check, and one that keeps length and
modification time is beyond `SourceStamp`.

## Conversion publication and saving (#189)

`write::EncodedWave`
shares FloatWave's safe staging/publication and writes all twelve supported types, rounding
integers to nearest with ties away from zero, saturating without dither and counting
clipped scalar values. Floats keep finite overscale values; nonfinite data fails.
Sources are stamped before/after reads and after final sync, before publication.
Conversion writes only to private temporary storage and asks for no user destination.
The result reader is prepared while the current document remains intact; only its
Opened delivery adopts those same readers, without an unsaved-input prompt or Undo.
Failed, cancelled and stale preparation never replaces or stops the current reader. `OpenFile::unsaved` holds the generated display
name and requested WAV/FLAC container separately from the private encoded-WAVE backing
path. WAV supports all twelve sample types; FLAC supports U8/I16 with the existing codec
rate limits and exact-rate/reference comments. Choosing FLAC changes any unsupported
draft format to I16 in its current domain, preserving rate and carrier and showing a
warning; U8/I16 remain unchanged and returning to WAV keeps the chosen format.

First Save and selection export use the requested container; an explicit
WAV/FLAC Save as extension overrides it. `History::unsaved` starts dirty without an Undo
version. The first Save asks for a destination, then successful rebind clears unsaved
identity. Private computed/encoded backing is opened with explicit exact sample-rate and
reference hints from the processing request. Computed sources retain those hints
for later analysis, clipboard and save reads; WAVE integer header fields are not
the source of truth for internal DSP parameters.
Temporary paths never enter Recent. `Source::backing` retains encoded
scratch separately from computed FloatWave samples, including clipboard references,
so it is never encoded twice. Input/output scratch ownership lasts until the job ends;
reader leases retain backing through termination, and opening another capture cancels
conversion. The result opens with normalization
disabled so f16x8 arbitrary units are not automatically rescaled.

See [conversion DSP and reference semantics](processing.md#capture-conversion-189)
and [File information UI](ui.md#file-information-and-conversion-189).

## Native packed PCM24 (#192)

`SampleFormat::I24` is a signed three-byte little-endian scalar with nominal full
scale 8388608. `rl_i24` stores one scalar per sample and `iq_i24` stores an
indivisible six-byte I/Q pair. Conversion sign-extends bit 23 with fixed-byte
standard integer conversion, independently of host endianness; level scans use
the same scalar decoder and aligned three-byte chunks. Normalization, gain and
original-unit reconstruction follow the existing mapped integer path.

RIFF/RF64/BW64 PCM24 is native only when the frame is tightly packed and every
container bit is valid. Reduced-valid-bit extensible PCM24 retains its existing
decoder fallback and metadata semantics; no partial-valid-bit support was added.
The writer's generic RF64 guard still refuses any format the native reader cannot
reopen. Full-valid PCM24 passes that guard, preserves stored bytes for unchanged
segments, and uses the existing three-byte integer path for edited/computed output.
Raw input accepts both I24 tokens and existing offsets, whole-frame truncation and
source identity checks. FLAC decoding is unchanged; explicit FLAC conversion still
supports only U8/I16 and filters I24 in the standard format selector.

Full-valid PCM24 reports I24 rather than the decoder's I32 representation. Explicit
sample-type overrides follow native byte reinterpretation, as for other mapped
formats. Recent/session history preserves original opening hints and does not turn
inferred decoder metadata into an override; automatic PCM24 detection is repeated
on reopening. Deferred partial-valid layouts retain their decoder override semantics.
