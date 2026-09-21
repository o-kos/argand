# Issue #126: Standard UI compatibility checkpoint

Resolves [#126](https://github.com/o-kos/argand/issues/126).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 1.

## Overview

Deliver a bounded runnable verification fixture, a source-grounded inventory and
an honest native-evidence report before the production Root/frame migration.
The fixture is a Cargo example, not another application settings workflow.
Production UI, dependencies and user configuration remain unchanged.

Class **A**, declared before implementation. Implementer: `gpt-5.6-sol` high;
reviewer: `gpt-5.6-terra` high. Keep this pair for all implementation review rounds.

## Context

The branch starts from the locally accepted planning/policy revision for PR #125;
that PR is not being merged by this task. If still unmerged at publication, stack
the Draft against its planning branch and explicitly rebase/retarget to main after
the planning PR is accepted. A failed remote refresh must not be described as
proof that main is current.

Inspected locked APIs (GPUI 0.2.2 / gpui-component 0.5.1):

- Root is required at the window top level by standard inputs and unconditionally
  wraps `window_border()`. Its frame uses `window_bounds()` for resize geometry;
  compare this with the production viewport-based fixes under expanded/tiling cases.
- Popover retains its state through element identity; content should reuse retained
  InputState/SelectState entities. It supplies focus, cancellation and occlusion.
- PopupMenu dismisses its parent chain on Cancel; preserve this stock behavior in
  the probe so the one-level Escape mismatch is observable rather than hidden.
- ResizablePanelGroup exposes retained ResizableState and `on_resize`. In the
  inspected version the callback runs on mouse-up; sizes update during movement.
  Probe live size observation separately before recommending it for coalesced
  spectrum resizing. Do not infer live callbacks from the API name.
- Use standard Button, NumberInput, Select, Popover, PopupMenu, Tooltip, TitleBar
  and resizable components. Only the domain-like plot canvas and passive event
  counters are custom: their purpose is to observe the toolkit, not replace it.

## Decisions

- Add `crates/app/examples/ui_compatibility.rs` with small example-local modules
  if needed. It runs using the existing locked dependencies and toolkit assets.
- A standalone Root-backed window contains the stock TitleBar, a simple plot-like
  canvas with visible input counters/readout, and standard interactive controls.
- Retain editable state across renders. Probe NumberInput and Select inside a
  standard Popover, nested stock menus, passive hints, and standard zoom Buttons.
- Expose horizontal/vertical resizable arrangements and current panel dimensions
  alongside resize-callback counts, so live state vs release-only callbacks can be
  distinguished. A fixture counter is evidence of the callback it names, not proof
  of application DSP scheduling.
- Provide controls for light/dark theme and a predictable reset of probe counters.
  Use standard controls for these too. Do not add global navigation interceptors.
- Record actual window state/viewport versus restore dimensions in the fixture
  or its logs to make frame failures reproducible without modifying the toolkit.
- No production file loading or session persistence in the probe. No implicit
  writes to user configuration and no dependency, registry-cache or lint changes.
- Native runs are explicit. Do not control existing user windows or stop services.
  An isolated GPU-backed compositor is useful for scoped tests but is not evidence
  for unexercised desktop/compositor or Windows/macOS behavior.

## Implementation steps

- [x] Inspect required toolkit interfaces and commit this scoped plan before coding.
- [ ] Implement and compile the standalone standard-control verification example.
- [ ] Add deterministic tests for example-owned counter/state transitions where
      meaningful; tests must not merely restate toolkit behavior without a window.
- [ ] Complete `docs/ui/126-compatibility/inventory.md`, mapping every production
      render/input entry point to standard, custom-domain, custom-control or adapter
      ownership and recording supported candidates/verified limitations.
- [ ] Write `docs/ui/126-compatibility/README.md` with runnable commands, test cases,
      source evidence, native results and a go/no-go decision for #127.
- [ ] Exercise the current application baseline and fixture on available native
      environments without confusing historical screenshots with current evidence.
- [ ] Obtain missing native coverage before marking the compatibility gate complete;
      list unavailable cases as not exercised. Unresolved frame failures block #127.
- [ ] Run local gate, rebuild current releases/examples, inspect the diff and complete
      external review before presenting the implementation for owner acceptance.
- [ ] Publish/update a Draft PR linked to #126. Do not close #124 or start #127 here.
- [ ] Move this plan to completed only once all required compatibility evidence and
      decisions exist; a working fixture with incomplete evidence stays an open Draft.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo test -p argand --example ui_compatibility --locked`
- [ ] `cargo build --release --locked`, after checks pass
- [ ] `cargo build -p argand --example ui_compatibility --release --locked`
- [ ] Native R1/R2/R3: Root identity, single frame, move/resize edges/corners,
      maximize/restore/fullscreen/tiling and title-bar gesture isolation.
- [ ] Native F2/F3: cursor/selection, clipboard, undo, IME, numeric validation,
      Tab traversal, nested Select/menu Escape and focus restoration.
- [ ] Native P1/P2: popover/control event isolation; passive hint cursor/Alt-guide
      behavior and independent click/wheel compatibility observations.
- [ ] Native standard Button visual states and resizable horizontal/vertical minimum
      sizes, live dimensions versus callbacks, mouse-up outside and focus loss.
- [ ] Report rows use case, build, OS/backend/compositor, scale/theme/orientation,
      preconditions, input, expected/observed result, pass/fail/not-exercised and
      evidence. No skipped or inferred native check is marked passed.
- [ ] Full CI on an up-to-date final revision and all required native evidence before
      merge; Draft quick CI alone is not the compatibility gate.

## Post-completion

Link evidence from #124 and #126. Proceed to #127 only after an accepted positive
compatibility result; seek an explicit decision if the stock frame cannot meet the
functional contract. The other seven implementation stages remain separate work.
