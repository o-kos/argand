# Issue #6: Add Rust CI and tagged Linux and Windows releases

Resolves #6 and #12.

One plan covers two Issues here. The lint work in #12 was found while reviewing #6 and
had to land with it: shipping CI whose lint step was silenced in four unexplained places
would have shipped the formality rather than the check. #12 was opened once that work had
clearly grown past what #6 describes, and it is recorded below among the `➕` steps rather
than in a second plan written after the fact.

## Overview

The repository has no automated validation. Every check in `CONTRIBUTING.md` runs on the
contributor's machine and nothing on `main` proves it was run, so a Pull Request can be
merged with a broken build. There is no changelog, no versioned release, no downloadable
binary, and no badge that tells a reader whether the project builds.

This change adds two workflows -- one that validates Pull Requests and pushes to `main`
on Linux and Windows, and one that turns a `vX.Y.Z` tag into a published GitHub Release
with signed-off archives for both platforms -- plus the `CHANGELOG.md` and README badges
that make the result legible.

The `v0.1.0` tag itself is deliberately **not** pushed by this Pull Request. See the
decision below.

## Context

- `rust-toolchain.toml` already pins `1.88.0` with `rustfmt` and `clippy`. The version
  exists in exactly one place today and must stay that way.
- The workspace is four crates; the only binary is `aspec` from `crates/cli`, and
  `aspec --version` already prints `aspec 0.1.0` from `workspace.package.version`.
- `main` protection today: signed commits, linear history, required conversation
  resolution, `enforce_admins`. There are **no** required status checks. A check cannot
  be marked required until a run with that exact name has been seen on the repository,
  so wiring protection is a post-merge step, not an implementation step.
- Fixture-dependent tests (`every_repository_capture_reads_end_to_end`,
  `real_captures_render_end_to_end`, `the_half_hour_capture_renders_end_to_end`,
  `reads_the_external_format_matrix`) look for `tests/signals/`, print a skip line and
  return when it is absent. That directory is gitignored, so a clean CI checkout runs
  the rest of the suite without them. No workflow-side special casing is needed.
- Baseline on current `main`, verified locally before starting: `cargo fmt --all --
  --check`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo test
  --locked` all pass. CI is expected green on its first run.
- `.github/` holds only `ISSUE_TEMPLATE/` and `pull_request_template.md`.
- `Cargo.lock` is committed, so every job builds the pinned dependency graph.

## Decisions

- **No toolchain-installer action.** GitHub runners ship `rustup`, and any `cargo`
  invocation in a checkout containing `rust-toolchain.toml` installs and uses the pinned
  channel by itself. Naming `1.88.0` in a `dtolnay/rust-toolchain` step would duplicate
  the version in a second place that silently drifts from the first. The workflow gets
  the toolchain from the repository, which is what the Issue asks for.
- **Two named jobs, not a matrix.** The Linux and Windows jobs run different command
  sets -- only Linux runs `fmt` and `clippy` -- so a matrix would need conditionals per
  step. Explicit `linux` and `windows` jobs also give branch protection two stable check
  names that survive a runner-image rename, which `test (ubuntu-latest)` would not.
- **CI runs the `CONTRIBUTING.md` command list verbatim**, in the same order, ending
  with the release build. What a contributor runs locally and what blocks the merge are
  the same four commands, which is also the interface Issue #8 will extend by adding
  repository lint configuration that `cargo clippy` picks up with no workflow edit.
- **Cancel superseded runs for Pull Requests only.** `cancel-in-progress` is keyed on
  the event, so a `main` push always finishes; killing it would leave `main` with no
  recorded result for that commit.
- **Third-party actions are pinned to a commit SHA** with the version in a trailing
  comment; GitHub-owned actions are pinned to their major tag. A moving third-party tag
  is the supply-chain risk worth spending a line of noise on.
- **Workflow permissions are `contents: read` at the top level**, raised to
  `contents: write` on the single release-publishing job. Nothing else in either
  workflow can write to the repository.
- **Release verification lives in a repository-owned script**, not inline in YAML, so a
  release can be dry-run locally before a tag is pushed. The script resolves the version
  through `cargo metadata` rather than by parsing `Cargo.toml` with a regex, and prints
  the changelog section that becomes the Release body.
- **The release job graph is `verify` -> `build` (Linux, Windows) -> `publish`.**
  Version and changelog mismatches are caught before a single archive is built, which is
  the acceptance criterion, and `SHA256SUMS` needs a job that sees both archives.
- **Windows builds natively on `windows-latest`.** `x86_64-pc-windows-msvc` cannot be
  produced from a Linux runner without the MSVC toolchain, and the release must be built
  by the same code path that CI tests.
- **`v0.1.0` is not tagged in this Pull Request.** Issue #8 defines the workspace lint
  policy and audits the four existing `#[allow(clippy::too_many_arguments)]`
  suppressions. Releasing first would publish `v0.1.0` from code that has not been
  through the policy it is about to adopt. The tag moves to `Post-completion` and is cut
  after #8 merges. Everything that makes the release possible -- workflow, script,
  changelog, badges -- still lands here and is verified here.
