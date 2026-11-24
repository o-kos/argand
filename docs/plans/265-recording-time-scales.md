# Issue #265: Recording start metadata and persistent scale presentation

Resolves [#265](https://github.com/o-kos/argand/issues/265).

## Overview

Allow recording-start metadata to be set, edited, cleared, saved and reopened.
Present time relative to the recording or as absolute UTC, and frequency relative
to baseband or as physical RF, without changing samples, sample coordinates or
analysis requests. Keep the presentation preferences between application launches.

Work takes place in the main checkout on the Issue branch. No linked worktree,
toolkit change, SDK rebuild, toolchain installation or build-directory change is
authorized. Existing ignored mock artifacts and build outputs are preserved.
The owner has approved storage, presentation controls, timeline semantics and UTC
interpretation of bare auxi. Product implementation is authorized.

## Roles and risk

- Class: A. Changes cross public core metadata, readers/writers, immutable history,
  asynchronous save guards and multiple GUI consumers. Data safety and timestamp
  correctness require independent review and current-revision evidence.
- Coordinator: `gpt-6.1-sol` medium, owner communication and final assessment,
  without product-code edits.
- Executor: `gpt-6.1-sol` medium, investigation, plan, implementation, tests,
  acceptance evidence and Pull Request operations.
- Reviewer: `gpt-6.1-sol` high, read-only in a fresh separate context.
- Consultant: none currently. A justified unresolved question may use
  `gpt-6.2-astra` high through the read-only Codex CLI under the repository rules.

## Context

- `SignalMeta` in `crates/core/src/signal.rs` carries effective sample rate,
  frequency reference, sample domain and length, but no recording start.
- `crates/io/src/riff.rs` reads the reference from `auxi`. The existing writer
  creates a 164-byte body with start/stop SYSTEMTIME slots at offsets 0 and 16,
  reference at 32 and sample rate at 36. Start/stop are currently zero.
- `crates/io/src/decoder.rs` and `crates/io/src/write.rs` already read/write FLAC
  Vorbis comments `AUXI` and `ARGAND_SAMPLE_RATE`.
- `crates/app/src/editing.rs` versions reference and selection together. Metadata
  changes advance dirty/save serials with immutable capture ownership. Shell
  updates reference presentation without requesting FFT analysis.
- `crates/app/src/navigation/time_ruler.rs`, `navigation/selection_entry.rs`,
  `axes/`, and `shell/plot/navigation_ui.rs` share ruler, cursor and editable
  selection formatting. Selection endpoints need inverse parsing as well as text.
- `shell/reference_ui.rs` provides the existing metadata panel and numeric editor.
  `shell/hints.rs` supplies the shared information-panel lifecycle.
- `shell/app_menu_ui.rs` has View / Time scale format. The time ruler has its own
  context menu; the existing spectrum context menu belongs to the plot content.
- `session.rs` persists the existing hms/seconds/samples format using advisory,
  atomic, merged session writes and guarded version compatibility.

Read the current architecture contracts in `docs/architecture/overview.md`,
`analysis-rendering.md`, `navigation-selection.md`, `ui.md`, `editing-io.md` and
`processing.md`, the control inventory, and the GPUI Kit coding/design and Argand
UI-acceptance skills before changing their respective paths.

## Decisions

### Approved storage and precision

- Recording start is UTC with one millisecond precision in both containers.
- WAVE/IQW uses the existing `auxi` start SYSTEMTIME fields. No private RIFF chunk,
  reserved-byte extension or unrelated metadata format is introduced.
- FLAC uses the existing VORBIS_COMMENT block with `DATE` and a full RFC3339 UTC
  timestamp, for example `DATE=2026-11-23T12:34:56.123Z`.
- Approved follow-up supersedes the original no-filesystem-default decision: absent
  embedded start uses supported creation time from the stable opened file as a clean
  initial origin. Never substitute modification/change time or the current clock.
  Missing or malformed metadata does not prevent valid sample data from opening.
  Samples, true rate and domain are preserved by metadata edits.
- Reference remains the physical frequency of baseband zero. Presentation changes
  retain the stored reference, including zero, with real one-sided and complex
  two-sided spectra unchanged.

### Approved controls and presentation preferences

- View / Time scale offers Relative and Absolute UTC. The existing relative
  formats hms, seconds and samples remain available and remembered independently.
- View / Frequency scale offers Relative and Absolute RF.
- The corresponding ruler context menus expose the same commands in either
  orientation. Do not repurpose the plot content's edit context menu.
- Ctrl+Shift+T toggles relative/absolute time and Ctrl+Shift+F toggles frequency
  presentation, using the existing Cmd mapping on macOS. Ctrl+T remains orientation.
- Store application-wide presentation preferences in the session file, retaining
  existing session safety and compatibility rules. They are not per-file metadata.
- Without recording start, Absolute UTC is unavailable with an explanatory reason.
  Effective time presentation is Relative, while the saved preference is retained.
- A small standard Button at the overview/frequency-ruler junction indicates both
  effective modes and opens one common panel: bottom-left or top-right according to
  orientation. The label order is always frequency/time, such as `R/A`.
- Remove the separate reference-frequency status-bar metadata button. The shared
  panel owns both metadata editors and both presentation mode groups.
- Use standard radio buttons for frequency Relative / Absolute RF, time Relative /
  Absolute UTC, and the relative time formats. Reuse the existing numeric reference
  editor and locale digit grouping for frequency values.
- Include the standard UTC DatePicker with seconds-precision time and built-in
  inline clear. Real user Change events commit immediately. Unchanged dismissal
  preserves exact origin. Escape closes popup before parent. No separate footer
  Apply/Cancel/Clear/close buttons are added. Numeric editing retains existing
  Enter/blur commit and Escape uncommitted-draft conventions, mutual exclusion,
  external-tooltip and focus policies.
- Cosmetic layout details may be refined during implementation and running-app
  acceptance. No new nonstandard widget is approved by this layout decision.

### Approved timeline and import semantics

- Full conversion, normalization, inversion, band suppression and same-length DSP
  retain recording start. Prefix deletion, time-selection export and extraction
  advance start to the first remaining input sample. Tail deletion retains start.
- Interior deletion, insertion and replacement that change duration clear start.
  Undo/Redo restore start together with content. Keep precise internal offsets to
  avoid cumulative rounding drift; serialization rounds to nearest millisecond,
  with maximum error 0.5 ms. Sample endpoints remain sample-accurate and are not
  quantized to metadata precision.
- Read and write bare auxi calendar fields as UTC, never the host timezone. The
  owner explicitly selected this interpretation after the source comparison.
  Document that external local-time writers (including the inspected SDRangel
  writer) cannot be distinguished by auxi alone; the user can edit actual start.
- Clearing the recording-start field clears metadata without a separate button.

### Implementation invariants

- Version recording start alongside reference and selection in immutable history.
  No-op edits stay clean. Undo/Redo/Discard restore effective metadata correctly.
- Metadata-only updates and presentation switches do not ask for a new analysis
  generation. Existing source metadata/hints/stamps and capture identity stay intact.
- Keep selections in integer sample coordinates and internal frequency coordinates.
  Apply presentation conversion at shared formatting and inverse-entry seams.
- Absolute endpoints use start plus physical sample offset; durations stay durations.
  Avoid epoch-sized floating-point addition that loses subsecond/sample precision.
- Saves use effective versioned metadata under existing source protection, staged
  publication, cancellation and stale-save guards. RAW cannot persist new metadata:
  follow the existing metadata-change Save As route rather than silently lose it.
- Define DATE duplicate/conflict and invalid-input handling, range validation,
  and start/stop auxi behavior explicitly in code/tests and current documentation.

## Rejected alternatives

- Replacing or clearing the stored reference to implement baseband display.
- Treating a date-only FLAC DATE as a known absolute recording-start instant.
- Claiming auxi has a universal UTC convention: primary implementations disagree.
- Adding BWF, iXML, GUANO or a private metadata chunk/block automatically. Existing
  auxi and Vorbis-comment mechanisms are the owner-approved storage scope.
- New status-bar metadata controls, comboboxes or panel action buttons.
- Mock-only UI acceptance, a toolkit test build, or an older installed binary as
  evidence for the changed production application.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Set, edit, clear and reopen start with reference | WAVE/native and decoder-backed layouts, RF64 where supported, FLAC DATE round trips; compare samples, count, rate and domain | Pass | Native Enter/blur clear/save/reopen verified |
| Safe invalid and absent metadata | Zero/short/malformed SYSTEMTIME, invalid dates, incomplete DATE, unsupported precision/zone and conflicting tags | Automated pass | UTC import convention documented |
| Correct UTC labels and entries | Start before midnight, fractional seconds, selection endpoints/duration, zoom and both orientations; absolute entry maps to original sample span | Pass | Both orientations and640px production scenarios verified |
| Presentation leaves data and analysis intact | Toggle each command entry point, inspect selection/view/reference and analysis request count/generation | Pass | Native analysis invariant also checked by GPUI tests |
| Persistent and consistent modes | Isolated session normal restart, unavailable Absolute, files with zero/missing reference, checked menu and radio state, corner frequency/time label | Pass | Normal restart verified Absolute/Baseband and retained Seconds |
| Metadata history and file safety | No-op/Undo/Redo/Discard, whole and selection saves, RAW Save As, stale save completion, source protection and cancellation regression cases | Automated pass | Existing publication regressions pass |
| Processing and timeline semantics | Agreed prefix/tail/interior/paste/extraction/conversion scenarios and undo/save/reopen | Automated pass | Precise private-conversion origin regression passes |
| Production UI behavior and appearance | Current release binary in isolated Xvfb, both corner placements, pointer and keyboard, radios, grouped frequency input, timestamp edit/empty clear, Enter/blur/Escape, menus/shortcuts, busy and unavailable states | Local scenarios pass | Owner manual acceptance pending, Xvfb does not prove native Wayland/other OS behavior |
| Current revision independently reviewed | Neutral exact handoff, fresh read-only reviewer, dispositions and clean final round or explicit remaining findings | Two rounds complete, final code clean | Review does not replace owner UI acceptance |

## Implementation steps

- [x] Resolve and record pending owner decisions before product edits.
- [x] Verify installed toolchain selection without triggering a proxy download,
  existing toolkit/SDK selection and workspace-local target setup before Rust work.
- [x] Add validated recording-start value and metadata propagation at core/I/O seams.
- [x] Implement auxi/DATE reading and writing, invalid-input handling and round trips.
- [x] Version metadata and apply agreed save, processing and timeline semantics.
- [x] Add shared relative/absolute presentation, endpoint entry and session persistence.
- [x] Implement approved corner button/common panel and synchronized menu/shortcut paths.
- [x] Update metadata, navigation, UI and session architecture documentation and changelog.
- [x] Complete targeted tests, full local gate, local production scenarios and independent review.
- [ ] Obtain owner manual UI acceptance while the PR remains Draft.
- [ ] Move plan to `docs/plans/completed/`, rebuild current release and request Ready/full CI
  only after all agreed implementation and acceptance work is complete.

## Validation

For the initial plan-only commit, review the diff and local links. No product code,
Rust build or test is claimed. The complete code-change gate applies before code
pushes; initial plan publication follows the coordinator's scoped workflow decision.

- [x] Targeted timestamp, I/O, history, presentation, inverse-entry and session tests.
- [x] Relevant production-control GPUI interaction regressions with normal cleanup.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass.
- [x] Isolated production Xvfb acceptance report with exact revision/local-diff identity,
  binary/build command, scenarios, observations and remaining limitations.
- [x] Independent review of the corrected exact code snapshot. Coordinator assessment remains part of final handoff.
- [ ] Current Ready PR full CI succeeds explicitly on Linux, Windows and macOS.

## Owner UI feedback follow-up

The owner rejected the initial visual balance and corner affordance after manual
inspection. Class A and the existing roles remain unchanged. Previous production
captures are historical evidence for the first implementation, not acceptance of
this correction.

Approved independent corrections:

- [x] Use a visibly outlined standard corner Button positioned from the actual
  minimap/frequency-ruler corner geometry in both orientations.
- [x] Give the reference NumericEditor and current timestamp Input matching
  standard outlined field frames without changing other property panels.
- [x] Align Frequency and Time sections on shared label/control columns. Label
  each Scale radio group and the relative Time Format radio group explicitly.
- [x] Use Relative / Absolute RF or Absolute UTC consistently in the panel and
  both main/ruler menus.
- [x] Route menu command rows through the shared command/keycap wrapper. Resolve
  explicit scale commands to their canonical toggle shortcut for presentation,
  preserving explicit selection behavior and avoiding duplicated key strings.
- [x] Verify pointer/keyboard, alignment, frame consistency and shortcut labels
  in automated tests and the current production application under isolated Xvfb.

Approved follow-up policy:

- Use the standard DatePicker with UTC date/time at Second precision and its built-in clear. Actual user changes commit immediately through the standard Change event. Opening or dismissing an unchanged picker preserves the exact internal origin and history. No separate millisecond field or footer buttons.
- Embedded recording start wins. Otherwise use filesystem creation time as the initial effective clean origin, obtained from the same stable opened file. Unsupported birthtime remains unset, without modification/change-time or current-clock substitution.
- Source metadata remains the embedded metadata for identity/reopen checks. Initial history carries the effective default. Save rebinding and private conversion do not reapply birthtime, so explicit clear remains cleared in the current document. Serialization remains nearest millisecond.

Focused regressions pass for the real corner center/containment at 800/640 widths
in both orientations, equal field frame edges/heights, preserved pointer/keyboard
reference stepping, actual rendered shortcut keycaps and idempotent explicit mode
selection. The existing 27 menu tests pass. Standard InputGroup supplies the
reference field's shared frame and contains the retained Input/buttons/unit,
without changing the other property editors.

The independent correction passed formatting, Clippy, all 971 tests and the release
build, followed by bounded production Xvfb checks in both orientations, at 640px
and in dark/light themes. [Follow-up evidence](../ui/265-recording-scales/followup-acceptance.md)
records exact identity, actual input/outcomes and remaining decisions. Do not commit
or push this correction before coordinator assessment.

## Review progress

The first independent review identified three accepted corrections: preserve the
precise retained origin when an unchanged millisecond editor draft closes, step
relative durations in their own units while endpoints use UTC, and retain exact
origin through private conversion backing files. Focused regressions cover panel
blur/Escape without history or analysis changes, all relative duration modes and
sample fallback, and repeated conversions followed by millisecond user-file save.
The second independent review found all three root causes corrected and no further
findings. The test-only helper extraction was separately checked as behavior
preserving. Current formatting, Clippy and the complete test suite passed: 968
tests across the application, CLI, core, DSP, editing and I/O suites, plus doc tests.
The release build passed after that gate. Bounded production pointer/keyboard
scenarios passed in isolated Xvfb, including both orientations, menus/shortcuts,
metadata Enter/blur/Escape/clear/save/reopen, normal session restart,640px and light
theme. See [the acceptance report](../ui/265-recording-scales/acceptance.md) for exact
build identity, steps, captures and limits. Owner manual acceptance remains pending,
so this plan remains active and the PR remains Draft.

Follow-up verification: final formatting and all-target Clippy pass, 976 tests plus
doc tests pass, and the current release passed bounded isolated Xvfb scenarios.
The standard DatePicker opens by pointer/Enter, commits date/time immediately,
keeps the parent open for calendar clicks outside its original bounds and closes
its popup before the parent on Escape. Built-in clear survives current-document
save rebinding. Unchanged dismissal retains `.321` and precise internal offsets.
Both orientations, locale grouping and 640px light layout were checked. See
[the follow-up report](../ui/265-recording-scales/followup-acceptance.md). Owner
manual acceptance remains pending.

## Follow-up: concise absolute ruler marks

Owner request: omit `Z` only from absolute time tick labels. UTC/date captions,
cursor and selection readouts, DatePicker and RFC3339 millisecond storage retain
their existing semantics. The private shared `utc_tick` formatter serves both
ruler orientations and every tick precision, including date-rollover prefixes.
This bounded presentation-only follow-up is class B: one private formatter and
one existing GUI axis regression, with no public API, DSP or source-safety change.
Role assignments remain unchanged. A new scoped review covers this owner request
separately from the completed previous review cycle.

- [x] Remove the suffix only in `utc_tick`, preserving date context and decimals.
- [x] Extend the existing ruler regression to reject `Z`, run the required local
  gate and current release, and inspect the affected ruler in isolated Xvfb.
- [x] Submit the exact narrow snapshot and evidence for coordinator assessment.

## Post-completion

After accepted merge, verify accepted branch-head identity, unique/unpushed/untracked
work and clean checkout before normal branch cleanup under CONTRIBUTING. No release
or tag is authorized by this Issue.
