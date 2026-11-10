# Issue #107: About Argand panel

Resolves #107: https://github.com/o-kos/argand/issues/107.

## Overview

Add a compact About panel, composed like File information, with a permanent
right title-bar button before the window controls and a direct application-menu command. Show the approved
icon, product name, Cargo version, description, MIT licence, repository URL and
build commit identity. No keyboard shortcut or independent application window.

## Roles and risk

- Class: A, because build metadata, overlay lifecycle and integration are expected
  to span more than five code files or roughly 400 code lines.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Context

The implementation started on GPUI Kit 0.6.6. Before final validation, upstream
Rust 1.98.1 and GPUI Kit 0.9.1 / standard window frame work were merged.
The current integration uses the locked 0.9.1 graph and updated toolkit guides. Reuse `PinnedHint`,
standard Popover, Button and the keyboard-accessible Base Link, with existing
panel surface styling. The main working directory belongs to separate work;
the owner explicitly authorized a separate worktree after its branch changed.

## Decisions

- Right title-bar logo button and main-menu command open one retained panel. No shortcut.
- Hover shows a tooltip containing the package version, exact HEAD tag (or `untagged`), short hash and
  commit timestamp. Hover does not open the panel.
- The panel closes on any click within it (the repository Link also opens its
  URL), Escape, focus leaving its entire subtree, or window deactivation.
- Existing panel/menu opening transitions close About and restore normal focus.
- Version, description, licence and repository come from Cargo package metadata.
- Git metadata is captured at build time only: short HEAD hash, commit timestamp
  in ISO 8601 with offset, exact tag or `untagged`. No nearest-tag substitution.
- Rebuild tracking covers worktree HEAD, its symbolic ref, shared refs/tags and
  packed refs; missing Git/source archive data has explicit unavailable labels.
- Owner correction: use a standard compact ghost logo Button at the right end of
  the title bar, a right-anchored panel, an emphasized product heading and secondary
  version. Format the commit date with a space instead of `T`, preserving its timezone.
- A production-binary run on an isolated Xvfb X11 display verified the earlier UI.
  The owner requested the visual corrections above after inspecting that evidence;
  rebuild and refresh native acceptance for the changed UI. Physical Wayland and
  other-platform acceptance are not established by the X11 run.

## Rejected alternatives

- A separate native About window: contradicts the transient-surface requirement.
- A centred Dialog: superseded by the owner's File information panel analogy.
- Legacy component Link: locked 0.9.1 supports pointer activation only; use the
  standard Base Link with its focus, keyboard and accessibility contract.
- Runtime Git subprocesses or a duplicate hardcoded package version.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Toolbar and main menu open the same compact panel | Production-path GPUI integration tests; native interaction | Headless passed | Native interaction unavailable |
| Approved icon and complete Cargo/build information | Metadata tests and production composition inspection | Automated/static passed | Native appearance pending |
| Tooltip gives exact tag, hash and commit date | Build metadata tests and tooltip text assertion | Passed | Native tooltip pending |
| Repeated invocation never duplicates the panel | Integration test | Passed | Native pending |
| Any surface click, repository activation and Escape dismiss | Pointer and keyboard integration tests | Passed | Native pending |
| Focus leaving subtree and window deactivation dismiss | GPUI focus/activation integration tests | Passed | Native compositor pending |
| System, light and dark follow existing application theme | Semantic tokens and rendered themed scenarios | Headless light/dark passed | Native visual acceptance pending |
| Linux, Windows and macOS build and logic checks | Full remote CI on current branch | Pending | Native platform acceptance remains separate |

## Implementation steps

- [x] Add Cargo/build metadata with exact-tag and missing-Git behavior.
- [x] Compose the About panel and integrate toolbar/menu/lifecycle.
- [x] Add meaningful metadata and production-path interaction coverage.
- [x] Update architecture, control inventory and changelog.
- [x] Run formatting, lint and tests on GPUI Kit 0.9.1; record evidence/limits.
- Build the release from the final committed source before review handoff;
  retain its exit status and exact artifact identity in the external packet.
- [x] Prepare exact neutral independent review handoff; round 1 has no findings.
- Full remote CI must succeed without merging; exact revision and results are
  recorded in the Pull Request, keeping its CI evidence current.
- [ ] Native acceptance and completed-plan move remain pending while unavailable.

## Validation

- [x] `cargo fmt --all -- --check` (Rust 1.98.1 after upstream integration)
- [x] `cargo clippy --all-targets --locked` (Rust 1.98.1 after upstream integration)
- [x] `cargo test --locked` (Rust 1.98.1 after upstream integration)
- `cargo build --release --locked` follows the checks above and this final
  source commit; current-revision result and identity are in the review packet.
- [x] Metadata tests, rendered pointer/keyboard/focus integration tests.
- Independent review: round 1 is clean; full toolkit-integration revalidation
  and its exact snapshot/result are recorded in the Pull Request review handoff.
- Full Linux/Windows/macOS remote CI: current-revision run/result is recorded in
  the Pull Request. Native acceptance remains a separate pending requirement.

Current GPUI Kit 0.9.1 / Rust 1.98.1 formatting, lint and workspace tests pass:
1,169 tests passed, with one pre-existing ignored test. Six About production-path
UI scenarios cover both entry points, Escape, repeated opening, pointer/Enter/Space
repository activation, focus subtree transitions, window activation, outside
presses and light/dark layout. The harness uses the standard Root and Kit's full
key-press pairs. The scoped Link Enter binding remains necessary because
PinnedHint's explicit Confirm binding is deeper than PopoverContent's binding.

