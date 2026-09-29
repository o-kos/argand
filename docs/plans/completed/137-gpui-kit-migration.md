# Issue #137: Migrate the GUI stack to GPUI Kit and GPUI 0.3.x

Resolves [#137](https://github.com/o-kos/argand/issues/137).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 2.
Blocked by [#126](https://github.com/o-kos/argand/issues/126).

## Overview

Replace the separately selected GPUI 0.2.2, gpui-component 0.5.1 and
gpui-component-assets 0.5.1 dependencies with the crates.io `gpui-kit` 0.6.x
application facade and its aligned GPUI 0.3.x family. Adapt API changes without
changing product behavior or production window ownership. Migrate the #126 fixture
and prove that the supported borderless Root composition is ready for #127.

The requested target is the GPUI 0.3.x line, not an independent exact
`gpui-pre = 0.3.0` dependency. GPUI Kit selects and re-exports the compatible
GPUI release; at planning time gpui-kit 0.6.6 selects gpui-pre 0.3.6.

Implementation class: **A**. The framework graph and GUI APIs cross more than five
code files. Implementer: `gpt-5.6-sol` high; reviewer: `gpt-5.6-terra` high, fixed
for all review rounds. Do not begin implementation until #126 is accepted and merged.

## Evidence and constraints

- The application currently names GPUI or gpui-component in 16 GUI source/example
  files, with approximately 214 path references.
- A read-only temporary spike upgraded the underlying packages while preserving
  their old crate names. `cargo check -p argand` reached application code and
  reported 22 errors across startup, focus, text/image painting, shadows, layout
  and button variants. This establishes scope only; it is not a validated port.
- GPUI Kit 0.6.x re-exports GPUI, component, base and assets as one matched graph.
  Do not retain a parallel direct GPUI dependency that could create incompatible
  duplicate types.
- GPUI Kit's `Root::bordered(false)` disables its Linux CSD wrapper. #137 proves
  that composition in the fixture; #127 changes the production root and callbacks.
- Preserve the existing Argand frame, window geometry, analysis scheduling, GPU
  texture lifetime, toolkit-neutral crate boundary and application behavior.

## Implementation steps

### 1. Dependency graph and imports

- [ ] Start a focused branch from current main after #126 merges and open a Draft PR.
- [ ] Replace the three GUI dependencies with one crates.io `gpui-kit` 0.6.x entry;
      use default component/assets features unless inspection proves a smaller set.
- [ ] Regenerate and inspect `Cargo.lock`; record dependency and license changes.
- [ ] Replace direct paths with the facade: GPUI items from `gpui_kit`, styled
      controls from `gpui_kit::component`, and standard assets from
      `gpui_kit::assets`.
- [ ] Initialize with `gpui_kit::application()` and `gpui_kit::init(cx)` while
      preserving Argand's composed asset source and startup ordering.

### 2. GPUI 0.3.x API adaptation

- [ ] Adapt application/task update return types and window lifecycle APIs without
      changing error handling or silently swallowing window-open failures.
- [ ] Pass `App` contexts to focus and traversal APIs and preserve current focus
      targets, editor precedence and one-shot action dispatch.
- [ ] Adapt shaped-text and image painting signatures without changing measured
      axis/cursor geometry, texture sampling or retirement behavior.
- [ ] Adapt shadows, flex layout and component variants using supported semantic
      theme APIs; do not introduce lint suppressions or custom control substitutes.
- [ ] Compile every target, including the compatibility example, before interpreting
      runtime behavior.

### 3. Compatibility fixture

- [ ] Port the #126 fixture and retain the locked-stack observations as its baseline.
- [ ] Add a borderless Root case around representative Argand-framed content and
      prove there is no added border, shadow, inset or resize hit region.
- [ ] Re-run NumberInput, Select, Popover, Dialog, menus, tooltips, Tab/Shift+Tab,
      clipboard, undo/redo, nested Escape and focus-restoration cases. Record whether
      the old Select-in-Popover deferred-draw panic remains.
- [ ] Re-run the frame interaction matrix with the Argand frame. The rejected stock
      frame is not a migration target and need not be made production-ready.

### 4. Documentation and validation

- [ ] Update `AGENTS.md`, `CONTRIBUTING.md`, architecture text and fixture evidence
      from the old package names only when the corresponding migration has landed.
- [ ] Compare cold startup, release binary size and representative plot interaction
      with the #126 baseline; investigate material regressions.
- [ ] Run the full local gate and rebuild the release binary.
- [ ] Complete native Linux Wayland/X11 fixture checks and current three-platform
      `ci/full`; compilation does not substitute for native input/frame evidence.
- [ ] Complete the required external class-A review after implementation. This
      planning update does not run an automatic review.
- [ ] Move this plan to `docs/plans/completed/` only after every required check and
      evidence row is complete.

## Acceptance boundary

#137 ends with the migrated dependency/API graph and a positive borderless-Root
fixture result. The production main window remains rooted directly at `Shell`.
#127 then introduces Root-to-Shell ownership and `WindowHandle<Root>` while retaining
`chrome.rs` as the sole frame. PlotView extraction, overlay-policy consolidation,
ordinary-control replacement and product UI changes remain later issues.

If GPUI Kit 0.6.x cannot preserve current behavior or the borderless composition
does not pass the fixture, stop with reproducible evidence and return to the owner.
Do not pin an independently chosen GPUI snapshot, patch the registry, fork the
toolkit or change product behavior as an implicit workaround.
