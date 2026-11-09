# Issue #189: Conversion test portability

Resolves #189 through Draft PR #224.

## Overview

The first full three-platform run exposed two test-fixture assumptions. Keep the
production conversion implementation unchanged and make each test exercise its
actual platform contract.

## Roles and risk

- Class: B, two local test fixtures with known platform invariants.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Evidence and decisions

- The caption pointer regression uses Linux custom-caption coordinates. Windows
  and macOS render the stock TitleBar, where its simulated drag reaches the
  unimplemented GPUI TestWindow::start_window_move. Compile that regression on
  Linux only; keep platform-neutral close/unsaved/cancellation tests everywhere.
- The FLAC stamp regression retains a metadata-only mmap reader while truncating
  its source. Windows refuses truncation of a mapped file (error 1224). Drop that
  reader after copying metadata. Retain the real source-byte mutation, SourceChanged
  assertion and unchanged destination assertion; do not fake timestamps or sleep.
- No production code, toolkit dependency, UI behavior or acceptance rule changes.

## Implementation

- [ ] Scope the custom-caption pointer regression to Linux.
- [ ] Release the metadata-only reader before mutation.
- [ ] Validate focused tests, normal compilation and the full local gate.
- [ ] Move the finished plan before independent final review.
- [ ] Record review and three-platform CI results.

## Validation

- [ ] `cargo check -p argand --locked`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] Verify production diff is empty, retaining the existing current-code release.
- [ ] Ready full CI: Linux, Windows, macOS and `ci/full` all succeed.