- **The initial changelog content sits under `[Unreleased]`, not under a `[0.1.0]`
  heading.** With the tag deferred there is no released `0.1.0`, and writing a section
  for a version nobody can download would be the changelog lying about the repository.
  The release-preparation Pull Request renames `[Unreleased]` to `[0.1.0] - <date>` and
  fixes the compare links, which is the Keep a Changelog flow the Issue asks for.
- **The release badge will read "no releases" until that tag is pushed.** That is
  accurate rather than broken: it is what the repository state is.
- ➕ **The Windows archive and the publishing step are rehearsed with a throwaway
  `v0.0.0` tag rather than trusted on first use.** Everything else here could be proven
  locally or by a failing probe; `7z`, the `.exe` suffix and `gh release create` could
  not. A rehearsal that is published and immediately deleted costs one throwaway tag and
  removes the chance of discovering a typo during the real release, when the fix is a
  deleted tag and a re-tag. The alternative -- a permanent `workflow_dispatch` dry-run
  mode -- would leave a second, less-tested path through the same workflow.
- ➕ **Strictness moved from the command line into the manifest.** A gate that is strict
  only when someone remembers `-D warnings` is not a gate. `[workspace.lints]` sets
  `warnings = "deny"`, every crate inherits it, and a plain `cargo clippy` now fails
  locally exactly as it fails in CI. The flag is gone from the workflow and the docs.
- ➕ **The owner extended this branch to cover the suppressions the new gate exposed.**
  Scoping them to Issue #8 was proposed and rejected: shipping CI whose lint step is
  silenced in four places by unexplained attributes would ship the formality, not the
  check. #8 keeps the part that adds *new* lints and thresholds.
- ➕ **Two review rules that no lint can enforce are written into `AGENTS.md`.** Opaque
  nested conditionals, and suppressions needing the owner's agreement before a push.
  Measurement backs the first: `write_report` scores 7 on cognitive complexity against
  19 for `Layout::compute`, and its nesting never exceeds three levels, so no threshold
  reaches it without flagging half the repository. It is a review obligation or nothing.
- ➕ **A release tag must point at a commit on `main`.** Branch protection does not reach
  tags: anyone able to create one could point it at unreviewed code and get an official
  Release built from it, and `gh release create --verify-tag` only checks that the tag
  exists. The `verify` job now refuses a tag whose commit is not an ancestor of `main`.
  This deliberately closes the rehearsal route used here -- a throwaway tag on a branch
  commit -- so any future rehearsal has to run from `main` after a merge. Protecting the
  `v*` tag namespace with a ruleset would close the remaining gap and is the owner's call.
