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
- Record actual window state/viewport and the reported WindowBounds variant/bounds
  to make frame failures reproducible without modifying the toolkit. Reported
  bounds are not guaranteed restore geometry on every backend; record actual
  before/after sizes separately when testing maximize/restore transitions.
- No production file loading or session persistence in the probe. No implicit
  writes to user configuration and no dependency, registry-cache or lint changes.
- Review corrections make observation explicit: label toolkit bounds as reported,
  not guaranteed restore geometry; provide focused plot key counts and a passive
  hint target/Alt-guide probe. NumberInput uses a small fixture-only bounded value
  adapter for Step events and invalid feedback, not product FFT policy.
- Native runs are explicit. Do not control existing user windows or stop services.
  An isolated GPU-backed compositor is useful for scoped tests but is not evidence
  for unexercised desktop/compositor or Windows/macOS behavior.

## Implementation steps

- [x] Inspect required toolkit interfaces and commit this scoped plan before coding.
- [x] Implement and compile the standalone standard-control verification example.
- [x] Add deterministic tests for example-owned counter/state transitions where
      meaningful; tests must not merely restate toolkit behavior without a window.
- [x] Complete `docs/ui/126-compatibility/inventory.md`, mapping every production
      render/input entry point to standard, custom-domain, custom-control or adapter
      ownership and recording supported candidates/verified limitations.
- [x] Write `docs/ui/126-compatibility/README.md` with runnable commands, test cases,
      source evidence, native results and a go/no-go decision for the locked stack.
- [x] Exercise the current application baseline and fixture on available native
      environments without confusing historical screenshots with current evidence.
- [x] Owner decision, 2026-09-22: terminate the gate on the recorded NO-GO verdict
      instead of investing further measurement in the locked 0.2.2 stack; all further
      compatibility hypotheses are made on GPUI 0.3.x. Cases never exercised are
      listed as such in `native-results.md`, and #137 re-owns fixture evidence on
      the migrated stack.
- [x] Run local gate, rebuild current releases/examples, inspect the diff and complete
      external review before presenting the implementation for owner acceptance.
- [x] Publish/update a Draft PR linked to #126. Do not close #124 or implement #137 here.
- [x] Move this plan to completed with the gate-termination decision recorded; a
      working fixture with incomplete evidence stayed an open Draft only until the
      owner resolved the gate.

## Validation

### Follow-up: supported overlay composition

The initial partial checkpoint is published and its independent review is clean;
it did not pass the compatibility gate. Continue the same Class A scope and model
pair with the following bounded experiment before any production migration:

- [x] Keep the stock Popover/Select failure reproducible, behind an explicitly
      labelled, opt-in crash probe; ordinary fixture use must not open it accidentally.
- [x] Add a separate standard Root-backed Dialog candidate with retained NumberInput
      and Select entities and exactly one Root dialog layer. Use public APIs only.
      Observe live validation and actual focus; do not force focus back to hide a
      toolkit failure. A modal Dialog is a different UX, not an approved replacement
      for an anchored editor or a change to #108 acceptance.
- [x] Move window metrics out of the crowded toolbar and keep fixture controls
      usable at the declared minimum size. This is fixture layout, not frame design.
- [x] Exercise dropdown selection, nested Escape, retained values, focus return,
      plot key isolation and pointer/wheel isolation in the isolated GPU-backed
      Linux environment; report each result separately and retain prior evidence.
- [x] Repeat local gates, release builds and independent review for this continuation;
      update the Draft report without claiming full cross-platform compatibility.

Source basis: locked Popover and Select both call `deferred`, which GPUI rejects
when nested. `Root::render_dialog_layer` composes Dialog without an outer deferred
draw; `WindowExt::open_dialog`/`close_dialog` save and restore prior focus. These
source observations are hypotheses for native validation, not passing test cases.

The [follow-up native ledger](../ui/126-compatibility/dialog-results.md) records
the bounded Linux sequence: the standard modal composition works in those cases,
while the explicitly enabled Popover reproducer still panics. Do not infer UX
equivalence, complete input coverage or cross-platform compatibility from this.

**Owner gate:** the stock fixture frame appearance is explicitly not approved.
Before production Root migration, demonstrate that resize cursors match the visible
edges and reset on entry,
right and diagonal resize targets are practical, bare-title double-click toggles
once and reliably, and dark-theme minimize/maximize controls have distinct
default/hover/pressed states.
Record geometry, pointer coordinates and screenshots; the owner must approve the
appearance independently of technical compatibility results. Do not redesign the
production frame in #126.

Locked-source inspection after the owner report found no application-level repair
through the public standard component. `TitleBar` keeps `ControlIcon` private,
binds every Linux click on maximize/restore directly to `zoom_window()`, and fixes
the non-close visual states to global secondary theme tokens. `Root` also owns the
private `window_border`, whose resize calculation uses reported `window_bounds()`.
Current upstream retains the same title-bar implementation, and its open #2496
reports stale caption-button hover on GPUI 0.2.2 / gpui-component 0.5.2. The resize
and bare-title failures are regressions of the standard Root fixture, not defects
of the current production frame. Treat the standard candidate as NO-GO unless a
supported upstream API or dependency version demonstrably meets the gate; do not
copy the private controls or patch the registry here. Test a deliberate global
secondary hover/active palette adjustment before classifying caption contrast as
a component blocker; it affects ordinary controls throughout the window.

Owner decision: maximize/restore responding to both clicks of a double-click is an
accepted, non-blocking behavior. Caption contrast is a shared-theme task, not a
reason to retain or replace a window-control implementation by itself.

### Initial checkpoint results

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo test -p argand --example ui_compatibility --locked`
- [x] `cargo build --release --locked`, after checks pass
- [x] `cargo build -p argand --example ui_compatibility --release --locked`
- [x] Native R1/R2/R3, F2/F3, P1/P2 and resizable/Button rows: terminated
      unexercised on the locked stack by the 2026-09-22 owner decision. The
      exercised subsets and the explicit not-exercised list live in
      `docs/ui/126-compatibility/native-results.md`; no unexercised case is
      recorded as passed anywhere.
- [x] Report rows use case, build, OS/backend/compositor, scale/theme/orientation,
      preconditions, input, expected/observed result, pass/fail/not-exercised and
      evidence. No skipped or inferred native check is marked passed.
- [x] Full CI on the up-to-date final revision before merge; Draft quick CI alone
      is not the compatibility gate.

## Post-completion

Final state: the compatibility gate is **NO-GO** and closed by the 2026-09-22
owner decision. Opening the stock Select inside Popover reproducibly panics in
the locked GPUI deferred drawing path, and the stock frame regresses current
resize and bare-title behavior; both reasons stand unpatched and unresolved by
design. Remaining platform/input checks were terminated unexercised rather than
completed against a stack this decision abandons. Do not patch dependencies or
substitute custom controls retroactively; the record above is the checkpoint.

Link evidence from #124 and #126. Proceed through #137 to the aligned GPUI Kit /
GPUI 0.3.x stack, then use #127 for borderless production Root ownership while
retaining the Argand frame. The other implementation stages remain separate work.
