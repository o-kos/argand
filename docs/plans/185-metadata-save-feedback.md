# Issue #185: Metadata and save completion feedback

Resolves [#185](https://github.com/o-kos/argand/issues/185), continuing Draft #250.

## Roles and risk

- Class A: capture metadata I/O and asynchronous save completion/data safety.
- Primary: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, fresh read-only context.
- Consultant: `gpt-6-astra` medium, read-only targeted advice on exact private
  scratch parameters when removing argd; public export decisions remain with owner.

## Owner decisions and scope

- Remove Argand-specific `argd` WAVE metadata. Use `auxi` reference frequency.
- Display/edit Reference frequency in whole Hz using the existing stock Input.
- Preserve IQW as a detected RIFF/WAVE container; do not classify it as RAW by extension.
- Investigate and fix persistent Saving 100% after a reference edit and Save.
- Inspect reported Convert availability after frequency edits; distinguish reference
  metadata commits from actual carrier/domain conversion before changing semantics.
- FLAC uses the Vorbis comment AUXI with an integer frequency in Hz.
- Owner chose whole Fs for WAVE export: use the standard header's rounded integer
  rate on standalone reopening, retaining exact private DSP metadata in hints.
- Keep this work in the current Draft PR. Preserve unrelated policy edits.
- Follow-up owner feedback authorizes Convert after a reference-only edit.
- Add a Baseband frequencies toolbar toggle beside Grid and a Ctrl+B shortcut,
  sharing the existing menu action and checked state. Keep numeric input key
  handling unchanged. The toolbar toggle uses the existing standard Button style.
- Owner requested removal of reference-edit persistence in Recent/session.
  Reference edits remain only in document history until explicit Save. Public
  WAVE/FLAC reopening reads the saved reference from the file without an added
  reference override. Preserve original opening parameters for unchanged RAW
  files and the pre-existing CLI opening-hint behavior.
- Replace the BB abbreviation with a frequency-zero spectrum icon in the same
  standard toolbar Button. Disable the button, View command and shortcut action
  when there is no active capture/reference or the effective reference is zero.
  Keep the saved baseband preference; do not reset it on zero-reference files.
- Baseband tooltip names the next action when enabled. At zero reference use
  the owner-selected text: "A nonzero reference frequency is required".
- Hide the Baseband check/pressed state when switching is unavailable, retaining
  the saved preference so a nonzero reference restores its presentation.
- Tooltip copy must not contain semicolons or sentence structures that need them.
  Simplify the message rather than replacing the punctuation.

## Acceptance

- WAVE/IQW save writes auxi without argd; reopening uses the whole-Hz reference.
- Existing foreign auxi captures continue to open, including the IQW extension.
- Reference editing uses whole Hz, including paste/Enter/blur/step/Escape paths.
- The reported edit-reference/Save sequence completes and removes Saving status;
  saved file can be reopened and its samples/reference verified.
- Failed/cancelled saves and publication/source identity protections remain intact.
- Convert behavior is explained and any agreed correction verified in production.
- Baseband toolbar, menu and Ctrl+B stay synchronized; switching preserves the
  physical reference, capture/history, selection and analysis generation.

## Tasks

- [ ] Reproduce the stuck save and establish the cause before correcting it.
- [x] Implement agreed metadata format and integer editor policy with regressions.
- [x] Resolve FLAC policy and verify Convert availability.
- [x] Update current architecture and changelog.
- [x] Full local fmt/Clippy/test/release-last gate.
- [x] Metadata/input production acceptance on isolated Xvfb with disposable data/settings.
- [x] Independent metadata review and current-snapshot assessment.
- [x] Enable Convert for committed reference changes relative to the source file;
  verify conversion, Undo and Save/rebind availability transitions.
- [x] Add and exercise Baseband toolbar and shortcut, including tooltip, focus,
  narrow-window layout and both orientations.
- [x] Run a new focused review cycle for the newly authorized Convert/baseband
  changes, after the local gate and production acceptance. The previous three
  rounds cover the completed metadata snapshot, not this subsequent scope.
- [x] Remove Recent updates on reference edits/discard and published-file
  reference overrides; restore legacy session zero-as-absent representation.
- [x] Verify unsaved edits do not alter Recent; Save and later reopening use
  file metadata, including zero, with RAW and conversion behavior preserved.
- [x] Full gate, production acceptance and independent review of this removal.
- [x] Replace the toolbar abbreviation and unify baseband availability across
  pointer/menu/shortcut, including zero/nonzero/Undo and retained preference.
- [x] Verify icon/disabled/tooltip appearance and behavior in production, full
  local gate and a focused independent review of this owner feedback.

- [ ] Apply the selected tooltip wording and hide inactive baseband checks,
  preserving the saved preference and verifying zero/nonzero/Undo transitions.
- [ ] Record the tooltip writing rule and complete local gate, production
  acceptance and independent review of this bounded GUI follow-up (Class B,
  with the existing primary/reviewer assignments).

## Investigation so far

- Baseline production Linux release, isolated Xvfb: a 2-second generated WAV,
  a 30-minute generated WAV, and a disposable copy of the owner's named IQW
  all completed metadata Save-over and removed Saving status. The IQW was saved
  twice with different committed references. This does not establish the cause
  of the owner's reported persistent status. OS and actual executable selection
  have been requested; no speculative save-state correction has been made.
- IQW is detected by RIFF/WAVE content, not its extension. The existing recording
  contains metadata chunks; no headerless conversion route is needed for it.
- Convert currently ignores metadata-only reference commits because committing
  updates both input and output references. The owner has since requested that
  these edits enable Convert; compare against the opened source baseline too.
- Stock Input preserves draft text; the numeric wrapper rounds Reference frequency
  on commit/step. Space-grouped editing tolerates changed integer group lengths.
- Focused regressions passed for integer reference formatting, edited whitespace
  grouping, and FLAC AUXI integer comments. Current full gate passed 903 tests,
  formatting and Clippy, followed by release build last.

- Consultant advice accepted: preserve exact private scratch Fs/reference in the
  existing OpenHints at initial opening and in computed-source ownership. No new
  scratch format or generic reopen override is introduced. The owner chose whole-Hz public WAVE Fs; signed/private DSP metadata stays f64.

## Review dispositions

Round 1 returned three P2 findings, all accepted:

- Scope rounding to WAVE/FLAC publication; unchanged RAW reference hints remain
  fractional. Added a headerless sample-save regression.
- Derive one saved-source description for reader restart and clipboard relocation.
  The whole-Fs rebind regression now also pastes the preserved clipboard back.
- Private identity-copy uses a representable neutral header reference when needed,
  while retaining the exact signed/large value in its request/opening hints.
  The I32 byte-copy regression covers zero, negative and above-u32 references.

No findings rejected. Rounds 2 and 3 had no substantive findings. The final round
also assessed the RAW test-harness correction (queued delete must run before Save).
The primary verified the current patch against that reviewed snapshot and assessed
source stamps, publication/cancellation and exact private metadata. Only this plan's
progress/evidence text changed afterward.

## Production results and remaining work

- Linux X11 release on task-owned Xvfb with isolated settings, Russian numbers.
  Actual zero deletion preserves the draft and cursor without error; Enter rounds
  to whole Hz, Up/Down step whole Hz, and Save completes with panel open or closed.
- Saved IQW contains only fmt/auxi/data, reference12579003 and rate24000; all
  172800000 sample bytes match the original fixture by SHA256.
- FLAC Save-over writes AUXI=14074000, omits the old reference tag, retains PCM MD5;
  fresh production reopening without a center override reads that same reference.
- Shared Analysis overlap retains its existing whole-number validation:75.5 is
  rejected,75 commits; adjacent Select opens/dismisses. Fractional Range runtime
  interaction was not established; its default state was not an editable stop.
- Owner's permanent Saving100 report remains unresolved. Before rebuilding, the
  running owner process used the prior release and a Wayland session environment;
  no active save thread or .part handles remained at observation. This is not proof
  of its visible UI state or of the cause. The owner was asked to confirm whether
  the status is still visible. No speculative save-state fix has been made.
- Metadata changes can be published in the current Draft, but this plan remains
  active until the save-status report is resolved. Do not mark the combined task
  complete or move the PR Ready. Native Wayland/Windows/macOS acceptance is not
  established by these Xvfb checks.

## Baseband and Convert follow-up

- Added Ctrl+B in the Plot context and the existing standard toolbar Button
  labelled BB beside Grid. Menu, toolbar and shortcut invoke ToggleBaseband.
- Convert compares the effective reference with the opened source metadata as
  well as its existing conversion options. The retained panel refreshes that
  baseline after Save; a newly opened panel also detects an already edited source.
- Full local formatting, Clippy and 905 tests passed; production release built
  last. The first test run exposed an unsuitable synthetic-document focus fixture;
  the corrected test uses a real ready I/Q capture. No product change was made
  to accommodate that fixture.
- Production Linux X11 on task-owned Xvfb: actual pointer and keyboard toggles,
  tooltip/shortcut and menu check state, both orientations, a 700x600 window and
  light/dark themes were exercised. Reference-only Convert enabled after Enter,
  disabled after Undo and Save, and created a new unsaved capture when clicked.
- The 30-minute IQW copy saved reference 12579011; Convert wrote 12579012 in
  its private backing. All 172800000 sample bytes had the same SHA256. Saving
  status disappeared after completion without further input in this X11 run.
- The owner's Wayland-environment process journal records another completed
  Save-over. This does not establish its visible state or explain the reported
  stale Saving status. That defect remains unresolved; no speculative fix.
- Session override persistence was unchanged in this earlier follow-up; the
  owner has since requested removal in the next bounded change above.
- The independent follow-up review returned no substantive findings. It covered
  source baseline/freshness, publication protections, shared action/focus,
  toolbar width/title-drag exclusion and supplied production evidence. The
  primary assessed the current source against that snapshot; only this result
  and progress text changed after review. Saving remains open.

## Remove reference edits from session

- Removed Recent writes from metadata commit/Undo/Redo and Discard. Document
  history still owns unsaved reference changes and the ordinary save prompt.
- Successful WAVE/FLAC publication clears the reference opening override so the
  file is authoritative on reopening; private DSP and unchanged RAW opening
  hints retain their existing contracts.
- Restored the legacy session reference representation: zero means no CLI
  override. Version 12 remains for the baseband preference alone. No heuristic
  cleanup of older, indistinguishable CLI versus edited reference values.
- Formatting, Clippy and all 907 tests passed, followed by release build last,
  including actual Recent unchanged
  assertions, Discard/reopen and nonzero/zero Save/reopen without an override.
- Production Linux X11/Xvfb acceptance verified unchanged Recent on disk during
  an unsaved edit and the original file reference after restart/Recent reopening.
  Nonzero WAVE Save/reopen and explicit CLI reference -> zero -> Save-before-Close
  -> fresh Recent reopening used file metadata without an override. All sample
  bytes stayed identical. FLAC Save/restart/Recent used AUXI without an override
  and retained PCM MD5. Only disposable copies/settings and owned displays used.
- Removal review round 1 found one P2: a pending Open/Close could run before
  rebound refreshed Recent, retaining an old CLI reference override. Accepted:
  remember the published origin immediately after successful commit, before
  continuing the pending operation. Added a nonzero CLI reference -> zero ->
  Save-before-Open regression; the same ordering covers Close.
- Round 2 found no further substantive issues. The independent reviewer checked
  the corrected publication ordering, regression, exact production binary and
  supplied runtime/sample evidence. No findings rejected. The primary assessed
  the current code against that snapshot; only this progress/result text changed
  after review. Native Wayland/Windows/macOS and Saving100 remain unverified/open.

## Baseband icon and availability feedback

- Replaced BB with a spectrum/zero SVG in the existing standard Button. Shared
  availability uses the effective reference for toolbar, View menu and action;
  zero or unavailable reference disables switching without resetting preference.
- Tooltips name the next relative-to-reference or absolute action. At zero the
  tooltip explains identical scales and omits the inactive shortcut.
- Full formatting, Clippy and all 907 tests passed; release built last.
- Production Linux X11/Xvfb exercised both action tooltips, disabled icon/reason,
  inert pointer/menu/shortcut with retained preference, Undo restoring nonzero
  availability, light/dark themes, a 700x600 window and no-capture disabled menu.
  Only owned displays, disposable files and isolated settings were used.
- Independent read-only review found no substantive findings and checked the
  supplied production screenshots, preference evidence and binary identity. The primary checked the source
  against the reviewed snapshot; only plan progress/results changed afterward.
- Owner acceptance of icon appearance remains pending. Native Wayland/Windows/
  macOS remain unperformed; the persistent Saving100 report remains unresolved.

## Validation and limits

Record actual runtime observations separately from automated tests. No toolchain,
SDK, dependency graph or artifact layout changes. Do not alter the owner's files.

## Post-completion

Keep Draft for owner feedback; no Ready, merge or release transition authorized.
