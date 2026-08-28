# Contributing to Argand

Argand uses an issue-driven workflow with versioned implementation plans. Repository content and all GitHub communication must be in English.

## Before implementation

Open-ended ideas may begin in a GitHub Discussion. Before implementation starts, create or select a GitHub Issue that states:

1. the problem;
2. the proposed solution or desired outcome;
3. verifiable acceptance criteria.

Create a branch from an up-to-date `main`:

```text
<type>/<issue>-<short-description>
```

Use `feature`, `fix`, `docs`, `refactor`, `test`, or `chore` as the type. For example, Issue `#42` could use `fix/42-iq-frequency-axis`.

Create `docs/plans/<issue>-<short-description>.md` from `docs/plans/TEMPLATE.md` and commit it before implementation. The plan must link to the Issue and describe the implementation tasks and validation strategy.

## Pull Request lifecycle

Push the branch and open a Draft Pull Request as soon as the initial plan is available. The Pull Request must:

- explain the problem and solution;
- link the active plan;
- close the Issue with `Closes #<issue>` or an equivalent GitHub keyword;
- remain Draft while implementation is incomplete.

Keep the branch focused on one Issue. If new work is independent of the Issue or materially expands its scope, create a separate Issue instead of silently adding it.

## Out-of-scope findings and backlog

Classify a problem found during implementation or review against the active Issue before
changing its scope. Apply these routes in order; the first matching route decides the
destination:

- Keep the finding in the active work when the branch introduced it, when it prevents an
  acceptance criterion or required check from passing, or when leaving it unresolved
  would make the plan, Pull Request, or claimed behaviour false.
- Otherwise, raise a separate normal Issue promptly when an unrelated finding is
  material or urgent, especially when it concerns security, correctness, or data safety.
  Do not hide it in either the active plan or the backlog.
- Only when neither route above applies, create a separate Issue with the `backlog` label
  when the finding is minor in impact, pre-existing, non-urgent, unrelated to the active
  objective, and has no effect on the current functionality or acceptance criteria. Do
  not expand the active plan or Pull Request to include it.

A backlog Issue must state the observed problem, the desired outcome, why it is safe to
defer, and where it was discovered. Link the active Issue or Pull Request so that the
context is not lost. When a review finding is deferred, reply with the backlog Issue link
instead of dismissing it only as out of scope.

Use `is:open is:issue label:backlog` to list backlog work and
`is:open is:issue -label:backlog` to list open Issues under normal triage. The absence of
the label does not itself promise that an Issue is scheduled.

## Implementing the plan

Keep commits atomic and write commit messages in English. When a plan task is completed, change its checkbox to `[x]` in the same commit as the corresponding work whenever practical.

Update the plan as reality changes:

- mark completed tasks with `[x]` immediately;
- mark newly discovered tasks with `➕`;
- mark blocked tasks with `⚠️` and explain the blocker;
- record material decisions and rejected alternatives;
- do not write commit hashes into the plan because rebases and squash merges make them unstable.

The branch commit history provides detail during review. The completed plan is the durable task-level record after squash merge.

Run relevant tests throughout implementation. Before requesting final review, run at least:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked
cargo test --locked
cargo build --release --locked
```

The release build comes last, after every other check has passed, so that
`target/release/` holds a binary built from the branch as submitted. Never
demonstrate, measure, or diagnose behaviour with a binary left over from
earlier code.

Document any additional validation or explain why a standard check does not apply.

Clippy warnings are errors. That is set in `[workspace.lints]` and inherited by every crate, so a plain `cargo clippy` is as strict on your machine as it is in CI. There is no `-D warnings` flag to remember and no way to get a laxer result by forgetting one.

The first three checks also run as a `pre-push` hook. Install it once per clone:

```sh
git config core.hooksPath .githooks
```

`git push --no-verify` skips it. If you use that, say why in the Pull Request.

## Lint suppressions

Every suppression must be agreed with the project owner before it is pushed. This covers `#[allow(...)]`, `#[expect(...)]`, `-A` flags, and any lint level relaxed in `Cargo.toml` or `clippy.toml`.

Refactor first. A suppression is the last resort, not the quick one, and the fact that a lint is inconvenient is not an argument that it is wrong. When one is genuinely unavoidable, ask for it explicitly and say what you tried, then write it as `#[expect(..., reason = "...")]` so that it fails the build once it stops being needed.

An unexplained suppression that nobody re-reads turns the whole gate into a formality. This repository has already seen that: four `#[allow(clippy::too_many_arguments)]` attributes silenced the only maintainability lint that was active, and one of them had stopped suppressing anything at all without anybody noticing.

## Continuous integration

`.github/workflows/ci.yml` runs on every Pull Request and on every push to `main`. Its `linux` job runs the four commands above, in that order; its `windows` job runs the test suite and the release build. Both are required checks on protected `main`.

