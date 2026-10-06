# Issue #152: Initialize the cursor before a stationary Wayland entry

Resolves #152.

## Overview

The owner supplied a deterministic reproduction: launch Argand, hover the left
resize edge, close with Alt+F4 without moving the pointer, move right by at least
50 pixels, and relaunch without further motion. The pointer inside the new
window still shows the horizontal resize image.

## Context

- Class B. Implementer: Codex in this session, selected by the owner's request.
- Proposed reviewer: gpt-6-sol, medium; owner confirmation pending.
- Worktree: `/tmp/argand-152-startup`; branch based on current `origin/main`.
- The existing dirty #152 worktree is preserved. Its reverted close interception
  and geometry experiment are not part of this change.
- Locked GPUI Kit 0.6.6 exports `platform::current_platform` and
  `Application::with_platform`. They create the same backend as `application()`.
- In GPUI Linux 0.3.6, Wayland's cursor cache initially contains `None`. Pointer
  Enter reapplies a cursor only when that cache contains a style. Setting Arrow
  before opening any window seeds the cache; Enter then applies it using its
  valid serial. Normal frame hit-testing can subsequently select the edge cursor.
- The backend also synthesizes a MouseMove on Enter. Why that path fails to
  replace the stale image in the reported launch remains unverified; native
  reproduction is required before claiming this initialization fixes the issue.

## Decisions

- Initialize the Linux backend's cursor to Arrow before running the application.
  Keep one backend instance shared with the application. Use only facade APIs.
- Preserve resize geometry, hit-testing, and all close paths.
- No custom controls, dependency changes, lint suppressions, or registry edits.

## Rejected alternatives

- Delaying or denying window closure: changes close behavior and the previous
  experiment made the window unclosable.
- Relocating resize strips: does not address an image retained between launches.

## Implementation steps

- [ ] Seed the Linux platform cursor before opening the application window.
- [ ] Update the changelog and document the startup invariant.
- [ ] Run the local gate and build the release binary afterward.
- [ ] Verify the owner's exact stationary-pointer sequence natively.
- [ ] Agree the reviewer and complete external review.
- [ ] Move the plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Normal window: stationary restart inside the plot shows Arrow; each edge
      still shows its corresponding resize cursor and resizes when dragged.
- [ ] Maximized window: stationary startup and plot motion show Arrow and no
      resize strips appear. Restore and recheck edges.
- [ ] Native Windows/macOS verification of unchanged startup and edge behavior.

## Post-completion

- Merge only after owner acceptance and current `ci/full` success.
