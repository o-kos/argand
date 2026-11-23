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
| Fractional ticks share step precision, integers stay bare | Shared axis tests for MHz `12.589`, `12.590`, `12.900`, `13`, other units, negative and zero | Pending |
| Zoom and panning retain appropriate precision and collision safety | Automatic/held scheme tests and isolated production UI zoom/pan | Pending |
| Locale, values and units remain correct | GUI localization tests and decimal-comma production rendering | Pending |
| Other rulers/readouts retain behavior | Existing core/GUI suites and focused regression assertions | Pending |

UI acceptance uses the repository skill, only on an isolated Xvfb display, with
supported disposable configuration/session/cache isolation. Inspect available
interaction capabilities before running. Missing display or authorized input leaves
acceptance incomplete and the PR Draft. A screenshot alone is not acceptance.

## Implementation steps

- [x] Confirm root cause and record scope, roles, plan and concrete expectations.
- [x] Add regression tests and private shared frequency tick formatting.
- [x] Update changelog and current axis architecture contract.
- [ ] Complete full local gate and build current production release.
- [ ] Exercise production frequency rulers, zoom, pan and both orientations in Xvfb.
- [ ] Complete independent review and dispositions.
- [ ] Move plan to completed before final review when acceptance is complete.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks pass
- [x] Focused core and locale regressions
- [ ] Current-production UI scenario with recorded build identity and limits

## Post-completion

Merge and safe worktree/branch cleanup follow owner acceptance and current full CI.

### Automated results

The focused regression reproduced `12.59` versus required `12.590` before the fix.
The locked workspace gate passed formatting, all-target Clippy, 957 tests and doc
tests. Core regressions cover step-derived precision in Hz/kHz/MHz/GHz, bare
integers, negative values/zero and retained schemes; GUI number tests cover
English and decimal-comma Russian localization. Existing clock/seconds/sample
ruler and standalone readout tests passed unchanged. The selected immutable SDK
and workspace-local `target/` were used without toolkit/toolchain changes.

### Independent review dispositions

Round 1 accepted one P2 finding: rounding a non-ladder retained step of 0.9996 Hz
before choosing precision shortened the labels to whole Hz. Precision now selects
the smallest decimal count that represents the original scaled step within
floating-point error, falling back to the existing unit resolution cap. The
regression expects `99.960` and `100.960` for that retained scheme. No findings
were rejected. The corrected snapshot and refreshed validation require round 2.
