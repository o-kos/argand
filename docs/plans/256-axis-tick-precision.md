# Issue #256: Consistent precision-aware frequency presentation

Resolves #256.

## Current scope

The owner expanded this Issue after the initial tick-padding implementation.
Apply one precision-aware frequency unit policy to every human-facing frequency
consumer. A unit requiring more than three fractional digits moves to the next
smaller unit: `12.582 MHz` stays, `12.5825 MHz` becomes `12 582.5 kHz`, and
`12.58255 MHz` becomes `12 582.55 kHz`. GUI grouping and decimal marks follow
the numeric locale. Physical Hz values and machine-readable contracts stay intact.

The consumer inventory and API below are the pre-implementation plan for this
expanded scope. The prior implementation, evidence and review record below cover
only the initial scope. They do not establish validation of the expanded scope.

## Initial scope

Frequency tick labels independently trim fractional zeros. Use one precision derived
from the displayed unit and selected tick step for every fractional frequency tick,
while retaining bare integer labels. Preserve numeric coordinates, units, locale,
clock/seconds/sample rulers and standalone frequency readouts.

## Roles and risk

- Class: A, conservatively treating shared public core axis presentation as a core
  API behavior change. No DSP, concurrency, storage or resource ownership changes.
- Coordinator: `gpt-6.1-sol` medium, owner communication and exact-result assessment.
- Executor: `gpt-6.1-sol` medium, investigation through validation and PR operations.
- Reviewer: `gpt-6.1-sol` high in a fresh read-only context.
- Consultant: none unless a concrete unresolved question requires one.

## Context and decisions

`crates/core/src/axis.rs` owns shared GUI/CLI tick selection and formatting.
`HertzUnit::format` currently trims all fractional zeros. Tick placement knows the
selected or retained scheme before localization and measurement. Add private
step-aware tick formatting there without changing public signatures or the
standalone frequency readout contract. Keep the existing unit resolution limit.
GUI localization must preserve the padded fraction before final text measurement.
Reuse the existing axis canvas and number formatter; introduce no controls.

Rejected: padding labels after placement would invalidate measured widths and tick
spacing. Changing clock/seconds/sample formatting expands the Issue's scope.

## Acceptance criteria and evidence

| Criterion | Verification | Result / limits |
| --- | --- | --- |
| Fractional ticks share step precision, integers stay bare | Shared axis tests for MHz `12.589`, `12.590`, `12.900`, `13`, other units, negative and zero | Automated pass, production rendering observed |
| Zoom and panning retain appropriate precision and collision safety | Automatic/held scheme tests and isolated production UI zoom/pan | Automated pass, interactive zoom/pan incomplete |
| Locale, values and units remain correct | GUI localization tests and decimal-comma production rendering | Automated and production rendering pass |
| Other rulers/readouts retain behavior | Existing core/GUI suites and focused regression assertions | Existing suites passed |

UI acceptance uses the repository skill, only on an isolated Xvfb display, with
supported disposable configuration/session/cache isolation. Inspect available
interaction capabilities before running. Missing display or authorized input leaves
acceptance incomplete and the PR Draft. A screenshot alone is not acceptance.

## Implementation steps

- [x] Confirm root cause and record scope, roles, plan and concrete expectations.
- [x] Add regression tests and private shared frequency tick formatting.
- [x] Update changelog and current axis architecture contract.
- [x] Complete full local gate and build current production release.
- [ ] ⚠️ Exercise production zoom, pan and live orientation switching in Xvfb.
  Static current-production rendering passed in both orientations and across an
  integer frequency. Authorized interactive input was unavailable.
- [x] Complete independent review and dispositions.
- [ ] Move plan to completed before final review when acceptance is complete.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks pass
- [x] Focused core and locale regressions
- [ ] Current-production UI scenario with recorded build identity and limits

## Post-completion

Merge and safe worktree/branch cleanup follow owner acceptance and current full CI.

### Automated results

The focused regression reproduced `12.59` versus required `12.590` before the fix.
The locked workspace gate passed formatting, all-target Clippy, 958 tests and doc
tests. Core regressions cover step-derived precision in Hz/kHz/MHz/GHz, bare
integers, negative values/zero and retained schemes; GUI number tests cover
English and decimal-comma Russian localization. Existing clock/seconds/sample
ruler and standalone readout tests passed unchanged. The selected immutable SDK
and workspace-local `target/` were used without toolkit/toolchain changes.