The unchanged metadata module also passes four standalone tests without GPUI.
A disposable Cargo project verifies actual rebuilds for same-ref commits, first
tags, packed refs creation/removal and linked-worktree shared refs; missing Git
returns explicit unavailable values without a panic. Raw validation and the full
source snapshot are retained in the external review packet. The release build
follows the final source commit, so its embedded identity names the submitted
revision; the artifact identity and exit status are retained in that packet.

Independent review round 1 on the earlier toolkit baseline found no substantive
metadata, lifecycle, UI-composition, architecture or validation-evidence findings.
There were no accepted/rejected findings to carry forward. The GPUI Kit 0.9.1
integration requires full current-snapshot code review; round 1 does not establish
the new snapshot. Current review and remote CI outcomes are retained in the Pull
Request without subsequent commits that would invalidate the checked revision.
Native acceptance remains incomplete.

## Post-completion

The owner authorized autonomous completion, current full CI, squash merge and safe
branch/worktree cleanup after accepting the final native appearance. Preserve the
independent main checkout and its running application.

## Current validation refresh

The earlier 1,169-test gate predates the accepted round-2 Git-environment isolation
finding and the owner's subsequent UI correction. It is historical evidence, not
validation of the current source. Round 2 accepted one Git-environment isolation
finding, with no rejected findings. Both production metadata commands and fixture
commands now clear repository-local Git environment variables. A subprocess
regression verifies that hostile inherited settings cannot redirect fixture writes
or change the caller repository.

After that correction and the title-bar refinement, ordinary `cargo fmt --all --
--check`, `cargo clippy --all-targets --locked` and `cargo test --locked` pass in the
owner-approved shared baseline. That snapshot recorded 1,171 tests passed,
with no failures and 1 ignored test. These include the isolation regression,
readable commit-date and version-tooltip assertions, and the production-path About
pointer, keyboard, focus, dismissal and right-anchored geometry scenarios.

The normal commands use the installed toolchain and stock rustup auto-install
setting disabled; no executable selectors, shell dispatch functions, PATH changes
or compiler wrappers are needed. Saved compiler diagnostics from an earlier
thiserror build can replay channel-sync/download text; that text alone does not
establish a new rustup installation attempt.

A production release from the committed source, refreshed native acceptance for
right placement, tooltip, themes and narrow-window layout, final independent round
3 and full remote CI remain pending. Keep the Pull Request Draft while these
requirements are incomplete. Exact snapshot, artifact and current check evidence
are retained in the external handoff and Pull Request.

## Owner visual refinement

Use the existing Argand logo artwork on the standard compact ghost title-bar
Button, retaining its accessible About name and hit target. Suppress its passive
tooltip while any retained shared pinned panel is open: Analysis settings, File
information, Selection or About. Do not change those panels' lifecycle or settings.
The tooltip title is `Argand <version>`; build tag/hash and commit date use smaller
muted supporting text. About uses a viewport-bounded 20-rem panel, the accurate
real-and-complex-signal description from Cargo metadata, and a smaller right-aligned
repository Link using the semantic theme link color. Refresh production native
appearance and cross-panel hover checks after rebuilding; final review remains
pending visual assessment.

The application uses the toolkit blue palette for its semantic link color
(`blue-400` in dark mode, `blue-600` in light mode). About is currently the
only application consumer of this token. The existing theme-switch test also
checks that the link color survives mode changes.

The subsequent canonical workspace test run after this visual refinement exited
successfully and passed all 629 application tests, including cross-panel hover
suppression. However, its CLI and core executables reported two combined test
names from the independent main checkout, absent from this branch. Therefore
that run does not establish a full current-source workspace gate. No cache
cleanup, timestamp manipulation, compiler flags or dependency changes were used
to force a different result; current-source workspace validation remains pending
resolution of the shared-artifact identity discrepancy.

## Final title-bar refinement

The owner selected the standard ghost Button with the existing toolkit outline
Tag icon and the package version without a `v` prefix. Its text uses the same
extra-small normal-weight muted typography as the empty status bar. The About
header distributes the title and version over the logo height; production
screenshots must confirm visible glyph alignment in both themes. Existing panel
behavior and tooltip suppression remain unchanged. The owner directed continuation without intervening in the shared cache.
Application tests and bounded production release/native iterations are allowed;
the workspace source-identity limitation remains unresolved and prevents claims
of final validation, Ready or review readiness.


## Accepted final native refinement

The owner accepted the outline Tag plus unprefixed package-version title-bar
button and requested only a one-logical-pixel upward offset of the Version line.
The standard relative text positioning implements that offset; logo, title and
line-box distribution remain unchanged. No universal pixel-perfect ink alignment
across fonts or scaling is claimed.

Current app-only validation passes formatting, Clippy and 629 binary tests. The
ordinary production release compiled only Argand. Actual isolated system-Xvfb
screenshots cover light/dark at 100% and 125%; button opening and Escape dismissal
were refreshed. At 125%, Version's visible bottom moved from one pixel below the
colored logo to its bottom; at 100% it moved one pixel upward as requested. The
owner accepted the result. Previous link, menu and cross-panel hover evidence
remains applicable to the unchanged interaction code. All owned native test
processes were stopped; the owner's running instance was preserved.

Independent round 3 returned no substantive findings on the preceding frozen
source. The coordinator assessed the bounded owner-requested Version offset;
that delta was not independently reviewed in a fourth round. Full workspace
source identity remains pending legitimate upstream integration and a fresh gate;
no manual shared-cache intervention is authorized or performed.