- ➕ **`actions/checkout` is pinned by commit SHA like every other action.** The original
  decision pinned only third-party actions and left GitHub's own on a major tag, but
  GitHub's own guidance is that nothing short of a full SHA is immutable, and in the
  release workflow that checkout decides what ends up inside the published archives. The
  pinned SHA is what `v5` already resolved to, so CI's result carries over unchanged.
- ➕ **Branch protection is wired before the merge, not after it.** GitHub will accept a
  required check as soon as it has seen a run with that name, and both names have now
  been seen on this Pull Request. Requiring them now closes the Issue's "protected `main`
  requires both CI jobs" criterion inside the Issue instead of leaving it as a promise.

## Rejected alternatives

- `dtolnay/rust-toolchain` with an explicit `1.88.0`. See the decision above.
- A `matrix.os` job. See the decision above.
- `actions/cache` hand-rolled over `~/.cargo` and `target/`. Getting the key right --
  lockfile, toolchain, target triple, and pruning the registry so the cache does not
  grow without bound -- is exactly what `Swatinem/rust-cache` already does. Reproducing
  it by hand is more YAML for a worse cache.
- Cross-compiling Windows from Linux with `cargo-xwin` or the GNU target. It would save
  a runner but ship a binary built by a toolchain no test ever exercised, and the Issue
  names `x86_64-pc-windows-msvc` specifically.
- Building the release from the CI artifacts of the tagged commit. Artifact retention
  and the tag event are independent, so the release would sometimes have nothing to
  download. Rebuilding at tag time is a few minutes and always correct.
- Generating the Release body from commit subjects. Squash-merge subjects are accurate
  but not a changelog; `CHANGELOG.md` is the file the Issue asks to be the source.
- `cargo-dist`. It generates most of this, but it also owns the workflow file and adds a
  release-engineering dependency to a project whose stated goal is one binary with
  minimal external dependencies.

## Implementation steps

- [x] Add `CHANGELOG.md` in Keep a Changelog format, with everything `aspec` does today
      under `[Unreleased]`.
- [x] Add `.github/workflows/ci.yml`: `linux` and `windows` jobs, toolchain from
      `rust-toolchain.toml`, cargo caching, Pull-Request-only run cancellation, and
      read-only permissions.
- [x] ➕ Fix `an_error_states_its_cause_once`, which the first Windows run failed. It
      asserted on the Unix wording of `ErrorKind::NotFound` while what it actually
      guards is that the cause appears once; it now asks the platform for its own
      wording. This is the first thing CI found, and it is what the Issue exists for.
- [x] Add the release verification script: check the tag against
      `workspace.package.version`, check that the matching changelog section exists, and
      print that section.
- [x] Add `.github/workflows/release.yml`: `verify`, then Linux and Windows builds with
      an `aspec --version` smoke check, then `publish` with `SHA256SUMS`, the archives
      and the changelog section as the Release body.
- [x] Add the CI, release, license and Rust-version badges to `README.md`, and document
      where to get a binary.
- [x] Document the release procedure and the changelog convention in `CONTRIBUTING.md`.
- [x] Verify that a deliberate format, lint and test failure each turn CI red, and that
      a tag whose version does not match the workspace fails before anything is built.
- [x] ➕ Rehearse the whole release on a throwaway `v0.0.0` tag, then delete the release
      and the tag. The Windows `7z` archive, the `.exe` staging path and `gh release
      create` had no other way of being exercised before the real release.
- [x] ➕ Require the `linux` and `windows` checks on protected `main`, with strict
      up-to-date branches. Moved out of `Post-completion` once it became clear GitHub
      accepts the check names as soon as it has seen them, which it now has.
- [x] ➕ Write the two unenforceable review rules into `AGENTS.md`, and document the
      local gate and the suppression rule in `CONTRIBUTING.md`.
- [x] ➕ Deny warnings through `[workspace.lints]` inherited by every crate, drop the
      `-D warnings` flag from the workflow and the docs, and add a `pre-push` hook that
      runs the same three checks before anything leaves the machine.
- [x] ➕ Split `write_report` into two named decisions, one per output stream, and cover
      the policy with unit tests that do not need a spawned process.