### Independent review dispositions

Round 1 accepted one P2 finding: rounding a non-ladder retained step of 0.9996 Hz
before choosing precision shortened the labels to whole Hz. The first correction selected
the smallest decimal count representing the scaled step within floating-point
error, falling back to the existing unit resolution cap. The
regression expects `99.960` and `100.960` for that retained scheme. No findings
were rejected. The subsequent round is recorded below.

Round 2 accepted one P2 finding: a relative floating-point tolerance could treat a
large genuinely fractional GHz step as an integer. Removing tolerance alone is
insufficient because multiplication can also discard the fraction. Precision now
requires an exactly integral scaled candidate and textual equality between the
candidate padded to the unit cap and the original full-cap step text. Otherwise
it retains the cap. Regressions cover both reported large-step cases alongside
the 0.9996 Hz case. No findings were rejected. Round 3 is the final allowed round.

### Final evidence and remaining acceptance

The final source passed the refreshed full gate, 958 tests and doc tests, then
production release build. Direct Xvfb rendering confirmed decimal-comma fractional
padding in horizontal/dark and vertical/light layouts, and bare `13` MHz beside
fractional ticks. [Production evidence and limits](../ui/256-axis-tick-precision/README.md)
record exact source/binary identity, startup isolation, screenshots and cleanup.
Interactive zoom, pan and live orientation switching were not exercised because
available native input was disabled and no installed input utility was available.
This is incomplete acceptance, not a successful interactive scenario. The PR
remains Draft and the plan stays active.

Round 3 returned no substantive findings. Both accepted P2 findings were fixed,
none were rejected, and the final source was frozen during review. The final
evidence/plan update changes documentation only.

## Expanded scope: consumer inventory

| Consumer | Current owner | Precision and state contract |
| --- | --- | --- |
| GUI frequency ticks/caption, both orientations | core `axis`, app `axes` | Selected step and accepted measured text, retained unit while panning |
| CLI spectrogram and PSD ticks/captions/gutters | CLI `render`, core `axis` | Same core policy and final measured text, canonical ASCII numbers |
| Cursor status and Alt frequency badge | app `axes/cursor_guides` | Device-pixel precision, shared readout and width-cache key |
| Selection edge/width badges, inline inputs, panel | app `navigation/selection_entry`, `shell/badge_entry`, `shell/plot` | Edges retain physical offset translation, width independent of absolute origin, explicit suffix overrides displayed default unit |
| File/status sample rate, declared reference and hints | app `document` | Fixed-value context and existing measurement resolution, independent of viewport |
| Effective-reference status and property editor | app `shell/reference_ui` | Whole-Hz commit range and 1 Hz steps, unchanged draft preserves original Hz |
| Conversion sample-rate and carrier editors | app `shell/conversion_ui`, shared `NumericEditor` | Physical Hz bounds/step, existing draft transaction and carrier meaning, unchanged text cannot round-trip through a rounded display |
| Conversion retained-band warning | app `shell/conversion_ui` | One common unit for range endpoints, source-local frequency semantics |
| Saving rate warning | app `shell/saving_ui` | Human frequency policy, no save semantics change |
| Ruler resolution hint | app `axes::UnitHint` | Frequency-per-device-pixel quantity with its own resolution context |
| Human CLI source/title/footer/ENBW/peak offset and absolute peak | core `format_hz`, CLI `report` | Existing report resolution, shared policy, no GUI locale dependency |
| Suggested extraction filename frequency portion | app `processing/saving::band_label` | Shared unit/precision rule, portable ASCII and no locale grouping |
| Source filenames, CLI argument syntax, JSON, session and file metadata | Existing I/O/CLI/session owners | Physical Hz/identity contracts, do not rewrite machine values or source names |

## Expanded scope: architecture and invariants

1. Add toolkit-neutral `argand_core::frequency::{FrequencyUnit, FrequencyFormat}`.
   Keep fields private and expose unit/divisor/decimals readers, canonical numeric
   formatting, formatting with a unit and unit-suffix recognition. Constructors
   distinguish step/resolution contexts from fixed-value contexts. Fixed values
   select units from their actual displayed significant digits at the existing
   agreed resolution, not from the maximum precision cap or binary noise.
