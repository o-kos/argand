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
  [control inventory](../ui/124-inventory.md) describe the old 0.6.6 integration.
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

## Rejected alternatives

- Selecting GPUI and gpui-component separately would recreate graph drift.
- Blindly retaining every 0.6 workaround or silently changing behavior to satisfy
  0.9 APIs would avoid the required component and owner decisions.
- Rewriting unrelated controls to satisfy every upstream style example would
  obscure migration regressions and exceed this Issue.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| One 0.9.1 facade and compatible graph | Manifest, lockfile and Cargo dependency inspection | Pending | Default backend; no backend substitution |
| Supported Root/overlay ownership | Exact-source review and focused regression tests | Pending | Owner decision before substantial redesign |
| Menus, nested Selects and panels preserve agreed interactions | Input/focus regression tests and native application scenarios | Pending | Native checks must be observed, not inferred from headless tests |
| Numeric transactions and disabled controls remain correct | Settings/conversion regression coverage and native checks | Pending | Any behavior change needs owner agreement |
| Custom-control inventory reflects selected graph | Each retained exception reviewed against exact locked source | Pending | Do not generalize tag evidence to a different registry version |
| Theme, window chrome and multiwindow input work | Targeted automated tests and native scenarios where available | Pending | Windows/macOS runtime acceptance is not claimed from Linux |
| Texture retirement retains originating-window ownership and intervening-frame lifetime | Current texture-retirement and multiwindow regression tests | Pending | No resource-lifetime changes without renewed evidence |
| Current code passes required checks | Full local gate, then release build | Pending | Server CI and native acceptance reported separately |
| Independent review and current-revision evidence | Separate reviewer using neutral exact snapshot | Pending | Coordinator launches reviewer after implementation |

## Implementation steps

- [x] Refresh main, preserve unrelated work and create a focused branch.
- [x] Record Class A scope, roles, checkpoints and upstream-skill provenance.
- [ ] Open an early Draft Pull Request linked to #226.
- [ ] Update facade requirement and lockfile; inspect the selected dependency graph.
- [ ] Migrate unambiguous keybinding, keystroke, modifier, scroll and theme APIs.
- [ ] Resolve Root/frame and menu checkpoints with source-backed owner decisions.
- [ ] Migrate approved integration and update focused regression tests, including
      the toolkit context-menu retention fix and executor-driven animations.
- [ ] Reassess every relevant inventory exception; remove superseded paths and
      update architecture/inventory documentation alongside the code.
- [ ] Complete local gate and build the current release binary.
- [ ] Run permitted native acceptance scenarios and record actual limitations.
- [ ] Prepare neutral current-revision evidence for independent review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Installed upstream skill trees match the exact v0.9.1 archive byte-for-byte;
      bundled relative references were checked and source-only links documented.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the full gate passes
- [ ] Focused Root, overlay, menu, input, animation and multiwindow regressions.
- [ ] Native application acceptance using the current release binary, disposable
      signal data and supported configuration/session isolation. Exercise startup,
      resize/maximize/restore, nested lists, panel transactions, keyboard focus,
      pointer/wheel isolation, numeric disabled/repeat states and theme switching.
      Record unavailable interaction capabilities and untested platforms explicitly.
- [ ] Independent review of the exact current diff and validation evidence.

## Post-completion

Ready status, full three-platform CI, owner acceptance and squash merge follow the
repository workflow. Migration completion does not authorize a release.
