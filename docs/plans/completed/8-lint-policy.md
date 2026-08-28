# Issue #8: Define and enforce a workspace-wide Rust quality policy

Resolves #8.

## Overview

Clippy runs with its default lint set. That set is large and catches real bugs, but it
says almost nothing about maintainability. Of the ten lints this Issue names, one does
work by default; `excessive_nesting` is at `warn` but inert, because its threshold
defaults to zero and zero disables it; and the remaining eight are `allow` in `pedantic`,
`nursery` or `restriction`. Thresholds are implicit, so nobody can tell what the project
considers too long, too wide or too deeply nested without reading Clippy's source.

This change writes that policy into the repository, turns it on for every crate, and
fixes what it finds.

## Context

Issue #12, merged in #11, already did part of what this Issue's text asks for and this
plan does not repeat it:

- `[workspace.lints]` exists and every crate inherits it; `warnings = "deny"` makes a
  plain `cargo clippy` as strict locally as in CI;
- there are no `#[allow]` or `#[expect]` attributes left anywhere in `crates/`;
- `CONTRIBUTING.md` documents the local validation procedure and the suppression policy,
  and CI runs the same commands.

What remains is the lint *selection* and its *thresholds*, plus the code they flag.

Measured on `main` at the thresholds proposed below, `--all-targets`:

Line numbers are against `main` at `7fff1d6`, the base this branch was finally rebased
onto. The counts held across the rebase; only the positions in `render.rs` moved, because
Issue #14 reworked that file in between.

| lint | places | where |
|---|---|---|
| `excessive_nesting` | 6 | `dsp/src/stft.rs` 626, 633 (twice), 647; `dsp/src/waveform.rs` 80, 83 |
| `too_many_lines` | 2 | `dsp/src/stft.rs:211` `analyze`, 176 lines; `cli/src/render.rs:375` `Layout::compute`, 147 |
| `branches_sharing_code` | 1 | `cli/src/render.rs:698` |
| `cognitive_complexity` | 0 | — |
| `fn_params_excessive_bools`, `manual_let_else`, `redundant_else` | 0 | — |
| `allow_attributes`, `allow_attributes_without_reason` | 0 | nothing left to find; they guard against regression |

Note when measuring by hand: `warnings = "deny"` turns these into errors, and a crate
that fails to compile is not linted, so `argand-cli` findings stay hidden behind
`argand-dsp` ones unless `-W warnings` is passed to put the level back.

## Decisions

- **`excessive-nesting-threshold` is 5, not the 4 the Issue proposes.** At 4 the lint
  fires 15 times, and 6 of those are `if`/`else` used as an *expression* --
  `let src = if i < half { i + half } else { i - half };` -- because each arm counts as
  its own block. Three more are early-return guards and two are test code; only four are
  real nesting. At 5 six places remain, all genuine. At 6 nothing fires, so 6 buys
  nothing. The Issue explicitly allows adjusting a threshold with documented reasoning.
- **The other three thresholds are the Issue's proposed values**, unchanged: 7 arguments,
  100 lines, cognitive complexity 25.
- **Lints are enabled individually, never by group.** `pedantic`, `nursery` and
  `restriction` contain lints that contradict this codebase's style and each other, and
  a group grows silently on a toolchain bump.
- **Thresholds live in `clippy.toml` with a comment each**, because a bare number is not
  a policy: the next person needs to know what it is protecting against. That includes
  `max-fn-params-bools`, which review found had been left at Clippy's default: a
  threshold the repository relies on but does not state is exactly the implicitness this
  Issue exists to remove.
- **`analyze` is split by giving its streaming state a name.** Its length came from
  seven interdependent locals -- buffer, fill level, remaining, buffer origin, folded
  count -- moved in step by reading, envelope folding and carrying the overlap. A `Block`
  owns them and each of those steps is a method, which is what the code was already doing
  implicitly. Passing them as arguments instead would have traded a long function for a
  wide one.
- **`Layout::compute` is split along the `match` it already had**, one method per
  orientation, since the two arms shared nothing but the content rectangle.
- **`cognitive_complexity` is enabled even though it currently finds nothing.** Its
  value is preventing the next `Layout::compute`, not flagging today's.
