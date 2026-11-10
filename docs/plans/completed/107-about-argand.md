# Issue #107: About Argand panel

Resolves #107: https://github.com/o-kos/argand/issues/107.

## Overview

Add a compact About panel like File information. A standard ghost Button at the
right end of the title bar, before native window controls, shows the existing
outline Tag icon and Cargo version without a `v` prefix. The main menu provides
`About Argand…`; no shortcut or independent application window is added.

## Roles and risk

- Class A: build identity, overlay integration and more than five code files.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, separate read-only context.

## Decisions and architecture

- Reuse the retained shared `PinnedHint`, standard Popover, Button, Tooltip and
  keyboard-accessible Base Link on the committed GPUI Kit 0.9.1 graph.
- Button text uses the empty status bar's extra-small normal muted typography.
  Reserve its measured width symmetrically so the window title remains centered.
- Use approved Argand artwork, emphasized product name, secondary version and
  the Cargo description: real and complex signals. The owner accepted a
  one-logical-pixel upward offset of the Version line. This is an optical
  refinement, not a universal exact glyph-bounds guarantee.
- The viewport-bounded 20-rem panel shows Cargo version, description, MIT licence,
  repository, short Git hash, exact tag or `untagged`, and commit date with timezone.
  Replace the timestamp's `T` with a space for display.
- Build-time Git metadata has honest unavailable fallbacks. Track HEAD, active
  refs, shared refs/tags and packed refs without watching the entire Git directory.
  Git subprocesses clear repository-local inherited environment variables; no Git
  work or network request runs when opening About.
- Tooltip: `Argand <version>` with smaller muted tag/hash/date details. Suppress
  it whenever any shared pinned panel is open, without changing those panels.
- One retained About instance closes on Escape, any panel click, focus leaving its
  subtree or window deactivation. Link activation opens the repository once and
  closes the panel. Child focus remains inside the surface.
- Repository Link uses the semantic link token, toolkit blue-400 in dark mode and
  blue-600 in light mode. It is the only current application consumer of this token.

## Rejected alternatives

A separate native window or centered Dialog contradicts the File information
analogy. Runtime Git, duplicated package metadata and nearest-release tags would
misidentify the running build. Legacy component Link on the locked toolkit lacks
keyboard activation; the standard Base Link supplies the required contract.
No custom widget, updater, network backend or toolkit dependency was introduced.

## Implementation and acceptance

- [x] Capture Cargo/build metadata with exact-tag, rebuild and missing-Git behavior.
- [x] Integrate the standard title-bar button, menu, panel and focus lifecycle.
- [x] Cover Git isolation and production-path keyboard/pointer/focus interactions.
- [x] Add tooltip suppression across Analysis, File information, Selection and About.
- [x] Update architecture, control inventory and changelog.
- [x] Complete three independent review rounds and disposition the substantive finding.
- [x] Obtain owner acceptance of production appearance and the final Version offset.
- [x] Integrate current upstream context-menu and test-layout/retirement work.
- [x] Pass current-source formatting, Clippy and full workspace tests.
- [x] Prepare the final committed source for release/native refresh and full CI;
  exact final artifact identity and current remote results are recorded in the PR.

## Validation and review evidence

Ordinary `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked` and
`cargo test --locked` pass on the integrated source: 883 tests, no failures or
ignored tests. Counts are app 514, CLI 50, CLI integration 24, core 36, DSP 97,
edit 12 and I/O 150. CLI/core test names were checked against the actual integrated
source. Earlier shared-artifact identity ambiguity is historical; legitimate
upstream integration aligned those sources. No cache cleanup, timestamp tricks,
compiler flags or dependency changes were used.

Production acceptance used the real release binary on an isolated system-Xvfb
X11 display, with isolated application state and no physical-desktop input.
Evidence covers toolbar/menu opening, tooltip metadata, repeated invocation,
Escape/outside/deactivation, child focus, keyboard/pointer repository activation,
File information comparison, narrow windows and both themes. The final Version
refinement was captured at 100% and 125%; the owner accepted it. This does not
establish native Windows/macOS or physical Wayland appearance. Platform builds and
headless logic are covered by the required current Linux/Windows/macOS CI matrix.

Round 2 accepted one substantive Git-environment isolation finding. Production and
fixture commands now share isolation, and a hostile-environment subprocess
regression proves the caller's refs, config, index and worktrees remain unchanged.
There were no declined findings. Round 3 found no substantive source findings on
its exact frozen snapshot. The coordinator separately assessed the later bounded
owner-requested Version offset and upstream test-only conflict resolutions; no
fourth review was started. Raw review, snapshot and validation limits are retained
in the external handoff. The final committed release and integration smoke check
are refreshed after this record; their exact result and identity belong in the PR.

## Post-completion

The owner authorized autonomous current full CI, squash merge and safe cleanup.
Require successful current `ci/full`, an up-to-date branch and resolved review
conversations before merging. Delete only the accepted feature ref/worktree after
checking its exact tip, clean state and unique data. Preserve the independent
main checkout, its unrelated work and the owner's running application.
