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
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

The release build comes last, after every other check has passed, so that
`target/release/` holds a binary built from the branch as submitted. Never
demonstrate, measure, or diagnose behaviour with a binary left over from
earlier code.

Document any additional validation or explain why a standard check does not apply.

## Completing the plan

Before marking the Pull Request ready for final review:

1. complete every in-scope plan task and validation item;
2. rebuild the release binary so it matches the final state of the branch;
3. move the plan to `docs/plans/completed/` in a final commit;
4. update the Pull Request description if its scope or validation changed;
5. mark the Pull Request ready for review.

After all checks pass and all review conversations are resolved, squash-merge the Pull Request into `main`. Delete the branch after merge. The linked Issue closes through the Pull Request keyword.

Actions that can only happen after merge belong in the plan's `Post-completion` section and are not represented as unfinished implementation checkboxes.
