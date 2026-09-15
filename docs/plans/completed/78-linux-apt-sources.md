# Issue #78: Isolate Linux CI dependency sources

Resolves [#78](https://github.com/o-kos/argand/issues/78).

## Overview

Install GPUI's Ubuntu dependencies without contacting unrelated package feeds
on the hosted runner. A broken Chrome index must not prevent Rust validation.

## Context

The Linux job in `.github/workflows/ci.yml` currently updates every configured
APT source. Both quick and full validation use this step. Authentication and
checksum verification must remain enabled, and required-source failures must
remain fatal.

## Decisions

- Pin the Linux job to Ubuntu 24.04, whose deb822 source is
  `/etc/apt/sources.list.d/ubuntu.sources`; upgrading the image requires checking
  this explicit contract. Reuse that definition, preserving its mirror, suites,
  components and signing configuration.
- Restrict both update and installation to that source and fresh package lists;
  keep downloaded package archives in the same temporary workspace.
- Fail on any required-source update error, missing source configuration or
  package installation failure.

## Review

- Accepted the first-round finding that production still used the shared archive
  cache while tests supplied a private one. Added an explicit archive-directory
  override and a regression that fails before the change when the inherited
  archive path is unusable. No findings were declined.

## Rejected alternatives

- Do not ignore APT failures or disable signature/hash checks.
- Do not delete or rewrite the runner's unrelated source definitions.

## Implementation steps

- [x] Isolate Ubuntu sources and package lists for dependency installation.
- [x] Add failure-path coverage and wire it into Linux quick/full CI.
- [x] Document the source policy and validate it with real APT; all seven GPUI
  dependency packages installed successfully in an Ubuntu 24.04 container.
- [x] Complete local checks and independent review.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Nine real-APT regression tests pass as an unprivileged user in Ubuntu 24.04:
  a broken unrelated index, stale third-party indexes, required index/package hash
  failures, unsigned metadata, unavailable required feed, missing package, and
  absent/empty source definition, and an unusable runner archive cache. Tests use signed local fixtures and download-only
  mode; they do not install packages on the host.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the standard checks
- [x] Linux Draft CI: APT regression tests, real dependency installation, formatting,
  Clippy and `ci/quick` passed on the GitHub Ubuntu 24.04 runner.
- [x] External review returns no substantive findings in the second round.

## Post-completion

Merge through the PR after owner acceptance and successful full CI.
