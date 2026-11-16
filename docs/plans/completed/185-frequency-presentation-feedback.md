# Issue #185: Frequency presentation feedback

Resolves [#185](https://github.com/o-kos/argand/issues/185).

## Overview

Address owner feedback on the Draft implementation: keep Reference frequency on
one line, and show a selected band's width in independently chosen Hz/kHz/MHz
instead of a small fraction of the absolute frequency ruler's unit.

## Roles and risk

- Class A: the shared formatting, input owners and regressions touch more than
  five code files; no DSP, source metadata or selection-coordinate changes.
- Primary: `gpt-6.1-sol` medium.
- Consultant: none.
- Reviewer: `gpt-6.1-sol` high in a separate read-only context.

## Decisions

- Keep the full Reference frequency label. Give the shared property label column
  enough font-relative space, with the numeric-column budget using the same size.
- The owner chose automatic Hz/kHz/MHz for band width, with an explicit suffix.
  Band edges continue using the ruler's unit and physical/baseband mode.
- Width badge, badge input and selection-panel width use the same text. Bare
  width input and stepping use its displayed unit; explicit units are accepted.
- Preserve pixel-derived precision, numeric locale, selected bins, and existing
  Enter/Escape/blur behavior. No toolkit, dependency or resource-layout changes.

## Acceptance criteria and evidence

| Criterion | Verification | Result / limits |
| --- | --- | --- |
| Reference frequency fits one line without overlapping its numeric control | Production File panel at ordinary and narrow sizes; compare Analysis panel | Passed |
| A 1580 Hz band at 10 MHz reads 1.58 kHz with unchanged edge units | Formatting and round-trip tests; production badge/panel | Passed |
| Width input and stepping preserve physical bandwidth in each unit | Hz/kHz/MHz, explicit/bare input, locale, offsets and boundary tests | Passed |
| No neighboring property interaction regresses | Existing tests and bounded production input/select checks | Passed |

## Implementation steps

- [x] Correct the shared property label/numeric space budget.
- [x] Format and parse width independently, including last-digit stepping.
- [x] Add meaningful numeric regressions and update architecture/changelog.
- [x] Complete the full local gate, production Xvfb acceptance and independent review.
- [x] Move this plan to completed before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, last
- [x] Production application on a task-owned Xvfb display, with isolated settings.

## Recorded results

- Full local gate passed; 899 tests passed. Release was built last from the
  validated code, using the existing toolchain, toolkit graph and local target.
- Production Linux release verified on an isolated, task-owned Xvfb display:
  File information at 1100x800 and 700x600; one-line label, full fractional
  14074000.25 Hz entry and commit; unchanged sample-rate entry and commit;
  Analysis settings layout and FFT select open/dismiss.
- Entered 10.09917 / 10.10075 MHz edges: badge and selection-panel width read
  1.58 kHz. Entered bare 0.5 and pressed Up: width became 600 Hz.
- Font-size 16/24 geometry and full-value fit are covered by an application
  layout test. Enlarged-font production interaction, Windows and macOS were
  not exercised in this feedback pass.
- Independent review by `gpt-6.1-sol` high completed in three rounds. Accepted
  findings fixed pending width-unit binding before stepping, font-relative
  panel width, and the geometry test's actual input identity/full-value budget.
  No findings were rejected; final round had no substantive findings.
- The primary assessed the final code against the reviewed snapshot and these
  results. Physical selection coordinates, DSP and capture ownership are
  unchanged. Only completion evidence and plan location changed after review.

## Post-completion

Keep the Pull Request Draft for owner feedback; no merge or release is authorized.