2. Axis candidate formatting obtains its frequency format before localization and
   measurement. Store the accepted format with the retained tick scheme so panning
   cannot switch units at a magnitude threshold. Adapt the evolving public
   `TickScheme` with `#[non_exhaustive]`, a constructor and a format reader.
   GUI and CLI captions derive from the accepted scheme. Conservative pre-layout
   bounds must account for every unit the policy can select.
3. Keep reference/sample-rate/carrier values, bounds and steps in physical Hz.
   Extend the existing standard Input/Button NumericEditor composition with
   frequency presentation. Callers use Hz value/set-value methods rather than
   reinterpreting InputState text. Bind a focused draft to its displayed unit.
   Update unit and committed display together. Preserve original canonical Hz
   when unchanged displayed text is committed, and retain the established
   preview/keep/revert, stale-document, disabled, focus and stepping contracts.
4. Keep explicit suffixes authoritative. Locale-aware number parsing remains in
   the app; SI unit conversion/presentation policy remains in core. Preserve
   reference-offset translation for edges and avoid applying offsets to widths.
5. Formatting never schedules signal analysis, changes samples, source identity,
   saved physical metadata, capture/view coordinates or persistence schema.
   Zoom/size/orientation can select fresh presentation; panning retains its unit.
   Numeric locale remains immutable and final strings enter measurement/cache keys.
6. Reuse existing axis canvases and shared standard numeric controls. No new widget,
   toolkit graph, feature, toolchain, worktree or shared build artifacts are needed.

### Product decision pending

At 24 kHz over 600 pixels, a 1 kHz tick step needs three MHz fractional digits,
but a pixel-resolution cursor needs approximately five. The owner is choosing
between one viewport-family unit based on the finest required resolution, or
consumer-specific units chosen through the same policy. A common viewport format
must be independent of whether the cursor is currently visible. No dependent
unit-flow implementation begins before this choice is recorded.

At the Hz floor, preserve the consumer's existing precision when no smaller
supported SI unit exists rather than discard physical information. Fixed metadata
has no viewport dependency. Editable values require canonical dirty tracking to
avoid accidental loss from a rounded untouched display.

## Expanded scope: acceptance and implementation

- [x] Inventory every human frequency consumer and machine-contract exclusion.
- [x] Record the owner-approved three-digit unit examples and role/risk assignment.
- [ ] Record the owner's viewport-family unit decision.
- [ ] Implement the shared immutable frequency presentation policy and value cases.
- [ ] Integrate accepted/retained axis formats and GUI/CLI captions/measurements.
- [ ] Integrate cursor/status/Alt and all selection display/input consumers.
- [ ] Integrate fixed metadata, shared frequency editors, hints, warnings and human CLI.
- [ ] Synchronize current architecture, changelog, Issue and PR scope/evidence.
- [ ] Refresh fmt, all-target Clippy, locked tests and then production release build.
- [ ] Complete the new expanded-scope independent review under the same role defaults.
- [ ] Exercise production scenarios in isolated Xvfb and record remaining limits.

Tests must independently establish the approved examples, unit transitions,
negative/zero/integer values, retained-unit stability across magnitude thresholds,
locale grouping, cursor/selection consistency under the chosen family rule,
explicit suffix interpretation, selection width/reference-offset separation,
1 Hz stepping and unchanged canonical values in both property panels. Preserve
coverage of the earlier non-ladder and huge-step review regressions. Human CLI
text/image changes need focused output and measurement tests; JSON stays unchanged.

Current-production acceptance must cover zoom/pan and both orientations, pointer
readouts and Alt badges, edge/width input commit/step, property-panel commit/step
and dismiss transactions, and narrow light/dark layouts where longer grouped
labels matter. Use only isolated Xvfb and permitted input. Unavailable interaction
remains incomplete and keeps the PR Draft. Earlier screenshots do not validate
this expanded scope.

### Expanded-scope review boundary

The earlier three-round review finished clean for its exact initial snapshot.
The owner subsequently approved new functionality. A new review cycle covers the
new plan and exact expanded snapshot with the same reviewer assignment and a
maximum of three rounds, retaining previous findings and dispositions as context.
This is not an additional attempt to bypass unresolved earlier findings.