- [x] ➕ Cut `Report::new` from nine arguments to four by handing it the
      `AnalysisRequest` that produced the analysis, built once and shared with `analyze`.
- [x] ➕ Group `TextRenderer::draw`'s placement and style into `Anchor` and `TextStyle`,
      collapsing eight repetitions of the same size-and-colour pair into one named
      `LABEL`.
- [x] ➕ Group `stft::render`'s parameters into `DbGrid` and `Shading`.
- [x] ➕ Delete the suppression in `crates/io/src/decoder.rs`, which had stopped
      suppressing anything: the function has exactly seven arguments and the lint fires
      at eight.
- [x] ➕ Refuse a release tag whose commit is not on `main`, and pin `actions/checkout`
      by SHA. Both came from an external review of the finished branch.
- [x] ➕ Update `docs/plans/TEMPLATE.md`, which still told every future plan to validate
      with `-D warnings` and would have kept reintroducing the flag this branch removed.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`: 219 tests, all passing.
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Both CI jobs pass on this Pull Request: `linux` in 47 s once the cache is warm,
      `windows` in 3 m 48 s.
- [x] A deliberate violation of each check turns CI red, and each probe was reverted:
      - formatting: `linux` failed at `Check formatting`, with `Lint`, `Test` and
        `Build release` skipped;
      - lint: `Check formatting` passed and `linux` failed at `Lint`;
      - a failing test: both `linux` and `windows` failed at `Test`.
- [x] Tag `v9.9.9` against workspace version `0.1.0` failed the `verify` job on its first
      step -- `tag 'v9.9.9' does not match workspace version '0.1.0'` -- with `build` and
      `publish` skipped and nothing published. The tag was deleted.
- [x] The release script was exercised over all six of its outcomes: a matching version
      with a section, a version mismatch, a missing section, a section with no content, a
      malformed tag, and a missing changelog file.
- [x] A full release rehearsal on a throwaway `v0.0.0` tag ran `verify` -> both builds ->
      `publish` green. Both archives were downloaded from the release: `SHA256SUMS`
      verified, each archive held `aspec`/`aspec.exe`, `README.md`, `LICENSE` and
      `CHANGELOG.md` under one top-level directory, and the extracted Linux binary
      reported `aspec 0.0.0`. The release and its tag were then deleted with
      `gh release delete v0.0.0 --cleanup-tag`, and the workspace version returned to
      `0.1.0`.
- [x] Protected `main` now lists `linux` and `windows` as required checks with `strict`
      enabled; signed commits, linear history, `enforce_admins` and required conversation
      resolution were all preserved.
- [x] No `#[allow]` or `#[expect]` attribute remains anywhere in `crates/`.
- [x] Removing the suppression in `report.rs` made a plain `cargo clippy` fail with
      `this function has too many arguments (9/7)`, and removing the one in `decoder.rs`
      changed nothing, which is how the stale attribute was identified.
- [x] The refactors are behaviour-preserving, checked against a binary built from the
      commit before them: the same capture rendered through both produces byte-identical
      PNGs, and `--json`, `-q`, `-v` and the default mode produce identical output apart
      from `elapsed_seconds`.
- [x] Both workflows parse as YAML after the changes, and the new tag check's shell body
      passes `bash -n`.
- [x] Every badge in `README.md` resolves: the CI badge to `ci.yml`, the licence badge to
      the repository's detected MIT licence, the Rust badge to `rust-toolchain.toml`. The
      release badge reads "no releases" until `v0.1.0` is tagged, which is the
      repository's actual state.

## Post-completion

- After Issue #8 merges, cut the first release through a `chore/release-v0.1.0` Pull
  Request that renames `[Unreleased]` to `[0.1.0] - <date>` and fixes the changelog
  links, then tag `v0.1.0`. The release workflow itself needs no further proving: the
  `v0.0.0` rehearsal already ran it end to end.
- Merge the Pull Request with squash after review conversations and checks are complete.