The commands are not restated in the workflow with different flags. Formatting and lint configuration lives in the repository, where `cargo` finds it on its own, so what runs locally and what blocks the merge cannot drift apart. Tighten a rule by changing that configuration, not the workflow.

The toolchain comes from `rust-toolchain.toml`, which is the only place the Rust version is written down. Tests that need real captures skip when `tests/signals/` is absent, so a clean CI checkout runs the rest of the suite.

## External review

Before the owner is asked to review, the Pull Request goes through a review by a second
agent. Run it read-only so that the changes stay deliberate and this repository's own
rules -- in particular that suppressions need the owner's agreement -- are not bypassed
by an agent that has not read them:

```sh
codex exec -s read-only -C "$(git rev-parse --show-toplevel)" "$(cat review-prompt.md)" < /dev/null
```

Closing stdin is required; without it the command waits for input forever. Note that
`codex review --base <branch>` cannot be combined with a custom prompt, which is why
`codex exec` is used with the diff range named in the prompt itself.

Write the prompt for this repository rather than asking for a general review:

- name the exact diff under review, and the Issues the Pull Request closes;
- tell it to read `AGENTS.md` and `CONTRIBUTING.md` first, since a change may both extend
  those rules and have to obey them;
- rank what matters. Rarely-executed code that fails expensively -- workflows, release
  scripts -- comes before everything else, and behaviour changes in refactoring that was
  meant to preserve behaviour come next;
- exclude what the automated gate already covers. Asking a second reviewer about
  formatting or Clippy findings wastes it;
- ask it to say so explicitly when a category has nothing, rather than inventing a
  finding to look useful.

Act on the findings, then run another round naming what was fixed and what was declined,
and ask it to challenge the reasoning behind the declines. Repeat until a round returns
nothing substantive.

Then tell the owner what was accepted, what was rejected and why. A reviewer without the
conversation's context will sometimes object to decisions the owner already made
deliberately; decline those with the reason, never silently. Its most valuable findings
are the ones that correct a claim in the Pull Request description or the plan.

## Completing the plan

Before marking the Pull Request ready for final review:

1. complete every in-scope plan task and validation item;
2. rebuild the release binary so it matches the final state of the branch;
3. move the plan to `docs/plans/completed/` in a final commit;
4. update the Pull Request description if its scope or validation changed;
5. mark the Pull Request ready for review.

After all checks pass and all review conversations are resolved, squash-merge the Pull Request into `main`. Delete the branch after merge. The linked Issue closes through the Pull Request keyword.

Actions that can only happen after merge belong in the plan's `Post-completion` section and are not represented as unfinished implementation checkboxes.

## Changelog

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Every user-visible change adds a bullet to the `## [Unreleased]` section under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed` or `Security`. Write for someone deciding whether to upgrade, not for someone reading the diff. Internal refactoring, test work and workflow plumbing that nobody outside the repository can observe do not need an entry.

A feature or fix Pull Request never writes a release date and never creates a version heading. The release event owns both.

## Releasing

A release is its own Pull Request on a `chore/release-vX.Y.Z` branch. It is the one branch that does not carry an Issue number, because the release is not an implementation task and has no plan. The Pull Request:

1. sets `workspace.package.version` in the root `Cargo.toml` to `X.Y.Z`;
2. refreshes `Cargo.lock` so the workspace crates carry the new version;
3. renames `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` and opens a fresh empty `## [Unreleased]` above it;
4. updates the link definitions at the foot of the changelog.

Dry-run the checks the release workflow makes before pushing anything:

```sh
.github/scripts/release-notes.sh vX.Y.Z
```

It fails if the tag does not match `workspace.package.version` or the changelog has no section for it, and otherwise prints the section that becomes the GitHub Release description.

After the Pull Request is squash-merged, the repository administrator tags the merge
commit on an up-to-date `main`:

```sh
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

A ruleset restricts creating, moving and deleting `v*` tags to the administrator, so this
step and the one below cannot be carried out by anyone else. Branch protection does not
cover tags, and the workflow's own `main`-ancestor check runs from the tagged commit, so
it guards against a mistyped tag rather than against someone who should not be releasing
at all. Prepare the release Pull Request as usual and hand the tagging over.

The tag starts `.github/workflows/release.yml`, which verifies the tag against the workspace version and the changelog, runs the test suite, builds `aspec` for `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`, checks that each binary reports the version being released, and publishes `aspec-vX.Y.Z-<target>.tar.gz`, `aspec-vX.Y.Z-<target>.zip` and `SHA256SUMS` with the changelog section as the release description. Verification failures happen before anything is built, so a mistyped tag publishes nothing.

To redo a published release, the administrator deletes it together with its tag and
starts again:

```sh
gh release delete vX.Y.Z --cleanup-tag
```
