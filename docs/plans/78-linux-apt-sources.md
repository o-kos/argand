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
- Restrict both update and installation to that source and fresh package lists.
- Fail on any required-source update error, missing source configuration or
  package installation failure.

## Rejected alternatives

- Do not ignore APT failures or disable signature/hash checks.
- Do not delete or rewrite the runner's unrelated source definitions.

## Implementation steps

- [x] Isolate Ubuntu sources and package lists for dependency installation.
- [x] Add failure-path coverage and wire it into Linux quick/full CI.
- [x] Document the source policy and validate it with real APT; all seven GPUI
  dependency packages installed successfully in an Ubuntu 24.04 container.
- [ ] Complete local checks and independent review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Eight real-APT regression tests pass as an unprivileged user in Ubuntu 24.04:
  a broken unrelated index, stale third-party indexes, required index/package hash
  failures, unsigned metadata, unavailable required feed, missing package, and
  absent/empty source definition. Tests use signed local fixtures and download-only
  mode; they do not install packages on the host.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the standard checks
- [ ] External review returns no substantive findings.

## Post-completion

Merge through the PR after owner acceptance and successful full CI.
