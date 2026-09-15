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

- Reuse the runner's Ubuntu source definition, preserving its mirror, suites,
  components and signing configuration.
- Restrict both update and installation to that source and fresh package lists.
- Fail on any required-source update error, missing source configuration or
  package installation failure.

## Rejected alternatives

- Do not ignore APT failures or disable signature/hash checks.
- Do not delete or rewrite the runner's unrelated source definitions.

## Implementation steps

- [ ] Isolate Ubuntu sources and package lists for dependency installation.
- [ ] Add failure-path coverage and wire it into Linux quick/full CI.
- [ ] Document the source policy and validate it with real APT.
- [ ] Complete local checks and independent review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] Regression checks for broken unrelated sources and required-source failures.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the standard checks
- [ ] External review returns no substantive findings.

## Post-completion

Merge through the PR after owner acceptance and successful full CI.
