# Issue #180: Release workflow fails: verify cannot build the GUI without GPUI system libraries

Resolves #180.

Class C (workflow configuration, no behavioural code). Implementer: Claude in the session. Reviewer: `gpt-6-luna` medium, agreed with the owner.

## Overview

The `v0.1.0` tag ran `release.yml`, whose `verify` job failed in `cargo test --locked` because the GPUI application now in the workspace links against system libraries the job never installs. No release was published. The owner chose to leave `v0.1.0` as a tag without a release and publish v0.1.1 once the workflow is fixed.

## Context

- `ci.yml` runs on `ubuntu-24.04`, tests source isolation with `.github/scripts/test-ubuntu-packages.py` and installs the GPUI libraries with `.github/scripts/install-ubuntu-packages.sh`.
- `release.yml` `verify` ran on `ubuntu-latest` without either step. Its `build` job ran `cargo build --release --locked` for the whole workspace on Linux and Windows, although only the `aspec` binary of `argand-cli` is archived and published.

## Decisions

- `verify` runs on `ubuntu-24.04` and repeats the two `ci.yml` steps verbatim before testing, so a release is tested exactly as `main` was.
- `build` builds `-p argand-cli` only. It ships `aspec` alone; building the GUI there would need the same libraries on Linux and cost time on Windows for an artefact that is thrown away. Shipping Argand itself is #86.

## Rejected alternatives

- Testing only the non-GUI crates in `verify`: a release would then be tested less than `main`.
- Moving the existing `v0.1.0` tag: the owner chose v0.1.1.

## Implementation steps

- [x] `verify` on `ubuntu-24.04` with source isolation and GPUI packages.
- [x] `build` limited to `argand-cli`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo build --release --locked -p argand-cli` builds `aspec` and it reports its version.
- [ ] The release Pull Request for v0.1.1 merges and its tag publishes the release.

## Post-completion

- Release v0.1.1 through `chore/release-v0.1.1`; the owner or, with the owner's permission, Claude tags it.
