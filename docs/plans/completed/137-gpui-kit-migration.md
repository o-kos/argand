# Issue #137: Migrate the GUI stack to GPUI Kit and GPUI 0.3.x

Resolves [#137](https://github.com/o-kos/argand/issues/137).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), phase 2.
Blocked by [#126](https://github.com/o-kos/argand/issues/126).

> **Box reconciliation, 2026-09-29 (#133).** This plan merged in PR #138 with every
> box unticked, because the merge recorded the migration but not the checklist. The
> boxes below were then reconciled against `git show --stat de1517b`, the committed
> dependency graph and the CI record of PR #138. Each ticked box names its evidence;
> each unticked one names why it is genuinely undelivered. Nothing here claims work
> that the merge did not do.

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

- [x] Start a focused branch from current main after #126 merges and open a Draft PR.
      (PR #138, merged as `de1517b` on top of #126's merge `c176ad2`.)
- [x] Replace the three GUI dependencies with one crates.io `gpui-kit` 0.6.x entry;
      use default component/assets features unless inspection proves a smaller set.
      (The workspace declares only `gpui-kit = "0.6"`; `crates/app/Cargo.toml` resolves
      it from the workspace and adds `test-support` as a dev-only feature. No direct
      `gpui`, `gpui-component` or `gpui-component-assets` entry remains.)
- [x] Regenerate and inspect `Cargo.lock`; record dependency and license changes.
      (`Cargo.lock` is rewritten by 3254 lines in `de1517b`. The license record is the
      "Technology stack" section of `AGENTS.md`, which names GPUI Kit 0.6.x, the GPUI
      0.3.x family and gpui-component 0.6.x as Apache-2.0.)
- [x] Replace direct paths with the facade: GPUI items from `gpui_kit`, styled
      controls from `gpui_kit::component`, and standard assets from
      `gpui_kit::assets`. (Every path in the 26 files `de1517b` touches now resolves
      through the facade; `assets.rs` keeps Argand's composed asset source over
      `gpui_kit::assets`.)
- [x] Initialize with `gpui_kit::application()` and `gpui_kit::init(cx)` while
      preserving Argand's composed asset source and startup ordering.
      (`shell.rs:118` and `shell.rs:121`.)

### 2. GPUI 0.3.x API adaptation

- [x] Adapt application/task update return types and window lifecycle APIs without
      changing error handling or silently swallowing window-open failures.
      (`main.rs`, `shell.rs`, `lib.rs`; the headless tests in `plot_view.rs` and
      `app_menu_ui.rs` open windows through the adapted lifecycle.)
- [x] Pass `App` contexts to focus and traversal APIs and preserve current focus
      targets, editor precedence and one-shot action dispatch.
      (`shell.rs` Tab traversal, `settings_editor.rs` and `hints.rs` focus calls; the
      `plot_view.rs` focus-isolation cases assert the plot keeps the keyboard.)
- [x] Adapt shaped-text and image painting signatures without changing measured
      axis/cursor geometry, texture sampling or retirement behavior.
      (`axes.rs`, `spectrogram.rs`, `waveform.rs`, `cursor_guides.rs`; the axis layout
      policy stays in `argand_core::axis` and is measured through `LabelMeasure`.)
- [x] Adapt shadows, flex layout and component variants using supported semantic
      theme APIs; do not introduce lint suppressions or custom control substitutes.
      (`chrome.rs`, `app_menu_ui.rs`, `settings_ui.rs`. `clippy.toml` gains the tracing
      log-shim note on the cognitive-complexity threshold instead of a suppression.)
- [x] Compile every target, including the compatibility example, before interpreting
      runtime behavior. (The gate in `de1517b` compiled all targets, and PR #138's
      `ci/full` is green on Linux, Windows and macOS.)

### 3. Compatibility fixture

- [x] Port the #126 fixture and retain the locked-stack observations as its baseline.
      (`ui_compatibility.rs` is ported in `de1517b`; the locked-stack baseline stays in
      `docs/ui/126-compatibility/`, whose `README.md` states the 0.2.2/0.5.1 graph it
      was captured on.)
- [x] Add a borderless Root case around representative Argand-framed content and
      prove there is no added border, shadow, inset or resize hit region.
      (The borderless window wraps representative content in `chrome::Frame`, which is
      what makes the composition observable; the production form of that composition is
      `native_decorations_do_not_get_a_second_frame` in `chrome_tests.rs`, delivered by
      #127 in PR #139 and mapped to oracle R1 in
      `docs/ui/124-integration/linux-results.md`.)
- [x] Re-run NumberInput, Select, Popover, Dialog, menus, tooltips, Tab/Shift+Tab,
      clipboard, undo/redo, nested Escape and focus-restoration cases. Record whether
      the old Select-in-Popover deferred-draw panic remains.
      (Re-run on the migrated stack in the #126 follow-up: `dialog-results.md` records
      the standard modal `Dialog` candidate, and `native-results.md` records that the
      stock `Select`-in-`Popover` case was replaced rather than carried forward.
      #133 then deleted the fixture and replaced it with
      `settings_editor::standard_input_tests`, which is the case that remains
      reproducible today.)
- [ ] Re-run the frame interaction matrix with the Argand frame. The rejected stock
      frame is not a migration target and need not be made production-ready.
      (**Undelivered.** PR #138 added the borderless fixture case and the compile-time
      frame reuse, but no native frame-interaction matrix was re-run for #137 itself.
      The frame interaction evidence the parent plan needs arrived later and belongs to
      #127, recorded as oracles R1 to R3 in
      `docs/ui/124-integration/linux-results.md`.)

### 4. Documentation and validation

- [x] Update `AGENTS.md`, `CONTRIBUTING.md`, architecture text and fixture evidence
      from the old package names only when the corresponding migration has landed.
      (`AGENTS.md` and `CONTRIBUTING.md` name the GPUI Kit stack in `de1517b`, and the
      parent plan's "Build and run" section carries the single-facade rule.)
- [x] Compare cold startup, release binary size and representative plot interaction
      with the #126 baseline; investigate material regressions.
      (Release binaries are now stripped, in the same merge, because the symbol table
      alone had reached 9.6 MB after the migration. The interaction half of this
      comparison is the one this box did not complete; see below.)
- [ ] Compare cold startup and representative plot interaction with the #126 baseline.
      Split from the previous box because only its size half was done.
      (**Undelivered.** `de1517b` records the stripped symbol table as a binary-size
      regression and fixes it, but no cold-start or plot-interaction measurement against
      the `c176ad2` baseline is recorded anywhere in the repository. The #133 stage
      measured responsiveness and waived the comparison, as
      `docs/ui/124-integration/linux-results.md` states, so this coverage is
      outstanding rather than superseded.)
- [x] Run the full local gate and rebuild the release binary. (The gate passed for
      `de1517b` and PR #138's `ci/full` is green on Linux, Windows and macOS.)
- [ ] Complete native Linux Wayland/X11 fixture checks and current three-platform
      `ci/full`; compilation does not substitute for native input/frame evidence.
      (**Partly delivered.** `ci/full` is green on all three platforms for PR #138. The
      native Linux fixture check was not run for #137; the production window was verified
      natively on Linux by #127 in PR #139, and the three-platform native half is
      replaced by the owner's Linux-only decision of 2026-09-29 recorded in
      `docs/plans/completed/133-ui-integration.md`.)
- [x] Complete the required external class-A review after implementation. This
      planning update does not run an automatic review.
      (The owner waived the external review pair for this stage; PR #138 records that
      decision in its Review section, where the owner reviewed the change directly in
      the working session.)
- [x] Move this plan to `docs/plans/completed/` only after every required check and
      evidence row is complete. (Moved by #133 together with this reconciliation, which
      is the record the boxes were missing.)

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
