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
- Refresh on startup, successful file opening, window activation and File-menu
  opening. Coalesce pending checks per path and identify results by path across
  reordered history; old blocked paths must not prevent checking new paths. Keep popup entries stable until
  dismissal; the next opening uses the latest completed availability snapshot.
- Refresh availability without changing saved history. Keep ordering, labels and
  opening hints paired after filtering, and reject obsolete check results.
- Keep the current file visible; #59 remains a separate task.

## Rejected alternatives

- A global ten-worker cap was rejected after review demonstrated that ten blocked
  old paths could permanently hide newly opened local files. Pending checks are
  bounded per distinct path instead; abandoned checks are not cancellable, and
  repeated changes to distinct blocked paths can retain additional detached threads.
- Passing file availability through the analysis/DSP delivery pipeline would
  couple menu refreshes to file analysis and still leave reappearing queued history
  behind blocked checks. Independent per-path scheduling fixes the starvation at
  its source without changing the analysis contract.

- Checking paths when rendering or synchronously opening the menu can block the
  UI on network mounts.
- Removing unavailable entries from session history prevents temporarily offline
  files from reappearing.

## Implementation steps

- [x] Extend recent availability lifetime and refresh policy beyond the start page.
- [x] Use the same filtered entries for the menu, start page and its shortcuts.
- [x] Cover unavailable files, empty results, reappearance and changed history.
- [x] Update architectural context and the changelog.
- [ ] Complete validation and external review; address substantive findings.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [x] Verify shared ordering, hints, refreshes and nonblocking checks with focused tests.
- [ ] Verify native menu/start-page behavior where a real GPU session is available;
  record any environment limitation precisely.
- [ ] Read-only external review with GPT-5.6 Sol at High, repeated until clean.

## Validation notes

- The 13 recent-file tests pass, including mixed regular/missing/directory entries,
  disappearance and reappearance, label disambiguation, raw hints, reordered
  history, per-path coalescing, shutdown without joining, and a full old history
  of blocked probes followed by a new local path.
- Initial read-only review found one P2 starvation case in the global worker cap.
  Accepted and fixed through independent per-path scheduling with a regression
  test. The suggested analysis-delivery integration was declined for the reasons
  above; the final round must challenge that alternative decision.
- Local environment: CachyOS, system-repository rustup and Vulkan headers, pinned
  Rust/Cargo 1.97.1 with rustfmt and Clippy; Intel Iris Xe using Mesa Vulkan.

## Post-completion

Keep the PR Draft for owner feedback. Request full remote validation when Ready;
merge only after the current revision passes all required platform checks.
