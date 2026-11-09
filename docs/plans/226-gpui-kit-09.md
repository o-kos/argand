# Issue #226: Upgrade GPUI Kit to 0.9.1

Resolves #226.

## Overview

Upgrade the single `gpui-kit` facade dependency from 0.6 to 0.9.1 and migrate
application integration to the supported APIs. Reassess recorded toolkit
workarounds against the selected registry sources instead of preserving old
solutions by default. DSP semantics and unrelated work, including #222, are out
of scope.

## Roles and risk

- Class: A: toolkit integration spans more than five code files and affects window
  ownership, overlays, input routing and texture-lifetime regression coverage.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Context

- [UI architecture](../architecture/ui.md) and
  [control inventory](../ui/124-inventory.md) describe the current integration and
  retain the historical 0.6.6 audit.
- The owner approved target 0.9.1 and project-local installation of both upstream
  skills, including their complete references. Their unchanged source and known
  documentation caveats are recorded in
  [provenance](../../.agents/skills/GPUI-KIT-PROVENANCE.md).
- Upstream v0.9.1 resolves to `39ab849a93b5f15507b969164d49891db8d5b470`.
  Registry sources selected by Cargo.lock remain authoritative for behavior.
- Main was refreshed before branching. The owner's pre-existing AGENTS guidance
  is already represented by the documentation consolidation on main. Its original
  local patch is retained separately; it is not part of this change.

## Decisions

- Keep one crates.io facade and commit the complete lockfile. Do not add direct
  toolkit-layer dependencies, registry patches or lint suppressions.
- Apply upstream coding/design guidance at changed integration seams. Do not
  bundle wholesale naming, pixel-unit, public-field or visual refactors.
- Prefer supported toolkit composition. A retained custom control requires a
  fresh inventory entry with the standard alternative, exact-source limitation,
  composition assessment and focus/keyboard/pointer/disabled regression coverage.
- Before implementing substantial architecture or UX changes, present small,
  concrete options and a recommendation to the owner. Initial checkpoints are:
  1. Root automatic overlay hosting and the standard window frame versus the
     existing frame, including resize bounds, rounding and caption behavior.
  2. Standard menu activation/navigation behavior, including submenu activation,
     versus the previous Right-only contract.
  3. Numeric editing if standard composition cannot preserve the approved
     transaction and press/repeat interaction.
- Continue independent API migration while a decision is pending. Existing UX is
  evidence to reassess, not a prohibition on better standard solutions.
- The initial plan/unchanged-upstream-skills commit contains documentation only.
  Its scoped validation is source equality, provenance/link review and diff/scope
  checks; compilation and the full gate are not run manually for that initial
  documentation-only push. The installed hook remains enabled and any checks it
  runs must pass. Every code push requires the full local gate.

### Owner-approved frame decision

The owner approved the configured standard Root/frame preview: retain Argand
colors, set the supported window radius token to 8 logical pixels, and preserve
the existing caption controls. The standard Linux shadow inset increases from
12 to 20 logical pixels, reducing content by 16 pixels per axis at the same outer
window size. Remove the custom frame/resize implementation; verify resize,
maximize, restore and tiling. This decision does not approve a caption or menu UX
redesign. The isolated preview backend fixture must never enter production.

The owner also approved standard submenu activation with Enter as well as Right.
Space remains intentionally inert in the application menu.

## Rejected alternatives

- Selecting GPUI and gpui-component separately would recreate graph drift.
- Blindly retaining every 0.6 workaround or silently changing behavior to satisfy
  0.9 APIs would avoid the required component and owner decisions.
- Rewriting unrelated controls to satisfy every upstream style example would
  obscure migration regressions and exceed this Issue.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| One 0.9.1 facade and compatible graph | Manifest, lockfile and Cargo dependency inspection | Passed: facade 0.9.1, GPUI pre 0.3.12 | Default backend; no backend substitution |
