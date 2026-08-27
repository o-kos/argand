# Issue #4: Process multiple files with non-recursive input masks

Resolves #4.

## Overview

`aspec` accepts exactly one input path, so processing a directory of captures needs an
external shell loop. That loop behaves differently across shells, repeats the full
multi-line report for every file, and cannot apply one consistent set of raw format
parameters without repeating them by hand.

This change accepts several inputs, each either an exact path or a non-recursive mask
over the filenames in one directory, and turns the single-file run into a batch that
keeps going after an individual failure.

## Context

- `crates/cli/src/main.rs` is linear today: open, analyse, render, report. Everything
  between `argand_io::open` and the report has to become a per-file unit, with a driver
  around it.
- `Args::input` is one `PathBuf`. `--output` names one PNG, which cannot survive a batch.
- The report is built per file and printed two ways: the path on stdout, the multi-line
  block on stderr, or pretty JSON on stdout under `--json`.
- The STFT progress bar is per file and is cleared when the file finishes.
- `IoError` in `crates/io/src/lib.rs` interpolates `{source}` into three of its own
  messages while the same field carries `#[source]`. Printing with anyhow's `{:#}` walks
  the chain and appends the source again, and `main.rs` adds a context line that repeats
  the path the error already names. One missing file currently reads:
  `opening X: cannot open X: No such file (os error 2): No such file (os error 2)`.

## Decisions

- The doubled error text is fixed first, before any batch work. A batch gives each file
  one compact line, and a message that repeats itself three times does not fit on one.
- The mask matcher is written here rather than taken from the `glob` crate. What is
  wanted is one filename component with `*`, `?` and `[...]`; `glob` also walks
  directories and understands `**`, which would then have to be suppressed to keep the
  Issue's semantics. `argand` hand-writes its RIFF parser and colour ramps for the same
  reason.
- An argument with no metacharacters is an exact path and is not checked for existence
  during resolution. It fails when opened, like any other file, which is what lets the
  batch continue past it and what keeps the single-file exit status as it is today.
- A mask that matches nothing is a resolution error naming the mask. This is the one
  case the Issue calls out, and it cannot be reported per file because there is no file.
- `*` does not match a leading `.`, following the Unix convention, so a mask cannot
  sweep up dotfiles by accident.
- Matches sort by filename and are deduplicated by canonical path, falling back to the
  literal path when canonicalization fails.
- `--json` keeps printing one pretty object per file, whatever the file count. That
  holds the Issue's "single-file input retains its current behavior" criterion exactly
  and leaves one code path. Concatenated pretty objects still stream: `jq` and
  `serde_json::StreamDeserializer` both read a whitespace-insensitive sequence of JSON
  values.
- The per-file STFT progress bar stays and gains an `[n/N]` file counter. A batch of one
  long capture still needs to show that it is alive.
- The batch summary goes to stderr even under `--json`, so stdout carries nothing but
  report objects.
- A per-file failure line is printed even under `--quiet`, which is what a single file
  already does: `--quiet` suppresses the report, never the reason a file produced none.
- The batch output modes apply when more than one file resolves. One file keeps today's
  output byte for byte, including having no summary line after it.

## Rejected alternatives

- The `glob` crate. See the decision above.
- Compact JSON Lines, as the Issue's text proposes. It would serve line-oriented tools
  (`while read`, `json.loads` per line) but changes single-file output from pretty to
  compact, contradicting the Issue's own first acceptance criterion. If line-oriented
  batch parsing is wanted later it belongs behind its own flag.
- Hiding the progress bar during a batch, or replacing it with one bar counting files.
  Either reads better for many small captures and worse for the case the tool is built
  for, which is a small number of very long ones.
- Checking exact paths for existence during resolution. It would report a typo sooner,
  but it splits "this file is unusable" across two unrelated code paths.

## Implementation steps

- [x] Stop `IoError` from printing its source twice, and drop the `main.rs` context line
      that repeats the path.
- [x] ➕ Apply the same fix to `SourceError::Io`, `DspError::Source` and
      `ParseRawSpecError`, which repeat their own source the same way and sit in the
      same chain, so fixing `IoError` alone would still print it twice.
- [x] Add a filename mask matcher supporting `*`, `?` and `[...]` with ranges and
      negation, rejecting `**`.
- [x] Add input resolution: exact paths pass through, masks list one directory, results
      sort by filename and deduplicate, zero matches and directory-component
      metacharacters are errors.
- [x] Accept several inputs on the command line and reject `--output` unless exactly one
      file resolves.
- [x] Split the run into a per-file unit and a batch driver that continues past a
      failure and exits non-zero if any file failed.
- [x] Give the batch its output modes: a compact line per file, the full report under
      `-v`, silence under `--quiet`, one pretty JSON object per file under `--json`, and
      a processed/succeeded/failed/elapsed summary on stderr.
- [x] Add the `[n/N]` file counter to the progress bar.
- [x] Update `README.md` and the CLI help and examples.
- [ ] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use ➕ for tasks discovered after implementation begins and ⚠️ for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked -- -D warnings`
- [x] `cargo test --locked`: 218 tests, all passing.
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Matcher tests cover `*`, `?`, ranges, negation, the leading-dot rule and rejected
      patterns.
- [x] Resolver tests cover exact paths, masks, sorting, deduplication, zero matches and
      metacharacters in a directory component.
- [x] End-to-end tests cover a batch, a batch with one failing file and its exit status,
      `-o` rejected for a batch, every output mode, and the error text that no longer
      repeats itself.
- [x] Run a real batch over `tests/signals/` and read the summary. Four masks over the
      twelve captures there, including the two 30-minute I/Q files and two names holding
      spaces, rendered in 1.26 s with `processed 12 · 12 succeeded · 0 failed`.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
