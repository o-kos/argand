# Contributing to Argand

Argand uses an issue-driven workflow with versioned implementation plans for code
changes. Repository content and all GitHub communication must be in English.

## Roles and workflow scope

[Agent roles and model selection](AGENTS.md#agent-roles-and-model-selection) is the
source of truth for role assignments and risk classes. The approved defaults need
no repeated per-Issue approval.
The coordinator discusses analysis, architecture, plans, acceptance and disagreements
with the owner and assesses the result; the executor performs repository
investigation, edits, Git/GitHub operations, tests and builds. The reviewer reads
the assigned material without changing it. Changes to product behaviour, UX or
acceptance criteria require the owner's decision.

For work without code changes, the coordinator decides whether an Issue, Pull
Request, new branch, formal plan, external review, tests or the full local gate is
needed, based on scope and risk. Record the decision and reason in the response or
an existing artifact. A wording correction may need only a diff and link check;
CI, configuration, release-policy or security edits can affect behaviour and need
proportionate scrutiny and validation. No code does not mean no risk.

This decision scopes the otherwise mandatory workflow, review and local validation
requirements in this document, including the plan and Pull Request templates.
Code changes follow the complete process. Protected `main`, required remote checks,
release authority, lint-suppression approval and owner decisions about product or
acceptance changes remain binding. A merge into `main` still requires a Pull Request
and the full required checks; omitting a Pull Request never permits a direct push.
Preserve unrelated work and leave completed historical plans unchanged.

## Engineering and dependency conventions

- Local Linux development uses the separately installed [GPUI Kit SDK](docs/development/gpui-sdk.md). After explicit SDK selection, ordinary `cargo build`, `cargo test`, `cargo clippy` and `cargo run -p argand --locked` consume it without building it. Portable source releases use `cargo build --release --locked --features source-toolkit`; source tests and all-target Clippy also select `toolkit-test-support`.
- Take the GUI toolkit as one crates.io dependency, `gpui-kit`, with the version requirement in `Cargo.toml`, and let the committed `Cargo.lock` fix the exact graph. `gpui-kit` selects and re-exports the compatible GPUI 0.3.x family, gpui-component and the toolkit assets; do not add direct `gpui`, `gpui-component` or `gpui-component-assets` dependencies beside it, because a type from one copy of the graph is not that type from the other.

  This rule used to name `gpui` and `gpui-component` separately on crates.io, and before that, fixed Git revisions. The separate selections drifted out of compatibility, which is the mistake `gpui-kit` exists to prevent. Do not go back to naming the layers, and do not `[patch]` the registry source without a reason that names what is only available at tip.

- Commit the complete dependency graph in `Cargo.lock`; do not commit `vendor/`. Run `cargo fetch --locked` before an offline build, then use `cargo build --frozen`.
- A real GPU is required. Software rendering and some virtual machines may degrade performance.

- Use `thiserror` in libraries and `anyhow` in binaries. Non-test code must not use `unwrap` or `expect` on external data. Panics must not unwind across FFI boundaries.

- Write code comments only when the intent is not evident from the code itself, and keep them concise.

- Do not write nested, multi-level, opaque `if` chains. A reader must be able to tell what a branch does without holding three conditions and a later early return in their head at once. No lint catches this: it is a review obligation, and the two shapes below are the ones that keep appearing.
  - **One flag tested twice in two different roles.** Here `args.quiet` gates stdout in the `else if`, then gates stderr as an early return three lines later, so it reads like a duplicate that has to be proven not to be one:

    ```rust
    if args.json {
        println!("{}", report.to_json());
    } else if !args.quiet && reporting.echo_paths {
        if let Some(output) = &report.output {
            println!("{}", output.path);
        }
    }
    if args.quiet {
        return;
    }
    ```

    Decide once, in one place. Name the outcome — an enum, a `match`, or a single early return at the top — and let each branch do one thing.
  - **A condition repeated inside its own `else`.** Here `reporting.batch` is tested in the `if` and again in the `else`, which forces the reader to work out that the second test is not the same question as the first:

    ```rust
    if reporting.batch && args.verbose == 0 {
        report.write_compact(&mut stderr, index, reporting.total).ok();
    } else {
        if reporting.batch && index > 1 {
            writeln!(stderr).ok();
        }
        report.write_human(&mut stderr).ok();
    }
    ```

    Hoist the shared condition, or split the function so each half has one job.

### Local Rust and build resources

Use one owner-approved installed Rust toolchain for all projects, preferably the
system-managed Rust and Cargo. Installing or upgrading Rust requires explicit owner
approval. Project pins document reproducibility requirements, not permission for an
automatic rustup download. Check executable selection without triggering a missing
toolchain download; stop and discuss mismatches instead of adding overrides or
changing the installation, defaults, PATH or project pins. CI provisioning is separate.

Argand and Wireroom use one agreed GPUI Kit version and compatible locked backend
graph. Do not independently change toolkit/backend versions, toolkit features or
locked toolkit dependencies. Stop and discuss incompatibilities with the owner.

Use one explicitly configured common Cargo `build.build-dir` outside all project
worktrees across projects and worktrees for intermediate artifacts and the build
cache. Keep final outputs in each workspace's local `target/` (`build.target-dir`);
do not set a shared global target-dir. Do not create per-worktree intermediate build
caches or scratch build caches. If the common build-dir is not known or configured,
stop before building. Preserve existing intermediate artifacts when configuring the
split; do not move or clear them just to change the final-output directory.
The owner-approved baseline covers debug/test, release and check/Clippy artifacts. Before initial dependency
compilation, agree the installed toolchain and common cache with the owner and obtain
explicit approval to build that baseline. Ordinary recompilation of changed project
code is allowed within the agreed setup. If GPUI Kit or other dependencies need to be
rebuilt beyond that baseline, stop before rebuilding, explain the concrete cause of
artifact invalidation, and obtain explicit owner approval. Required validation does
not waive this approval. Avoid unnecessary feature, profile, compiler, target or
RUSTFLAGS changes and repeated builds; different compilation inputs can invalidate
dependency artifacts even when GPUI source is unchanged. Keep final binary paths
local to their workspace and verify the exact built revision before use; a path
alone does not identify the source revision. Do not clear the common intermediate
cache while another project or task may use it. Configuration or metadata checks
do not prove that Cargo will reuse every dependency artifact without rebuilding.

Only the owner controls the disk budget. Do not add quotas, monitoring scripts,
periodic measurements or extra budget-management procedures.

## Before implementation

Open-ended ideas may begin in a GitHub Discussion. Under the workflow scope above, before implementation starts, create or select a GitHub Issue that states:

1. the problem;
2. the proposed solution or desired outcome;
3. verifiable acceptance criteria.

Create a branch from an up-to-date `main`:

```text
<type>/<issue>-<short-description>
```

Use `feature`, `fix`, `docs`, `refactor`, `test`, or `chore` as the type. For example, Issue `#42` could use `fix/42-iq-frequency-axis`.

Use the repository's main working directory by default. Create an additional
worktree outside it only when the project owner explicitly asks for one; do not
infer that request from parallel work, isolation, or convenience.
Place every requested linked worktree under the sibling `argand.worktrees`
directory (`../argand.worktrees/<task>` from the main checkout). Use the common Cargo
intermediate build-dir outside all worktrees and keep final target-dir outputs local
to each workspace. The intermediate cache can hold multiple legitimate compiler,
feature and profile variants; verify the built revision before demonstrating an
artifact.
Remove completed linked worktrees using the safe completion checks below; do not
force removal of unique or unrelated work.

Create `docs/plans/<issue>-<short-description>.md` from `docs/plans/TEMPLATE.md` and commit it before implementation. The plan must link to the Issue and describe the implementation tasks and validation strategy. Record its risk class and the approved role assignments before execution; classes describe scrutiny, not reviewer-model selection.

## Pull Request lifecycle

Push the branch and open a Draft Pull Request as soon as the initial plan is available. The Pull Request must:

- explain the problem and solution;
- link the active plan;
- close the Issue with `Closes #<issue>` or an equivalent GitHub keyword;
- remain Draft throughout implementation and iterative owner feedback, including UI wording and appearance refinements.

A Draft is reviewable: share the locally validated result without moving it to Ready
or waiting for remote CI after every adjustment. Move to Ready only when the agreed
implementation is complete and the full pre-merge check is wanted. Return to Draft
before another round of small changes. After Ready, every push requests full CI again.

Keep the branch focused on one Issue. If new work is independent of the Issue or materially expands its scope, create a separate Issue instead of silently adding it.

## Out-of-scope findings and backlog

Classify a problem found during implementation or review against the active Issue before
changing its scope. Apply these routes in order; the first matching route decides the
destination:

- Keep the finding in the active work when the branch introduced it, when it prevents an
  acceptance criterion or required check from passing, or when leaving it unresolved
  would make the plan, Pull Request, or claimed behaviour false.
- Otherwise, raise a separate normal Issue promptly when an unrelated finding is
  material or urgent, or when it concerns security, correctness, or data safety
  regardless of its apparent impact or urgency. Do not hide it in either the active plan
  or the backlog.
- Only when neither route above applies, create a separate Issue with the `backlog` label
  when the finding is minor in impact, pre-existing, non-urgent, unrelated to the active
  objective, has no effect on the current functionality or acceptance criteria, and does
  not concern security, correctness, or data safety. Do not expand the active plan or
  Pull Request to include it.

A backlog Issue must state the observed problem, the desired outcome, why it is safe to
defer, and where it was discovered. Link the active Issue or Pull Request so that the
context is not lost. When a review finding is deferred, reply with the backlog Issue link
instead of dismissing it only as out of scope.

Use `is:open is:issue label:backlog` to list backlog work and
`is:open is:issue -label:backlog` to list open Issues under normal triage. The absence of
the label does not itself promise that an Issue is scheduled.

## Implementing the plan

### UI component selection

Use only standard gpui-component controls and GPUI interaction mechanisms, or
already existing shared project UI elements. Before implementing a UI change,
search the project for the corresponding behavior and presentation; inspect the
locked toolkit implementation and the existing shared element's supported styling,
composition and configuration. Reuse behavior and style together. Different colors,
spacing or geometry do not justify replacing standard interaction behavior.

Every new nonstandard widget requires separate owner approval before implementation.
For each proposed widget, add a separate entry to the active plan naming:

- the standard alternative and its exact, verified limitation in the locked version;
- why existing project elements and supported composition or styling cannot meet
  the requirement;
- the behavior and maintenance cost the application would take over;
- focus, keyboard, pointer, tooltip, disabled-state and regression validation;
- the owner's decision before implementation.

Extending an existing shared element does not bypass this requirement when the
extension creates a custom replacement or introduces new nonstandard widget
behavior. Apply the same audit to retained custom controls during UI reorganization.
Domain canvases such as spectrograms, waveforms, axes and minimaps remain valid custom
drawing; ordinary controls embedded over them have no exemption. Do not hand-write
text editing or ordinary button behavior to bypass an unresolved root, focus or
styling problem. Reviewers must challenge unsupported exceptions and verify that
replacements remove the obsolete implementation. Changes to established UX need a
separate owner decision.

When changing a shared UI element, check each affected consumer, including both
panels when two panels share it. Verify relevant appearance, focus, keyboard and
pointer behavior, tooltips and disabled states in those consumers rather than
assuming that a local field's success establishes the shared contract.

### UI acceptance

For UI work, the coordinator includes acceptance in the plan, the executor reads
and follows the repository-local [UI acceptance skill](.agents/skills/argand-ui-acceptance/SKILL.md)
without waiting for the owner to invoke it by name, and the reviewer assesses the
evidence. This section remains the canonical acceptance policy.

UI acceptance requires running the current production application binary and
exercising the actual affected scenario, checking appearance and interactions.
Use the current-revision build and validation requirements below. Compare relevant
shared or neighboring UI behavior and exercise the affected entry points and
transitions, including focus, Select open/choose/dismiss, Enter/Escape, disabled or
busy state and closing an overlay or window where the change can affect them.

Code review, automated or headless tests and a static screenshot alone do not
establish UI acceptance. Record what was exercised in the running application and
separate that evidence from automated checks. If the application cannot be run or
the required interactions cannot be exercised, acceptance remains incomplete:
report the limitation, do not call the UI work complete or ready for acceptance,
and do not move its Pull Request to Ready. Locally verified partial progress may
still be shared in Draft with that limitation stated.

### Implementation and validation

Keep commits atomic and write commit messages in English. When a plan task is completed, change its checkbox to `[x]` in the same commit as the corresponding work whenever practical.

Update the plan as reality changes:

- mark completed tasks with `[x]` immediately;
- mark newly discovered tasks with `➕`;
- mark blocked tasks with `⚠️` and explain the blocker;
- record material decisions and rejected alternatives;
- do not write commit hashes into the plan because rebases and squash merges make them unstable.

The branch commit history provides detail during review. The completed plan is the durable task-level record after squash merge.

For a bug fix, inspect related cases that share the same root cause across formats,
fields and action entry paths. Apply the correction to those affected cases and
verify the common invariant; do not stop at the first reproducer. Keep agreed
behavior and acceptance boundaries intact. A related case that requires a product
or scope change needs the existing owner decision and triage process, not an
unannounced expansion.

Run relevant tests throughout implementation. For code changes, before requesting final review, run at least:

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

Document any additional validation or explain why a standard check does not apply. For work without code, use the coordinator's scoped validation decision above; do not claim an unrun gate passed.

Clippy warnings are errors. That is set in `[workspace.lints]` and inherited by every crate, so a plain `cargo clippy` is as strict on your machine as it is in CI. There is no `-D warnings` flag to remember and no way to get a laxer result by forgetting one.

The first three checks also run as a `pre-push` hook. Install it once per clone:

```sh
git config core.hooksPath .githooks
```

`git push --no-verify` skips it. Code changes still require the full local gate
before every push. A scoped decision for work without code does not alter the hook;
if it justifies skipping the hook, record why in the Pull Request or task response.
Required remote checks are unaffected.

## Remote CI tiers

The hook and remote CI are unchanged by scoped local-validation decisions. Remote CI is a separate feedback schedule:

| Trigger | GitHub validation | Aggregate status |
| --- | --- | --- |
| Open/update/reopen a Draft PR; return to Draft | Linux formatting and Clippy, plus the small CI-policy test | `ci/quick` |
| Move to Ready; update/reopen a Ready PR | Formatting/Clippy on Linux; tests and release builds on Linux, Windows and macOS | `ci/full` |
| Push to `main`; manual workflow dispatch | Full validation | `ci/full` |

Returning to Draft cancels the superseded PR run. In-progress main runs are not
cancelled by later pushes. Manual dispatch validates the selected ref; use the Ready PR run as
the merge-validation path so the current base is included.

Do not wait for GitHub CI between small iterations. Report local validation and
whether remote CI is pending; inspect failures when reported. Before merge, wait
for `ci/full` on the current PR revision, resolve review conversations and ensure
the branch is up to date. An earlier green revision is not evidence for a new one.

The full result requires explicit success from all three platforms; failure,
cancellation or a skipped platform fails it. Drafts publish `ci/quick`, never a
successful `ci/full`. Keep `ci/full` required on `main`, tied to GitHub Actions,
with strict up-to-date checking. Migrate protection in stages: after a full PR run
proves the new status, add it alongside the old required checks before merging the workflow. Remove the old
contexts only after the workflow reaches `main`; preserve the remaining protection.
Older open branches must incorporate the new `main` before using the new checks.

## Lint policy

`[workspace.lints]` in the root `Cargo.toml` names every lint this project enforces
beyond Clippy's defaults, and each crate inherits it with `lints.workspace = true`. The
thresholds those lints read live in `clippy.toml`, one comment per value saying what it
protects against.

Lints are enabled one at a time. The `pedantic`, `nursery` and `restriction` groups are
never turned on wholesale: they contain lints that contradict this codebase and each
other, and a group grows silently on a toolchain bump. Adding a lint is a deliberate act,
and so is changing a threshold — say why in the Pull Request.

When a maintainability lint fires, the first answer is to change the code. A long
function is a sequence of stages that has not been named; a wide signature is a type that
has not been written. Raising a threshold to make a warning go away needs the same
agreement as a suppression.

One trap when measuring by hand: `warnings = "deny"` turns findings into errors, and a
crate that fails to compile is never linted, so findings in a crate that depends on a
failing one stay hidden. Pass `-W warnings` to put the level back while surveying.

## Lint suppressions

Every suppression must be agreed with the project owner before it is pushed. This covers `#[allow(...)]`, `#[expect(...)]`, `-A` flags, and any lint level relaxed in `Cargo.toml` or `clippy.toml`.

Refactor first. A suppression is the last resort, not the quick one, and the fact that a lint is inconvenient is not an argument that it is wrong. When one is genuinely unavoidable, ask for it explicitly and say what you tried, then write it as `#[expect(..., reason = "...")]` so that it fails the build once it stops being needed.

An unexplained suppression that nobody re-reads turns the whole gate into a formality. This repository has already seen that: four `#[allow(clippy::too_many_arguments)]` attributes silenced the only maintainability lint that was doing any work at the time, and one of them had stopped suppressing anything at all without anybody noticing.

## Continuous integration

`.github/workflows/ci.yml` implements the [remote CI tiers](#remote-ci-tiers).
Drafts receive quick Linux feedback; full runs include all three platforms.
Linux uses `ubuntu-24.04` and its `/etc/apt/sources.list.d/ubuntu.sources` definition.
`install-ubuntu-packages.sh` restricts both APT operations to that source and fresh
package lists and archives; it preserves the runner's signing configuration and fails on every
required-source update error. Unrelated feeds and their cached indexes are excluded
without rewriting the runner's configuration. Revisit this source-file contract
when upgrading the Linux image. `test-ubuntu-packages.py` runs before installation
in both CI tiers, using signed local repositories and download-only APT operations.
The aggregate `ci/full` is the required check after the staged protection migration.

No runner has a GPU, so no job opens a window. What the three of them cover is that the workspace builds on every supported platform and that everything not needing a window passes; the application's own configuration and geometry logic is written to be testable without one.

The commands are not restated in the workflow with different flags. Formatting and lint configuration lives in the repository, where `cargo` finds it on its own, so what runs locally and what blocks the merge cannot drift apart. Tighten a rule by changing that configuration, not the workflow.

The exact toolchain comes from `rust-toolchain.toml`; the workspace's minimum Rust version is declared in `Cargo.toml` and repeated in the Rust badge in `README.md`. The declared minimum and the badge must agree with the toolchain's major and minor version. Tests that need real captures skip when `tests/signals/` is absent, so a clean CI checkout runs the rest of the suite.

## External review

For a required review, the coordinator directs the executor to prepare and update
an exact, neutral handoff using the repository-local
[review handoff skill](.agents/skills/argand-review-handoff/SKILL.md), without waiting
for the owner to invoke it by name. This section remains the canonical review policy.

Use the owner-approved [reviewer assignment](AGENTS.md#agent-roles-and-model-selection)
in a separate agent and context. Keep that assignment for every round of a Pull Request;
changing the defaults requires the owner's agreement.

Before presenting code implementation results for owner review, including Draft
feedback, the Pull Request goes through external review. For work without code,
apply the coordinator's scoped decision above. Run the reviewer read-only so it
cannot bypass deliberate edits or the owner's approval requirements.

Give the reviewer the task, agreed plan when present, exact diff and revision, and
repository rules, without the executor's discussion or self-assessment. The executor
prepares the prompt and runs a fresh review context, for example:

```sh
codex exec -s read-only --model gpt-6.1-sol \
  -c 'model_reasoning_effort="high"' \
  -C "$(git rev-parse --show-toplevel)" "$(cat /tmp/argand-review-prompt.md)" < /dev/null
```

Keep temporary review prompts and reports outside the visible worktree. Closing
stdin is required; without it the command waits for input forever. Note that
`codex review --base <branch>` cannot be combined with a custom prompt, which is why
`codex exec` is used with the exact diff named in the prompt itself. For uncommitted
changes, provide the actual patch as well as the base revision so the review covers
the submitted state.

Write the prompt for this repository rather than asking for a general review:

- name the task, exact diff and revision under review, the agreed plan when present, and any Issues the Pull Request closes;
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
nothing substantive, with a maximum of three rounds. If substantive findings remain
after the third, the coordinator reconsiders the task and plan instead of starting a
fourth. Later prompts may include findings and their dispositions, but must still
exclude the executor's discussion and self-assessment. Only the owner can waive a
required round; record that waiver in the Pull Request.

The coordinator arbitrates technical disagreements using the evidence; product, UX
or acceptance changes go to the owner. The executor records the decisions. Before
presenting the result, the coordinator assesses the exact diff, review findings and
validation evidence against the current revision, including local changes. Refresh
evidence affected by later changes and identify its limits. For DSP, concurrency
or data safety, the coordinator additionally examines the critical invariants and
the evidence for them. A clean review does not replace this assessment. Confirm
through the executor that any demonstrated release binary was built from the current
code after the required checks passed.

Then tell the owner which model reviewed, what was accepted and how it was addressed,
what was rejected and why, and whether the final round was clean. A reviewer without the
conversation's context will sometimes object to decisions the owner already made
deliberately; decline those with the reason, never silently. Its most valuable findings
are the ones that correct a claim in the Pull Request description or the plan.

## Completing the plan

Apply the workflow scope above for work without code. For code changes, before marking the Pull Request ready for final review:

1. complete every in-scope plan task and validation item;
2. rebuild the release binary so it matches the final state of the branch;
3. move the plan to `docs/plans/completed/` in a final commit;
4. update the Pull Request description if its scope or validation changed;
5. mark the Pull Request Ready to request full remote validation.

Earlier UI feedback stays in Draft after local validation and external review;
it does not require waiting for the remote matrix.

After `ci/full` passes on the current revision and all review conversations are resolved, squash-merge the Pull Request into `main`. Delete the accepted Pull Request branch both remotely and locally, regardless of its type. First verify that its tip still matches the accepted Pull Request head, every worktree using it is clean, and it has no unpushed commits; preserve any data, then switch or remove those worktrees. Delete the remote ref conditionally with a lease bound to the accepted Pull Request head. Delete the local ref conditionally with that same head as its expected old value, because a squash merge does not make the branch commits ancestors of `main`. Abort cleanup if either ref moved. When there is a linked Issue, it closes through the Pull Request keyword.

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
