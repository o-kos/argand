# Issue #185: Reference frequency and baseband presentation

Resolves [#185](https://github.com/o-kos/argand/issues/185).

## Overview

Edit the capture's reference frequency directly in File information, retain it
through ordinary Save/Save As and Recent, and optionally show baseband frequencies
without changing samples or recomputing FFTs.

## Roles and risk

- Class: A: frequency interpretation, asynchronous display, history and safe saving.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate read-only context.

## Decisions

- The owner approved an inline Reference frequency numeric editor in the upper
  File information metadata section, including zero and baseband mode. No separate
  dialog, Edit button or duplicate Edit menu command.
- Enter/blur applies a valid value; Escape restores the prior value. Invalid
  input shows an inline error without changing the capture.
- A reference change is an undoable metadata edit, dirty until explicitly saved.
  Save and Save As write the effective reference, without sample modulation.
- View > Baseband frequencies is a checked presentation preference, persisted
  with other session display preferences. The physical reference is retained.
- Conversion's Reference is renamed Carrier frequency and keeps its DSP meaning.
- Source metadata and analysis coordinates remain immutable. Display labels use
  an explicit source-to-display offset. Selections retain source-frequency
  coordinates; extraction/suppression subtract the paired source reference once.
  Whole/time exports write effective metadata; extracted-band output stays at
  reference zero under its existing contract.
- Metadata and sample edits share one history serial and saved-version guard.
  Reference/baseband changes and metadata-only Undo/Redo do not request FFTs.
- Recent distinguishes a missing override from an explicit zero. Legacy zero
  hints remain absent overrides during session migration.
- The owner approved routing ordinary Save to Save As with a WAVE proposal when
  a RAW/headerless capture has an effective reference different from its opened
  source baseline. The original RAW stays untouched. Sample-only RAW Save and
  Undo restoring the baseline reference retain ordinary headerless Save-over.
  The unsaved prompt uses the same route; chooser Cancel retains the dirty document.

## Context and boundaries

WAVE already reads/writes exact `argd` reference metadata and compatible `auxi`;
FLAC uses Argand Vorbis comments. Opening hints override file metadata, including
explicit zero. Frequency navigation is normalized while worker caches, display
grids and selections currently carry source physical Hz. Source stamps, staged
publication, cancellation and texture retirement must remain intact.

No toolkit/toolchain/dependency changes, custom controls, spectral inversion or
frequency correction of sample values are included. Use the existing NumericEditor
and standard menu controls. The installed SDK and local debug/test/release/check
baseline must be reused; dependency rebuilds require explicit owner approval.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Inline editor commits valid reference, preserves invalid drafts, restores on Escape | Production stock Input Enter, blur at zero, fractional value, invalid draft and Escape | Passed on Linux Xvfb IQ capture | Negative/locale input and other platforms not exercised in production |
| Reference changes are undoable, dirty and saved exactly | Mixed history and stale-save regressions; production Save/reopen metadata and sample comparison | Passed for covered WAVE/metadata paths | RAW follow-up passed |
| Baseband preserves samples, selected bins, zoom and levels | Offset/selection tests; production checked menu, physical reference retained, horizontal/vertical presentation | Passed for covered scenarios | Real-signal runtime and full gesture routes unperformed |
| Neither metadata edits nor baseband require FFTs | Worker-generation and metadata-only Undo regressions | Passed | Automated evidence |
| Processing/export uses the effective reference exactly once | Extraction zero-reference, suppression and conversion regressions; retained-panel Convert after Save in production | Passed for covered scenarios | RAW follow-up passed |
| Save rebind preserves coordinates and source safety | Sample-integrity/rebind, failed-publication/navigation/suppression, stale request/source tests | Passed | Automated and production Save/Convert evidence |
| Explicit zero survives Recent and old sessions migrate | Explicit-zero and legacy session migration regressions | Passed | Automated evidence |
| Standard controls fit the affected panel | Production invalid/valid input, Escape, retained Save/Convert, menu, narrow vertical window and light theme | Passed for exercised Linux X11 scenarios | Native Wayland/Windows/macOS and other unperformed routes remain limits |

## Implementation steps

- [x] Add versioned reference metadata and effective output metadata.
- [x] Add source/display coordinate mapping and baseband presentation.
- [x] Preserve processing, saves, asynchronous deliveries and rebind semantics,
  including the agreed RAW Save As route.
- [x] Add inline reference editor and distinguish conversion carrier.
- [x] Migrate session/Recent persistence and add checked View command.
- [x] Add focused history, stale-save, numeric-coordinate, session, save-rebind
  sample-integrity and extraction/suppression regressions.
- [x] Update architecture and changelog.
- [x] Complete bounded isolated production Xvfb acceptance and three independent
  review rounds; final round clean, all three findings accepted and resolved.
  Platform/runtime limits remain recorded in the acceptance evidence.
- [x] Move this plan to `docs/plans/completed/` after bounded RAW review and acceptance.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked` (891 tests; application 522; doc tests passed).
- [x] `cargo build --release --locked`, last after the other checks passed.
- [x] Bounded production application acceptance on isolated Xvfb, with exact revision and
  local-diff identity recorded outside the historical plan.

The initial plan-only publication uses diff/reference checks; it does not compile
code or claim UI acceptance. Code pushes require the full local gate. Source
inspection and fingerprints alone do not prove dependency artifact reuse.

Final evidence and review snapshots are retained outside the worktree under
`/tmp/argand-185-ui/acceptance.md` and `/tmp/argand-185-review-round3/`.
The final release binary hash matches the binary exercised in production.
The subsequent plan evidence update is documentation-only; scoped diff and link
checks apply, with no additional build required. The Pull Request remains Draft for owner feedback; completion of this plan does
not authorize Ready or merge.

## Post-completion

No release or merge is authorized by this plan.

## RAW follow-up acceptance

The owner resolved the prior pending format decision. This bounded follow-up
retains class A and the same role assignments, with a separate review cycle
against the previously reviewed implementation.

- [x] Route changed-reference RAW Save through existing Save As, proposing WAVE.
- [x] Verify Cancel, unsaved-prompt continuation, WAVE metadata/sample integrity,
  source protection/rebind/Recent and unchanged-reference RAW Save/Undo behavior.
- [x] Run the exact full local gate, release last, and isolated production Xvfb.
- [x] Obtain independent review and coordinator assessment before publication.

No new widget, dependency, toolchain, feature, target or configuration is needed.

RAW follow-up results: exact fmt/Clippy/full tests (893 tests, application 524)
and release-last passed. Production Linux X11 used an isolated Xvfb and a private
D-Bus/GTK portal: ordinary Save proposed WAVE, Cancel retained the dirty RAW,
accepting Save created/rebound WAVE with exact reference and all sample bytes
unchanged while the RAW remained untouched. Automated regressions cover metadata
Undo plus sample-only Headerless Save, unsaved-prompt Save/Cancel, selected-band
translation, Recent reopen and cleared RAW hints for subsequent ordinary Save.
The separate bounded follow-up review accepted and corrected two test-harness
findings; its second round was clean. Evidence and exact snapshot identity are in
`/tmp/argand-185-raw-ui/acceptance.md` and `/tmp/argand-185-raw-review-round2/`.
Native unsaved-confirmation interaction, cold restart/FileOpen, other RAW encodings,
real-signal runtime, native Wayland and Windows/macOS remain unperformed limits.
The subsequent plan completion/move is scoped documentation-only, checked by diff
and links without rebuilding unchanged production code.
