# Issue #19: Upgrade actions/checkout to v7

Resolves #19.

## Overview

`actions/checkout` is pinned to v5.1.0 while v7.0.1 is current. It runs first in every
job and decides what the workspace contains, so it is the one action worth keeping
current. The other three pins are already at their newest release.

This change moves the four pins and proves the new version on every form the workflows
use, without cutting a release to do it.

## Context

- The drift came from writing `@v5` before the pinning pass in #11, then pinning to
  whatever `v5` resolved to rather than to the newest release. The review that prompted
  the pinning questioned the mutable tag, not the version behind it.
- v7.0.1 is commit `3d3c42e5aac5ba805825da76410c181273ba90b1`, which is also what the
  `v7` tag currently resolves to.
- The net change from v5.1.0 is smaller than the v6 and v7 release notes suggest, because
  v5.1.0 already carries two of the headline items: it runs on `node24`, and the
  `pull_request_target` fork-checkout protection was backported into it. What the upgrade
  actually brings is credentials written to a separate file rather than the shared git
  config (v6.0.0), a tag-handling fix that preserves annotations and honours an explicit
  `fetch-tags` (v6.0.2), and the move to ESM with refreshed dependencies (v7.0.0).
- That tag-handling fix lands on the one path this branch cannot exercise, which is an
  argument for taking the upgrade rather than against it.
- `checkout` appears four times, and `publish` does not use it at all:

  | workflow | job | inputs |
  |---|---|---|
  | `ci.yml` | `linux` | none |
  | `ci.yml` | `windows` | none |
  | `release.yml` | `verify` | `fetch-depth: 0` |
  | `release.yml` | `build` | none, on both runners |

## Decisions

- **The upgrade is its own branch.** The release path is proven on v5 by the `v0.0.0`
  rehearsal and the `v0.0.1` release. Changing it next to anything else would make a
  failure ambiguous, and this is the code that decides what ends up inside published
  archives.
- **Verification covers two of the three forms, and the third is named rather than
  claimed.** The two CI jobs run a plain checkout on Linux and on Windows, and a
  mismatched tag exercises `fetch-depth: 0` in `verify`. What neither reaches is the
  combination `build` uses: a shallow checkout of a *tag* ref. CI runs on `pull_request`,
  so its shallow fetch takes the merge ref, and checkout builds a different refspec for
  each; calling those equivalent would be wrong. `build` only runs after `verify` passes,
  which needs a version that matches the workspace, so nothing short of a real release
  reaches it.
- **That residual gap is accepted, because its failure mode is safe.** If v7 mishandled a
  tag ref, `build` would fail before `publish` ever runs: no archive, no release, nothing
  to retract. The cost is a failed release run and a re-tag, against the cost of
  publishing a version whose only content is a workflow pin in order to test it.
- **No full release rehearsal.** The `main`-ancestor check added in #11 refuses a tag on a
  branch commit, which is what the `v0.0.0` rehearsal relied on. Rehearsing now would mean
  publishing a real version whose only content is a workflow pin, which the changelog
  convention says should not exist.
- ➕ **The Issue's fourth acceptance criterion was written for a probe that cannot exist.**
  It asked for a mismatched tag to pass the ancestor check and then fail on the version.
  Passing the ancestor check needs a tag on `main`, where the workflow still carries v5
  until this merges, so the probe would have tested the old version. The probe was run on
  a branch commit instead, which still reaches and completes the checkout under test.

## Rejected alternatives

- Pinning to the `v7` tag instead of its SHA. The repository pins by SHA precisely because
  a tag can move.
- Upgrading only the CI pins and leaving the release workflow on v5. Two versions of the
  same action is worse than either version alone: it doubles what has to be reasoned about
  and the release path is the half that matters more.
- Cutting `v0.0.2` to rehearse the release end to end. A release whose changelog entry
  would be workflow plumbing contradicts the convention in `CONTRIBUTING.md`, and the
  archives would be identical to `v0.0.1`.

## Implementation steps

- [x] Move all four `actions/checkout` pins to the v7.0.1 SHA with the version in a
      trailing comment.
- [x] Confirm no other action is behind its current release: `Swatinem/rust-cache`
      v2.9.2, `actions/upload-artifact` v7.0.1 and `actions/download-artifact` v8.0.1 are
      each their newest.
- [x] Verify both CI jobs pass, exercising a plain checkout on Linux and on Windows.
- [x] Verify the `fetch-depth: 0` form. The probe was tagged on a branch commit, so it
      failed at the ancestor check and the version check was skipped. The log shows
      checkout v7 running with `fetch-depth: 0`, the fetch of `main` completing without a
      git error, and the failure coming from the check's own message, so the form under
      test was exercised. See the decision above on why the criterion as originally
      worded could not be met.
- [x] ➕ ⚠️ Leave `build`'s combination of a tag ref and a shallow fetch unexercised, and
      say so. It is unreachable without a real release, and its failure mode is a failed
      `build` before anything is published.
- [ ] External review round, then act on the findings.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`: 226 tests, all passing.
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Both workflows still parse as YAML, and the diff is exactly the four pin lines.
- [x] Both CI jobs pass on the Pull Request.
- [x] The probe published nothing: `build` and `publish` were skipped, and the tag was
      deleted afterwards. Only `v0.0.1` remains.

## Post-completion

- Merge the Pull Request with squash after review conversations and checks are complete.