- ➕ **A classifier is written as early returns, not as an `if`/`else` expression.** Rust
  allows both and no lint prefers either; `needless_return` only objects to a `return` on
  a function's last expression. What settles it here is the codebase: `inputs::expand`
  reads as a chain of guards, and non-test code contains 73 early returns. Each condition
  then stands on its own line rather than being reached through an `else`.

## Rejected alternatives

- Enabling `clippy::pedantic` and allowing what does not fit. It inverts the burden: the
  repository would carry a growing list of exceptions, and every toolchain bump would add
  work. The Issue asks for individual lints for exactly this reason.
- Lowering `cognitive-complexity-threshold` until it catches `write_report`, whose
  readability prompted the rule in `AGENTS.md`. It scores 7, against 19 for
  `Layout::compute` and 10-15 for a dozen test helpers, so no threshold reaches it
  without flagging half the repository. That shape stays a review obligation.
- `excessive-nesting-threshold = 4` as written in the Issue. See the decision above.
- Suppressing the two long functions with `#[expect(..., reason = "...")]`. The
  repository requires the owner's agreement for suppressions, and neither function has an
  argument for being irreducible: both are sequences of stages that read better named.

## Implementation steps

- [x] Add `clippy.toml` with the five thresholds, each carrying its reason.
- [x] Enable the ten lints in `[workspace.lints.clippy]`, inherited by every crate.
- [x] Split `analyze` in `crates/dsp/src/stft.rs`, 176 lines, into named stages.
- [x] Split `Layout::compute` in `crates/cli/src/render.rs`, 147 lines, into named stages.
- [x] Resolve the six `excessive_nesting` sites in `argand-dsp`.
- [x] Resolve `branches_sharing_code` in `crates/cli/src/render.rs`.
- [x] ➕ Rebase onto a `main` that had gained two merged Pull Requests during the work,
      one of which reworked `render.rs` substantially. It rebased without conflict and
      its new code passes this policy unchanged. This also explained a discrepancy the
      review caught: CI reported 226 tests against 224 locally, because a `pull_request`
      run tests the branch merged into its base, so CI was already running code the
      branch did not have.
- [x] ➕ Restore three rustdoc blocks that inserting helpers had detached from their
      functions, leaving `analyze`, `Plan::frame` and `EnvelopeBuilder::fold`
      undocumented while their text described the new neighbours.
- [x] ➕ Write `stdout_line` and `stderr_block` as chains of early returns instead of
      `if`/`else if`/`else` expressions. Requested by the owner, and it matches the
      codebase: `inputs::expand` classifies exactly this way, and 84 lines across 18
      non-test source files start with an early `return`. Each condition now stands on
      its own instead of being reached through an `else`.
- [x] Update `CONTRIBUTING.md` so the documented policy matches what is enforced.
- [x] Verify a deliberate policy violation fails CI, then revert it.
- [x] External review round, then act on the findings. Four rounds. Every finding was
      about the accuracy of a claim rather than about the code: detached rustdoc, a wrong
      statement about `f32::min` and NaN, a threshold left implicit, an overstated claim
      that a lint can enforce the owner's agreement, a stale test count, a coverage claim
      the evidence did not support, and stale line numbers after the rebase.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`: 226 tests, all passing.
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Every refactor is behaviour-preserving, checked against a binary built from
      `main`. All sixteen combinations of the eight panel sets and two orientations
      rendered byte-identical PNGs, as did both `--reduce` modes against both `--ref`
      levels, and the JSON reports match apart from `elapsed_seconds`.
- [x] Every configured threshold fires on a deliberate violation, each probed separately:
      8 arguments against 7, 107 lines against 100, cognitive complexity 61 against 25,
      five levels of nesting against 5, and 4 bool parameters against 3. An `#[allow]`
      without a reason fails as well. A pushed violation failed the `linux` job at
      `Lint`, with `Test` and `Build release` skipped; the probe was reverted.
- [x] An `#[allow(...)]` without a reason fails the lint step.
- [x] Both CI jobs pass on the Pull Request.

## Post-completion

- After this merges, cut the first release through a `chore/release-v0.1.0` Pull Request
  that renames `[Unreleased]` to `[0.1.0] - <date>` and fixes the changelog links. The
  administrator then tags `v0.1.0`.
- Merge the Pull Request with squash after review conversations and checks are complete.
