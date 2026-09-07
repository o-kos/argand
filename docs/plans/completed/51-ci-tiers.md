# Issue #51: Fast Draft feedback and full pre-merge CI

Resolves [#51](https://github.com/o-kos/argand/issues/51).

## Overview

Keep intermediate PR feedback independent of full remote builds. Draft updates run
Linux formatting/Clippy; Ready PRs, main pushes and manual runs use the full matrix.
Preserve local validation and require a full result on the revision being merged.

## Decisions

- Subscribe to Draft/Ready transitions as well as normal PR updates. Returning to
  Draft cancels a superseded full run using the existing PR concurrency group.
- Keep UI feedback in Draft; Ready means the final pre-merge gate is requested.
- Name the aggregate result `ci/quick` for Draft and `ci/full` otherwise. A skipped
  full job must never produce a successful `ci/full` result in a quick run.
- Full aggregation requires explicit success from Linux, Windows and macOS.
- Preserve read-only workflow permissions and pinned dependency/action versions.
- Validate the new full status, then add it alongside the old required checks
  before merging this workflow. Remove the old contexts only after merge. This
  avoids a rollout window where skipped old checks alone could permit merging.
  Preserve strict up-to-date protection and bind the check to GitHub Actions.
- Keep PR #50's UI implementation separate and preserve its branch.

## Rejected alternatives

- Skipping required jobs under the same check name can count as successful CI.
- Disabling tests everywhere would remove the final cross-platform guarantee.
- Waiting for every remote run would retain the current feedback delay.

## Implementation steps

- [x] Split quick and full job execution and add an explicit aggregate result.
- [x] Test aggregation success/failure/skip/cancellation cases and lint workflows.
- [x] Update contribution and agent rules for Draft feedback and final validation.
- [x] Run the local gate, rebuild release and obtain clean external review.
- [x] Exercise Draft/Ready transitions and cancellation on GitHub.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after checks
- [x] Workflow syntax and result-aggregation tests
- [x] Draft runs omit tests/release and do not publish `ci/full`
- [x] Ready starts all three platforms; returning to Draft cancels them and fails `ci/full`
- [x] External review with the prescribed model returns no substantive findings

## Validation evidence

- Formatting, strict Clippy, all 362 Rust tests and the subsequent release build pass.
- actionlint 1.7.12 accepts the workflows. The result-policy tests exercise all 250
  combinations of success, failure, cancellation, skip and absent results across
  both modes, plus malformed mode/argument cases.
- The first Draft run completed Linux formatting/Clippy, skipped its Test and
  Build release steps, skipped Windows/macOS jobs and passed `ci/quick`.
- Ready started Linux, Windows and macOS. Returning to Draft cancelled that run;
  its aggregate reported `ci/full` failure instead of accepting cancelled jobs.
- A successful full run on the final revision is the remaining merge gate, recorded
  in the PR's GitHub checks. It must pass before protection migration and merge;
  recording its result in a later docs-only commit would invalidate that revision.

- External review found an outdated CI description in CONTRIBUTING.md. The
  finding was accepted: the paragraph now refers to the tier policy, and README
  uses the same distinction. No finding was declined. The final review round
  returned no substantive findings.

## Post-completion

After merge, remove the old required contexts while retaining `ci/full`, strict
up-to-date checking and all other protection settings. Bring active
PR branches up to date and keep iterative UI work in Draft. A manual dispatch
uses the selected ref; the Ready PR run remains the merge-validation path.
