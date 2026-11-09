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

- [UI architecture](../../architecture/ui.md) and
  [control inventory](../../ui/124-inventory.md) describe the current integration and
  retain the historical 0.6.6 audit.
- The owner approved target 0.9.1 and project-local installation of both upstream
  skills, including their complete references. Their unchanged source and known
  documentation caveats are recorded in
  [provenance](../../../.agents/skills/GPUI-KIT-PROVENANCE.md).
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
| Supported Root/overlay ownership | Exact-source review and focused regression tests | Automated checks passed; approved standard composition implemented | Owner accepted the result; individual compositor checks were not reported |
| Menus, nested Selects and panels preserve agreed interactions | Input/focus regression tests and native application scenarios | Automated input/focus tests passed | No individual native menu/panel scenarios were reported |
| Numeric transactions and disabled controls remain correct | Settings/conversion regression coverage and native checks | Automated transaction/disabled/repeat tests passed | No individual native numeric scenarios were reported |
| Custom-control inventory reflects selected graph | Each retained exception reviewed against exact locked source | Completed for the selected graph | Do not generalize tag evidence to a different registry version |
| Theme, window chrome and multiwindow input work | Targeted automated tests and native scenarios where available | Automated geometry/theme/window-isolation tests passed | Owner reported corrected resize behavior; other native/platform scenarios were not individually reported |
| Texture retirement retains originating-window ownership and intervening-frame lifetime | Current texture-retirement and multiwindow regression tests | Existing snapshot-retirement and window-isolation tests passed; retirement code unchanged | Headless tests do not establish GPU completion or native multiwindow lifetime |
| Current code passes required checks | Full local gate, then release build | Passed: current full gate and subsequent release build | Server CI and native acceptance reported separately |
| Independent review and current-revision evidence | Separate reviewer using neutral exact snapshot | Three rounds completed; missing changelog addressed; rounds 2 and 3 clean | Owner accepted the result with the evidence limits below |

## Implementation steps

- [x] Resolve the owner-observed severe lag of corner zoom buttons during resize:
      place the existing standard Button pairs from the current frame's measured
      plot bounds, not deferred geometry. Use supported element lifecycle inside
      the existing domain canvas; no new widget, entity, timer or redraw loop.
      Verify rendered button bounds on the first resized frame in both
      orientations, including width/height and minimap/axis gutters, and preserve
      action, focus, disabled, tooltip and pointer behavior.
- [x] Refresh main, preserve unrelated work and create a focused branch.
- [x] Record Class A scope, roles, checkpoints and upstream-skill provenance.
- [x] Open an early Draft Pull Request linked to #226.
- [x] Update facade requirement and lockfile; inspect the selected dependency graph.
- [x] Migrate unambiguous keybinding, keystroke, modifier, scroll and theme APIs.
- [x] Resolve Root/frame and menu checkpoints with source-backed owner decisions.
- [x] Migrate approved integration and update focused Root, overlay, menu, input
      and ownership regression tests. Record the toolkit context-menu retention
      fix in the inventory; native menu lifecycle and animation were not individually
      observed and remain recorded evidence limits.
- [x] Reassess every relevant inventory exception; remove superseded paths and
      update architecture/inventory documentation alongside the code.
- [x] Complete local gate and build the current release binary after the resize correction.
- [x] Record owner-provided resize observation and overall acceptance, separating
      them from unperformed individual native scenarios.
- [x] Prepare neutral current-revision evidence for independent review.
- [x] Move the accepted plan to `docs/plans/completed/` for final workflow completion.

## Validation

- [x] Installed upstream skill trees match the exact v0.9.1 archive byte-for-byte;
      bundled relative references were checked and source-only links documented.
- [x] `cargo fmt --all -- --check`, after the resize correction
- [x] `cargo clippy --all-targets --locked`, after the resize correction
- [x] `cargo test --locked`, after the resize correction
- [x] `cargo build --release --locked`, after the current full gate passes
- [x] Focused automated Root, overlay, menu, input and multiwindow regressions.
- [x] Record native animation and context-menu lifecycle evidence limits: no
      individual runtime observations were reported for these scenarios.
