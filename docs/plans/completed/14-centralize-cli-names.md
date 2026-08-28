# Issue #14: Centralize CLI names for panels and orientations

Resolves #14.

## Overview

Canonical panel and orientation names are repeated independently across parsing,
formatting, allowed-value diagnostics, help, and CLI defaults. A rename or new variant
can therefore leave those surfaces inconsistent without a compiler error.

This change makes the typed panel and orientation representations own their canonical
names and aliases, then derives every production-code consumer from that metadata. It is
a behaviour-preserving maintainability refactor: accepted values, canonical output,
diagnostics, help, defaults, and rendered images remain unchanged.

## Context

- `Panels` is a public set of three booleans because several panels may be combined; its
  parser also accepts a standalone special `none` value.
- `Orientation` is already an enum, but its canonical names are repeated in
  `ORIENTATION_NAMES`, `as_str`, `FromStr`, and the CLI default and help.
- Panel aliases are `wave`, `spectrum`, and `colorbar`; orientation aliases are `h` and
  `v`. Parsing is case-insensitive and trims surrounding whitespace.
- Tests should continue to spell expected CLI strings literally so they remain
  independent checks of the external contract.

## Decisions

- **A private `Panel` enum owns selectable-panel metadata.** A local macro generates the
  enum, its exhaustive `ALL` registry, and `as_str` from one declaration; exhaustive
  matches attach aliases and enabled-flag access. Parsing and formatting order follow
  `ALL`. `Panels` remains the public combinable set, so rendering code and report data do
  not change shape.
- **`none` remains separate metadata.** It is an empty selection token, not a fourth
  drawable panel. Its canonical spelling is defined once and reused by parsing,
  formatting, diagnostics, and help.
- **`Orientation` owns `ALL`, `as_str`, and its short aliases.** The same macro makes the
  variant registry exhaustive, while an exhaustive `short_alias` match gives the help
  example an explicit source. The parser walks those typed variants. The CLI default uses
  `default_value_t`, so the default is formatted by the type rather than repeated as a
  string.
- **CLI help is built from typed metadata.** Small helper functions construct the current
  help sentences from canonical names. Literal contract tests verify the resulting text.
- **No new dependency is introduced.** The existing custom parsers preserve trimming,
  case-insensitivity, comma-separated panel combinations, aliases, and tailored errors.

## Rejected alternatives

- Four standalone panel-name constants. They reduce literal repetition but still leave
  the relationship between names, aliases, flags, ordering, and parsing implicit.
- `clap::ValueEnum` for orientation. It would centralize some CLI metadata, but the
  rendering type would acquire CLI-framework metadata while the panel list would still
  require a custom parser. The shared inherent metadata keeps both types consistent and
  preserves the current normalization rules directly.
- Replacing `Panels` with a collection of enum values. That would spread a
  behaviour-neutral CLI cleanup into layout and rendering code that currently benefits
  from direct named flags.

## Implementation steps

- [x] Introduce typed panel metadata and derive panel parsing, formatting, diagnostics,
      ordering, help, and the CLI default from it.
- [x] Derive orientation parsing, formatting, diagnostics, help, and the CLI default from
      `Orientation` metadata.
- [x] Add contract coverage for every alias, generated help, canonical formatting, and
      unchanged invalid-value diagnostics.
- [x] Confirm the refactor does not change existing CLI or render behaviour.
- [x] ➕ Generate each enum and its `ALL` registry together so a new variant cannot be
      omitted from parsing, formatting, diagnostics, or help.
- [x] ➕ Model orientation short aliases explicitly instead of treating the first alias as
      the preferred help spelling.
- [x] ➕ Build orientation help by walking the exhaustive registry, with an exhaustive
      description match for each variant.
- [x] External review round, then act on the findings.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] `git diff --check`
- [x] Focused CLI and render tests cover every canonical name and alias, both defaults,
      stable diagnostics, and generated help.
- [x] Existing end-to-end panel/orientation rendering tests pass without output changes.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
- Remove `/tmp/argand-issue-14` after the Pull Request is accepted and merged.
