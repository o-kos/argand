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

- [x] Replace metadata hint sections with title, current value and explanation.
- [x] Add compact duration and precise clock formatting with boundary tests.
- [x] Add the ready-state analysis timing hint and non-ready-state coverage.
- [x] Render the hierarchy and update current documentation.
- [x] Complete implementation validation and move the plan to completed before final review.

## Validation

- [x] Formatting, strict Clippy and full local tests.
- [x] Fresh release after the gate; both themes, metadata and timing hints.
- [x] Required focused external review, acting on substantive findings.

## Post-completion

Keep Draft during owner feedback; reconcile stacked bases after #50/#54 merge.
Require full remote CI for final acceptance, then squash-merge after approval.

## Validation evidence

- Formatting, strict Clippy and all 377 local tests pass; release rebuilt after
  the gate. Tests cover compact duration examples, millisecond rounding and carry,
  long durations, I/Q pair counting, current metadata values, sample semantics,
  and removal of timing hints during opening, re-analysis and failure.
- GPU-backed Wayland checks at 640x400 in both themes cover container, samples,
  rate, optional centre frequency, signal duration and completed-analysis hints.
  A 30.456-second capture displays `30.456s` / `0:00:30.456`; a 4350-second capture
  displays `1h12m30s` / `1:12:30.000`. Descriptions and the timing hint fit inside
  the window and remain smaller/muted; values retain the heading's text size.
- Timing uses the existing worker measurement; no claim is made about total
  file opening, GPU upload or first visible frame time.
- Runtime validation is Linux Wayland; Windows/macOS runtime was not exercised.
- External review returned no substantive findings; none were declined.

## Owner feedback: one-sentence explanations

- [x] Express each explanatory hint as one concise sentence without a terminal
  period or forced line break; retain separate heading/value and `hms.ms` notation.
- [x] Preserve format and timing semantics, run the local gate, rebuild release
  and obtain focused external review before presenting the result.

Validation: formatting, strict Clippy and all 377 tests pass, and release was
rebuilt after the gate. The focused wording review returned no substantive
findings; none were declined. This iteration changes prose only, retaining the
previously verified tooltip layout and timing lifecycle.

## Regression: wrapped hint text exceeds its height

- [ ] Reproduce the reported vertical clipping with the current release and
  identify the layout/measurement mismatch.
- [ ] Make wrapped text contribute its full height without restoring manual
  sentence breaks or uniform hint widths.
- [ ] Validate the reproduced case, both themes and DPI scaling in a fresh
  release after the local gate, then complete focused external review.