- [x] Record owner acceptance of the current result. The owner reported that resize
      lag was resolved in an independently launched fresh build, then approved
      the result. Exact process/hash was not independently inspected; no separate
      claim is made for native menu/panel/focus/disabled/theme/close scenarios.
- [x] Final independent review of the resize correction and current evidence;
      all three rounds completed. Round 1's missing changelog finding was accepted
      and addressed; rounds 2 and 3 returned no substantive findings.

## Owner acceptance

The owner confirmed normal resize behavior in an independently launched fresh
build and then approved the current result. The coordinator accepted this as
completion of the agreed implementation/acceptance scope with the individual
native evidence limits above retained. No fourth review round is requested.
The branch incorporates the independent Symphonia update already accepted on main;
the only merge conflict combined the two unrelated changelog entries.
After that integration, the full formatting/Clippy/test gate passed again: 1152
tests passed, including 617 application and 176 I/O tests, with one existing
ignored manual cost measurement. The current release was then rebuilt successfully;
no migration UI code changed during the main integration or acceptance reporting.

## Post-completion

Full three-platform CI on the current head, resolved review conversations, an
up-to-date base and squash merge follow the repository workflow. Migration
completion does not authorize a release.

### Current implementation evidence

Owner feedback on the production build found severe corner-button positioning
lag during resize. This is an acceptance failure in #226; earlier validation does
not establish the corrected behavior. The measured plot rectangle is available
in canvas prepaint, while ordinary panel anchors include the minimap and axes.
Prepare standard Button elements during render, then use the toolkit's supported
layout/prepaint/paint lifecycle at that current rectangle in the existing canvas.
Keep one shared corner-zone policy for controls and deferred gesture geometry.
The third and final independent review round found no substantive findings in
this correction. After integrating main, current rustfmt, Clippy and full tests
exited successfully: 1152 tests passed, including 617 application and 176 I/O tests,
with one existing ignored manual release-mode conversion cost measurement. The
subsequent release build exited successfully. The owner confirmed normal resize
behavior in a fresh build and approved the result; the evidence limitations below
remain recorded.
The focused PlotView suite passed all 50 tests. The regression changes parent
layout dimensions and renders one frame inside the same update, before deferred
effects can converge. It compares rendered Button hitboxes to the actual painted
plot quad in both orientations and four width/height combinations. It failed on
the old placement (button y=988 versus current plot-derived y=528) and passed after
the correction. Standard Button callbacks, disabled styling and tooltip code are
unchanged; the production pairs remain enabled as before. A render/input regression
places the Paste-here marker through a zoom half, verifies the marker paints below
the control, and checks that the click emits exactly one zoom intent. Selection
badges remain below the controls; unit hints occupy the axis gutters, while saved
notices and entry overlays retain their later sibling order. The owner subsequently reported that the corrected resize behavior is normal.

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
handle Confirm first. Before the resize correction, the full application suite passed all 615 tests,
including the Root borrowing regression. The full local gate exited successfully:
rustfmt, Clippy and 1144 tests passed, with one existing ignored manual release-mode
conversion cost measurement. The subsequent release build also exited successfully.
Those earlier results predate the resize correction; the current gate and release
results are recorded above. The previous release's help/version commands exited
successfully. A bounded launch of that previous release
with a disposable I/Q capture, portable default configuration and isolated XDG
config/state/cache/data roots stayed alive for 12 seconds with no logged error,
then received SIGTERM from the harness. This establishes startup smoke only;
neither graceful close nor visible output or interaction was observed.
No executor-driven native interaction or compositor acceptance was performed.
The owner reported that resize lag is resolved in an independently launched fresh
build; its exact process/hash was not independently inspected. The owner then
approved the current result. This is overall owner acceptance, not evidence that
each menu, panel, focus, disabled-state, theme, maximize/restore/tiling, graceful-close
or Windows/macOS scenario was individually exercised. Those unreported scenarios
remain explicit evidence limits rather than claimed test passes.
