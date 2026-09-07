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
- Introduce and validate the new full status before replacing required checks.
  Preserve strict up-to-date protection and bind the check to GitHub Actions.
- Keep PR #50's UI implementation separate and preserve its branch.

## Rejected alternatives

- Skipping required jobs under the same check name can count as successful CI.
- Disabling tests everywhere would remove the final cross-platform guarantee.
- Waiting for every remote run would retain the current feedback delay.

## Implementation steps

- [ ] Split quick and full job execution and add an explicit aggregate result.
- [ ] Test aggregation success/failure/skip/cancellation cases and lint workflows.
- [ ] Update contribution and agent rules for Draft feedback and final validation.
- [ ] Run the local gate, rebuild release and obtain clean external review.
- [ ] Verify Draft and Ready runs on GitHub before migrating required checks.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after checks
- [ ] Workflow syntax and result-aggregation tests
- [ ] Draft runs omit tests/release and do not publish `ci/full`
- [ ] Ready transition runs tests/releases on all three platforms and publishes `ci/full`
- [ ] External review with the prescribed model returns no substantive findings

## Post-completion

After the validated PR is merged, migrate main's required checks to `ci/full`
without relaxing strictness or changing other protection settings. Bring active
PR branches up to date and keep iterative UI work in Draft. A manual dispatch
uses the selected ref; the Ready PR run remains the merge-validation path.
