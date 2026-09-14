# Issue #65: Consistent available recent files

Resolves [#65](https://github.com/o-kos/argand/issues/65).

## Overview

File > Recent currently reads saved history directly, while the start page hides
unavailable paths. Both surfaces will consume the same availability-filtered
snapshot in recent order without changing saved history or raw opening hints.
Hiding the active file (#59) is outside this issue.

## Context

`recent.rs` already checks regular-file availability in independent background
threads. Its lifetime is currently restricted to the start page. `shell.rs` owns
history updates and both recent-file surfaces. Network metadata checks must never
run or be joined on the UI thread.

## Decisions

- Share one toolkit-neutral availability model across both surfaces, including
  startup with a file argument and subsequent successful file openings.
- Refresh availability without changing saved history. Keep ordering, labels and
  opening hints paired after filtering, and reject obsolete check results.
- Keep the current file visible; #59 remains a separate task.

## Rejected alternatives

- Checking paths when rendering or synchronously opening the menu can block the
  UI on network mounts.
- Removing unavailable entries from session history prevents temporarily offline
  files from reappearing.

## Implementation steps

- [ ] Extend recent availability lifetime and refresh policy beyond the start page.
- [ ] Use the same filtered entries for the menu, start page and its shortcuts.
- [ ] Cover unavailable files, empty results, reappearance and changed history.
- [ ] Update architectural context and the changelog.
- [ ] Complete validation and external review; address substantive findings.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Verify shared ordering, hints, refreshes and nonblocking checks with focused tests.
- [ ] Verify native menu/start-page behavior where a real GPU session is available;
  record any environment limitation precisely.
- [ ] Read-only external review with GPT-5.6 Sol at High, repeated until clean.

## Post-completion

Keep the PR Draft for owner feedback. Request full remote validation when Ready;
merge only after the current revision passes all required platform checks.
