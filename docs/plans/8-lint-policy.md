# Issue #8: Define and enforce a workspace-wide Rust quality policy

Resolves #8.

## Overview

Clippy runs with its default lint set. That set is large and catches real bugs, but it
says almost nothing about maintainability: of the ten lints this Issue names, exactly one
is active by default, and the rest are `allow` in `pedantic`, `nursery` or `restriction`.
Thresholds are implicit, so nobody can tell what the project considers too long, too
wide or too deeply nested without reading Clippy's source.

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

| lint | places | where |
|---|---|---|
| `excessive_nesting` | 6 | `dsp/src/stft.rs` 626, 633 (twice), 647; `dsp/src/waveform.rs` 80, 83 |
| `too_many_lines` | 2 | `dsp/src/stft.rs:211` `analyze`, 176 lines; `cli/src/render.rs:232` `Layout::compute`, 147 |
| `branches_sharing_code` | 1 | `cli/src/render.rs:555` |
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
  a policy: the next person needs to know what it is protecting against.
- **`cognitive_complexity` is enabled even though it currently finds nothing.** Its
  value is preventing the next `Layout::compute`, not flagging today's.

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

- [ ] Add `clippy.toml` with the four thresholds, each carrying its reason.
- [ ] Enable the ten lints in `[workspace.lints.clippy]`, inherited by every crate.
- [ ] Split `analyze` in `crates/dsp/src/stft.rs`, 176 lines, into named stages.
- [ ] Split `Layout::compute` in `crates/cli/src/render.rs`, 147 lines, into named stages.
- [ ] Resolve the six `excessive_nesting` sites in `argand-dsp`.
- [ ] Resolve `branches_sharing_code` in `crates/cli/src/render.rs`.
- [ ] Update `CONTRIBUTING.md` so the documented policy matches what is enforced.
- [ ] Verify a deliberate policy violation fails CI, then revert it.
- [ ] External review round, then act on the findings.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Every refactor is behaviour-preserving, checked against a binary built before it:
      byte-identical renders and identical output in every reporting mode.
- [ ] A deliberate violation of each configured threshold fails the `linux` job.
- [ ] An `#[allow(...)]` without a reason fails the lint step.
- [ ] Both CI jobs pass on the Pull Request.

## Post-completion

- After this merges, cut the first release through a `chore/release-v0.1.0` Pull Request
  that renames `[Unreleased]` to `[0.1.0] - <date>` and fixes the changelog links. The
  administrator then tags `v0.1.0`.
- Merge the Pull Request with squash after review conversations and checks are complete.