| Supported Root/overlay ownership | Exact-source review and focused regression tests | Automated checks passed; approved standard composition implemented | Native compositor acceptance remains pending |
| Menus, nested Selects and panels preserve agreed interactions | Input/focus regression tests and native application scenarios | Automated input/focus tests passed | Native checks must be observed, not inferred from headless tests |
| Numeric transactions and disabled controls remain correct | Settings/conversion regression coverage and native checks | Automated transaction/disabled/repeat tests passed | Native interaction checks remain pending |
| Custom-control inventory reflects selected graph | Each retained exception reviewed against exact locked source | Completed for the selected graph | Do not generalize tag evidence to a different registry version |
| Theme, window chrome and multiwindow input work | Targeted automated tests and native scenarios where available | Automated geometry/theme/window-isolation tests passed | Native Linux and Windows/macOS runtime acceptance remain unperformed |
| Texture retirement retains originating-window ownership and intervening-frame lifetime | Current texture-retirement and multiwindow regression tests | Existing snapshot-retirement and window-isolation tests passed; retirement code unchanged | Headless tests do not establish GPU completion or native multiwindow lifetime |
| Current code passes required checks | Full local gate, then release build | Full gate and subsequent release build passed | Server CI and native acceptance reported separately |
| Independent review and current-revision evidence | Separate reviewer using neutral exact snapshot | Rounds 1–2 completed; missing changelog finding accepted and addressed; round 2 found nothing substantive | Native acceptance remains pending; plan stays active |

## Implementation steps

- [x] Refresh main, preserve unrelated work and create a focused branch.
- [x] Record Class A scope, roles, checkpoints and upstream-skill provenance.
- [x] Open an early Draft Pull Request linked to #226.
- [x] Update facade requirement and lockfile; inspect the selected dependency graph.
- [x] Migrate unambiguous keybinding, keystroke, modifier, scroll and theme APIs.
- [x] Resolve Root/frame and menu checkpoints with source-backed owner decisions.
- [x] Migrate approved integration and update focused Root, overlay, menu, input
      and ownership regression tests. Record the toolkit context-menu retention
      fix in the inventory; native menu lifecycle and animation checks remain pending.
- [x] Reassess every relevant inventory exception; remove superseded paths and
      update architecture/inventory documentation alongside the code.
- [x] Complete local gate and build the current release binary.
- [ ] Run permitted native acceptance scenarios and record actual limitations.
- [x] Prepare neutral current-revision evidence for independent review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Installed upstream skill trees match the exact v0.9.1 archive byte-for-byte;
      bundled relative references were checked and source-only links documented.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the full gate passes
- [x] Focused automated Root, overlay, menu, input and multiwindow regressions.
- [ ] Native animation and context-menu lifecycle checks; no dedicated current
      runtime observation has been made.
- [ ] Native application acceptance using the current release binary, disposable
      signal data and supported configuration/session isolation. Exercise startup,
      resize/maximize/restore, nested lists, panel transactions, keyboard focus,
      pointer/wheel isolation, numeric disabled/repeat states and theme switching.
      Record unavailable interaction capabilities and untested platforms explicitly.
- [x] Independent review of the exact current diff and validation evidence:
      round 1's missing changelog finding was accepted and addressed; round 2
      returned no substantive findings.

## Post-completion

Ready status, full three-platform CI, owner acceptance and squash merge follow the
repository workflow. Migration completion does not authorize a release.

### Current implementation evidence

The automatic Root/frame integration is implemented with the approved radius and
colors; captions retain their existing shared implementation. A stateless RootPlugin
observes bubbled frame presses through standard composition, without drawing or
resizing. The locked WindowBorder handler does not stop propagation. The callback
resolves Shell from Root without holding a Root update borrow or capturing a strong
Shell; child popup handlers retain their input first. Shared local/global backdrop
geometry uses public window_paddings plus the locked private border/hit-band widths.
Geometry tests cover restored, tiled and server-decorated states; GPUI dispatch
tests cover uncovered edge, content and blocking popup presses. Headless TestWindow
uses server decorations, so native resize/maximize/restore/tiling remain unverified.

An initial application suite exposed six Enter/focus regressions. Base 0.9.1
PopoverContent deliberately binds Enter to NoAction instead of inheriting trigger
activation. PinnedHint now explicitly binds and handles Confirm in its existing
scope, preserving its Enter-close contract while descendant fields/lists/actions
handle Confirm first. The current full application suite passed all 615 tests,
including the Root borrowing regression. The full local gate exited successfully:
rustfmt, Clippy and 1144 tests passed, with one existing ignored manual release-mode
conversion cost measurement. The subsequent release build also exited successfully.
Only documentation changed after that code snapshot; no runtime code changed.
The current release's help/version commands exited successfully. A bounded launch
with a disposable I/Q capture, portable default configuration and isolated XDG
config/state/cache/data roots stayed alive for 12 seconds with no logged error,
then received SIGTERM from the harness. This establishes startup smoke only;
neither graceful close nor visible output or interaction was observed.
No native application interaction or compositor acceptance has been performed.
