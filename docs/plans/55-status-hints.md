# Issue #55: Status hints and compact duration

Resolves #55.

## Overview

Use heading/value/optional explanation in status hints, show signal duration
compactly, and explain the completed-analysis timing at the right of the bar.
This explicitly resumes the deferred metadata work after #53 / #54 feedback.

## Context and decisions

- Stack on #54 while parent PRs remain under review; keep recent/shortcut UI intact.
- Keep toolkit-neutral hint values and duration formatting in `document.rs`.
- Preserve the existing heading font. Values use the same size with muted colour;
  explanations use smaller muted text. No terminal periods; content-sized hints.
- Round signal duration once to milliseconds before splitting units. Omit zero
  units and trailing fractional zeros in the status. Hint uses h:mm:ss.mmm and
  the owner's `hms.ms` explanation.
- `ready in` measures the latest analysis, including sample reading but excluding
  header opening and rendering. Name its hint `Analysis time`; do not change the
  timing lifecycle or claim total file/application loading time.

## Implementation

- [ ] Replace metadata hint sections with title, current value and explanation.
- [ ] Add compact duration and precise clock formatting with boundary tests.
- [ ] Add the ready-state analysis timing hint and non-ready-state coverage.
- [ ] Render the hierarchy and update current documentation.
- [ ] Complete validation and move the plan to completed before final review.

## Validation

- [ ] Formatting, strict Clippy and full local tests.
- [ ] Fresh release after the gate; both themes, metadata and timing hints.
- [ ] Required focused external review, acting on substantive findings.

## Post-completion

Keep Draft during owner feedback; reconcile stacked bases after #50/#54 merge.
Require full remote CI for final acceptance, then squash-merge after approval.
