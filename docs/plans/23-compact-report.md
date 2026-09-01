# Issue #23: Make the CLI report compact and free of repetition

Resolves #23.

## Overview

Replace the 12-line labelled block the CLI prints for one file with two named
sections: the input, carrying what was measured in the signal, and the render,
carrying what the picture shows and the transform that drew it. Facts are
indented two spaces under a header ending in `:`, separated by commas, and
printed once each. The batch line keeps its shape but adopts the same field
names, order and units, so the two report shapes stop disagreeing.

The change is limited to what the CLI prints and to `format_samples`. It does
not change the JSON report's field names or structure, the analysis, the
rendered image, or any option's meaning.

## Context

- `Report::write_human` prints a 12-column label gutter, a blank separator, and
  repeats the input name, the output path, the range recommendation and the
  `/32768` divisor. `Report::write_compact` prints one line per file with `·`
  separators and a different field set.
- `Reporting::new` currently echoes the render path on stdout for every single
  file and suppresses it for a batch on a terminal. Issue #23 makes the rule
  depend on the stream alone.
- `-v` today means "the full block, in a batch too" and raises the tracing
  level. It gains a second job: restoring the detail the default report drops.
- `format_samples` emits `43.2Mspl` with no space before the unit. It is used
  only by the report's length field, which moves under `-v`.
- The image footer's `(sugg -d N)` is drawn by `render`, measured against the
  footer's fitted width, and covered by render tests. It is a different medium
  from the console report and stays as it is.

## Decisions

- Print six lines for a single input: two sections of a header plus two facts.
  The acceptance criterion's "at most five lines" contradicts the issue's own
  worked example; the owner confirmed the example is normative, so the
  criterion is corrected rather than the shape.
- Format elapsed time with `format_duration`, the workspace's one duration
  rule, and drop the `in ` prefix that made a single file read differently from
  a batch. The example's `1.2s` would need a second duration formatter in
  `argand-core` for one field; owner confirmed reusing `format_duration`, so a
  single file reads `1.246s` where the example wrote `1.2s`.
- Head the render section with the output's own name when it is written beside
  its input, and with the full path otherwise. `-o` may point anywhere, and a
  bare `spec.png` would then name a file the caller cannot find.
- Give the compact line both `peak` and `bin`, in the block's order and units.
  The block names the time-domain sample peak `peak` and the spectral peak
  `bin`; today's compact line calls the spectral peak `peak`, so consistency
  alone would silently drop the level a batch is usually read for.
- Keep the suggestion before the arrow on a compact line. In the block it sits
  beside the range it argues with; a compact line has no range field, and
  putting it after the render's size and timing hides the one actionable field.
- Move the divisor to the input's own line as `full scale 32768` under `-v`,
  since it is one property of the file rather than of each level. Name it when
  it is large enough to be a count, which is the predicate the per-level form
  already used: an integer format brings its own full scale and `--normalize`
  measures one for a float capture that was never scaled to `[-1, 1]`, and
  both are worth naming. A divisor near one is not a count and is left out,
  and so is one the caller gave to `--normalize`: that line already prints
  the number, and repeating it is the defect this shape exists to remove.
- Keep `Report::range_suggestion` for the image footer and add a separate
  console phrasing. One `suggested_range_db` stays the single source of the
  number, so the two media cannot disagree about whether or what to suggest.
- Decide the stderr shape with a `Detail` enum on `StderrBlock` rather than a
  verbose flag threaded through the writer, so the three modes stay a lookup.

## Rejected alternatives

- Fold the render section into one line to satisfy the five-line criterion.
  It produces a line long enough to wrap on a normal terminal, which costs the
  line back and breaks the alignment the sections exist for.
- Keep the label gutter and only remove the duplicated values. The gutter is
  what forces every field to a fixed width and is a third of the problem the
  issue lists.
- Add `format_elapsed` to `argand-core` to reproduce the example's `1.2s`.
  A second duration formatter for one field contradicts the criterion that
  elapsed time uses one formatting rule.
- Change the image footer's `sugg -d N` to the console's wording. The footer is
  fitted against a measured width and is not part of the report this issue
  governs.
- Cover `std::io::stdout().is_terminal()` itself with a pty test. External
  review is right that `portable-pty` would make this portable to the Windows
  job, so this is a dependency trade-off and not an impossibility: a dev
  dependency and its transitive graph, for one boolean call whose two outcomes
  are already unit-tested through `Reporting::for_stdout` and whose pipe side
  every end-to-end test exercises. The terminal side is validated by hand from
  the release binary instead. Worth revisiting if a second test ever needs a
  pty.

## Implementation steps

- [x] Put a space before the unit in `format_samples` and update its tests.
- [x] Rewrite the human report as two indented sections with comma-separated
  fields, a default and a verbose detail level, and no blank lines; cover the
  line count, the absence of every repeat the issue lists, and the empty
  spectrum.
- [x] Bring the compact line to the same field names, order and units, and
  convert the batch summary to commas.
- [x] Echo the render path on stdout only when stdout is not a terminal, and
  select the stderr shape from quiet, verbose and batch in one place.
- [x] Update `--help`, README and the Unreleased changelog entry.
- [x] ➕ Correct the unreleased changelog entry for Issue #5, which described
  the report field this branch replaced before either had been released.
- [x] ➕ Make the terminal-versus-pipe decision testable by passing the one
  environment fact into `Reporting`, and record the pty dependency trade-off
  rather than covering the `is_terminal` call itself.
- [x] ➕ Head a render sent outside its input's directory with an absolute
  path, so a relative `-o` cannot read as a render written beside its input.
- [x] ➕ Correct the `--help` sentence on `--json` and `-q`, which had `--json`
  replacing the stderr report and `-q` silencing the JSON.
- [x] ➕ Say in the README which fields the compact line drops and that
  `--json` has an object only for a file that rendered.
- [x] ➕ Compare resolved directories, not the paths as typed, when deciding
  whether a render sits beside its input.
- [ ] Complete validation, including a real terminal and a pipe.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Run the release binary on a real capture with stdout on a terminal and
  again through a pipe, and confirm the path appears only in the second case.
- [ ] Compare the default, `-v`, batch and `-q` shapes against the issue's
  acceptance criteria from that binary.

## Post-completion

After owner acceptance and green required checks, squash-merge the Pull
Request, switch the main working directory to `main`, and delete
`feature/23-compact-report` remotely and locally after the required safety
checks. Correct the "at most five lines" acceptance criterion in Issue #23 so
the closed Issue matches what shipped.
