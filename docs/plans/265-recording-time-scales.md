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
- Never infer recording start from filesystem timestamps. Missing, malformed or
  unsupported metadata does not fabricate absolute time or prevent valid sample
  data from opening. Samples, true rate and domain are preserved by metadata edits.
- Reference remains the physical frequency of baseband zero. Presentation changes
  retain the stored reference, including zero, with real one-sided and complex
  two-sided spectra unchanged.

### Approved controls and presentation preferences

- View / Time scale format offers Relative and Absolute UTC. The existing relative
  formats hms, seconds and samples remain available and remembered independently.
- View / Frequency scale offers Baseband and Physical RF.
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
- Include recording-start editing in that same panel. No Apply, Cancel, Clear or
  close buttons are added. Clearing uses an empty recording-start field. Follow
  the existing panel's Enter/blur commit and Escape uncommitted-draft cancellation
  conventions, mutual exclusion, external-tooltip and focus policies.
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
| Set, edit, clear and reopen start with reference | WAVE/native and decoder-backed layouts, RF64 where supported, FLAC DATE round trips; compare samples, count, rate and domain | Pending | No implementation or Rust checks yet |
| Safe invalid and absent metadata | Zero/short/malformed SYSTEMTIME, invalid dates, incomplete DATE, unsupported precision/zone and conflicting tags | Pending | UTC import interpretation approved |
| Correct UTC labels and entries | Start before midnight, fractional seconds, selection endpoints/duration, zoom and both orientations; absolute entry maps to original sample span | Pending | Nearest-millisecond storage rounding approved |
| Presentation leaves data and analysis intact | Toggle each command entry point, inspect selection/view/reference and analysis request count/generation | Pending | Automated and production evidence required |
| Persistent and consistent modes | Isolated session normal restart, unavailable Absolute, files with zero/missing reference, checked menu and radio state, corner frequency/time label | Pending | Existing hms/seconds/samples retained |
| Metadata history and file safety | No-op/Undo/Redo/Discard, whole and selection saves, RAW Save As, stale save completion, source protection and cancellation regression cases | Pending | Existing publication guards must remain unchanged |
| Processing and timeline semantics | Agreed prefix/tail/interior/paste/extraction/conversion scenarios and undo/save/reopen | Pending | Approved timeline table above |
| Production UI behavior and appearance | Current release binary in isolated Xvfb, both corner placements, pointer and keyboard, radios, grouped frequency input, timestamp edit/empty clear, Enter/blur/Escape, menus/shortcuts, busy and unavailable states | Pending | Owner desktop untouched; Xvfb does not prove native Wayland/other OS behavior |
| Current revision independently reviewed | Neutral exact handoff, fresh read-only reviewer, dispositions and clean final round or explicit remaining findings | Pending | Review does not replace owner UI acceptance |

## Implementation steps

- [x] Resolve and record pending owner decisions before product edits.
- [ ] Verify installed toolchain selection without triggering a proxy download,
  existing toolkit/SDK selection and workspace-local target setup before Rust work.
- [ ] Add validated recording-start value and metadata propagation at core/I/O seams.
- [ ] Implement auxi/DATE reading and writing, invalid-input handling and round trips.
- [ ] Version metadata and apply agreed save, processing and timeline semantics.
- [ ] Add shared relative/absolute presentation, endpoint entry and session persistence.
- [ ] Implement approved corner button/common panel and synchronized menu/shortcut paths.
- [ ] Update metadata, navigation, UI and session architecture documentation and changelog.
- [ ] Complete targeted tests, full local gate, production acceptance and independent review.
- [ ] Move plan to `docs/plans/completed/`, rebuild current release and request Ready/full CI
  only after all agreed implementation and acceptance work is complete.

## Validation

For the initial plan-only commit, review the diff and local links. No product code,
Rust build or test is claimed. The complete code-change gate applies before code
pushes; initial plan publication follows the coordinator's scoped workflow decision.

- [ ] Targeted timestamp, I/O, history, presentation, inverse-entry and session tests.
- [ ] Relevant production-control GPUI interaction regressions with normal cleanup.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass.
- [ ] Isolated production Xvfb acceptance report with exact revision/local-diff identity,
  binary/build command, scenarios, observations and remaining limitations.
- [ ] Independent review and coordinator assessment of exact current revision.
- [ ] Current Ready PR full CI succeeds explicitly on Linux, Windows and macOS.

## Post-completion

After accepted merge, verify accepted branch-head identity, unique/unpushed/untracked
work and clean checkout before normal branch cleanup under CONTRIBUTING. No release
or tag is authorized by this Issue.
