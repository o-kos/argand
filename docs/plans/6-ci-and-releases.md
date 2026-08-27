# Issue #6: Add Rust CI and tagged Linux and Windows releases

Resolves #6.

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
- **The release badge will read "no releases" until that tag is pushed.** That is
  accurate rather than broken: it is what the repository state is.

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

- [ ] Add `CHANGELOG.md` in Keep a Changelog format, with an `[Unreleased]` section and
      a `[0.1.0]` section describing what `aspec` does today.
- [ ] Add `.github/workflows/ci.yml`: `linux` and `windows` jobs, toolchain from
      `rust-toolchain.toml`, cargo caching, Pull-Request-only run cancellation, and
      read-only permissions.
- [ ] Add the release verification script: check the tag against
      `workspace.package.version`, check that the matching changelog section exists, and
      print that section.
- [ ] Add `.github/workflows/release.yml`: `verify`, then Linux and Windows builds with
      an `aspec --version` smoke check, then `publish` with `SHA256SUMS`, the archives
      and the changelog section as the Release body.
- [ ] Add the CI, release, license and Rust-version badges to `README.md`, and document
      where to get a binary.
- [ ] Document the release procedure and the changelog convention in `CONTRIBUTING.md`.
- [ ] Verify that a deliberate format, lint and test failure each turn CI red, and that
      a tag whose version does not match the workspace fails before anything is built.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Both CI jobs pass on this Pull Request.
- [ ] A commit that breaks formatting fails the `linux` job; a commit that trips Clippy
      fails it; a failing test fails both jobs. Each is verified and reverted.
- [ ] A tag whose version does not match `workspace.package.version` fails the `verify`
      job and publishes nothing. The tag is deleted afterwards.
- [ ] The release script prints the correct changelog section for a given version and
      fails when the section is missing.
- [ ] Every badge in `README.md` resolves to a real workflow, release feed, license file
      or version.

## Post-completion

- Require the `linux` and `windows` checks on protected `main`, with strict up-to-date
  branches, once a run of each has been recorded on `main`.
- After Issue #8 merges, cut the first release: annotated tag `v0.1.0`, confirm the
  Release is published, then download both archives and verify their checksums and
  `aspec --version`.
- Merge the Pull Request with squash after review conversations and checks are complete.
