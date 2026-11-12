# Issue #185: Frequency presentation feedback

Resolves [#185](https://github.com/o-kos/argand/issues/185).

## Overview

Address owner feedback on the Draft implementation: keep Reference frequency on
one line, and show a selected band's width in independently chosen Hz/kHz/MHz
instead of a small fraction of the absolute frequency ruler's unit.

## Roles and risk

- Class B: local property layout and selection number formatting/parsing; no DSP,
  source metadata or selection-coordinate changes.
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
| Reference frequency fits one line without overlapping its numeric control | Production File panel at ordinary and narrow sizes; compare Analysis panel | Pending |
| A 1580 Hz band at 10 MHz reads 1.58 kHz with unchanged edge units | Formatting and round-trip tests; production badge/panel | Pending |
| Width input and stepping preserve physical bandwidth in each unit | Hz/kHz/MHz, explicit/bare input, locale, offsets and boundary tests | Pending |
| No neighboring property interaction regresses | Existing tests and bounded production input/select checks | Pending |

## Implementation steps

- [ ] Correct the shared property label/numeric space budget.
- [ ] Format and parse width independently, including last-digit stepping.
- [ ] Add meaningful numeric regressions and update architecture/changelog.
- [ ] Complete the full local gate, production Xvfb acceptance and independent review.
- [ ] Move this plan to completed before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, last
- [ ] Production application on a task-owned Xvfb display, with isolated settings.

## Post-completion

Keep the Pull Request Draft for owner feedback; no merge or release is authorized.
