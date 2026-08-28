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
- v6 moved credentials out of the shared git config into a separate file and added Node
  24 support. v7 blocks checking out a fork's pull request under `pull_request_target`
  and `workflow_run`, and moved the action to ESM. The v7 hardening does not apply here:
  this repository uses neither trigger and takes no fork pull requests.
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
- **Verification leans on equivalence rather than on a release.** The two CI jobs run a
  plain checkout on Linux and on Windows, which is exactly what `build` does on the same
  two runners with the same inputs. What CI cannot reach is `fetch-depth: 0`, and a
  mismatched tag reaches it: `verify` checks out, fetches `main`, runs the ancestor check
  and only then fails on the version. Between them the two cover every form in use.
- **No full release rehearsal.** The `main`-ancestor check added in #11 refuses a tag on a
  branch commit, which is what the `v0.0.0` rehearsal relied on. Rehearsing now would mean
  publishing a real version for no user-visible change, which the changelog convention
  says should not exist. The residual gap is `build`'s checkout, and it is covered by
  equivalence with CI rather than left unstated.

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
- [x] ⚠️ Verify the `fetch-depth: 0` form. The probe was tagged on a branch commit, so it
      failed at the ancestor check rather than reaching the version check. That still
      covers what this Issue needs: the log shows checkout v7 running with
      `fetch-depth: 0`, the fetch of `main` completing without a git error, and the
      failure coming from the check's own message. Reaching the version check would have
      required tagging a commit on `main`, where the workflow still carries v5 until this
      merges.
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
