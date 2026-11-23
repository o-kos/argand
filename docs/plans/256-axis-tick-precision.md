# Issue #256: Consistent frequency tick precision

Resolves #256.

## Overview

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
