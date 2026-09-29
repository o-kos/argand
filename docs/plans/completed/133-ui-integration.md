# Issue #133: UI architecture integration and handoff

Resolves [#133](https://github.com/o-kos/argand/issues/133).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), sections 5 (inventory
closure) and 6 (integration and handoff). Predecessor
[#132](132-fixed-minimap-height.md), merged in PR #162.

## Overview

#127 to #132 delivered the #124 architecture piece by piece. This stage proves the
pieces work together, closes the control inventory, removes the #126 prototype,
brings the documentation from "planned" to "implemented", hands #108 its updated
constraints and closes #124. It adds no product feature.

Boundaries: no new UI behaviour, no dependency change, no #108 editor, no #123 hint
contrast, no packaging. A regression found here is fixed here only when it is small
and local; anything larger becomes its own Issue before #124 closes.

Implementation class: **A**, declared before implementation: the stage spans the
whole #124 surface, deletes more than 1500 lines and gates the parent's closure.

Roles, set by the owner on 2026-09-29: implementer **Kilo Code** with
`openrouter/stealth/space-bunny-alpha`, variant `xhigh`; reviewer **Codex**
`gpt-6-sol`, reasoning effort `high`. Claude writes this plan, arbitrates, runs the
gate, reconciles #108 and #124 on GitHub and talks to the owner.

**Platform scope (owner, 2026-09-29).** Native interaction is verified on Linux
only. Windows and macOS are covered by `ci/full` builds and tests, and their native
behaviour is not verified in this stage; a problem found there later becomes its own
Issue. This replaces the parent plan's three-platform native matrix, and #124 closes
without it.

## Context

- The parent inventory (`124-standard-ui-architecture.md`, "Custom-control inventory
  and target disposition") has final rows for the zoom halves and toolbar (#130), the
  application menu (#131) and the splitter (#132). The frame, title-bar gestures,
  ready-input capture, symbolic-key interceptor, domain drawing, settings form, ruler
  context menu, hints and keycaps rows still read as targets, not outcomes.
- Parent phases 1 and 2 are unticked although #126 and #137 delivered them
  (`completed/126-ui-compatibility.md`, `completed/137-gpui-kit-migration.md`).
  Both are reconciled and ticked in the "Review round 1" section below.
- `crates/app/examples/ui_compatibility.rs` and `ui_compatibility/model.rs` (about
  1600 lines) are the #126/#137 probe. They are not shipped, but the Issue asks to
  remove temporary prototype UI while keeping a reproducible check of standard
  in-window `NumberInput` and `Select`.
- `docs/ui/126-compatibility/` holds the #126 evidence and stays as history.
- AGENTS.md "Current status" still opens with "Planned UI architecture (#124)".
- #108 (interactive FFT hint) carries a "no Root" constraint that #124 made obsolete.
  #122 is closed. #123 stays open and separate.
- Baseline for the performance comparison: `c176ad2`, the last `main` commit before
  the GPUI Kit migration (#138).

## Decisions

- **Inventory audit.** List every `impl Render`, `on_mouse*`, `on_scroll*`,
  `on_key*`, `on_action`, `on_drag`, `intercept_keystrokes`, `observe_keystrokes` and
  window-level `on_mouse_event` in `crates/app/src`, map each to an inventory row, and
  give every row a final disposition: **Replaced** with the standard component and
  the Issue, **Removed**, or **Retained** with its justification (the standard
  alternative, the verified limitation, why composition is insufficient, and the
  tests that cover it). Rows already final stay as they are. The audit list goes into
  `docs/ui/124-inventory.md`; the parent table keeps one line per control.
- **Prototype removal.** Delete `crates/app/examples/ui_compatibility.rs`,
  `ui_compatibility/model.rs` and any Cargo, CI or documentation reference to the
  example. Keep one headless GPUI test (`gpui-kit` `test-support`, dev-only) that
  opens a window with a standard `Root`, a `NumberInput` and a `Select`, types into
  the input, opens and chooses in the select, and closes the list with Escape
  without closing the window, so standard in-window input stays reproducible. If the
  settings editor's existing tests already prove exactly that, point to them instead
  of adding a duplicate.
- **Parent plan reconciliation.** Tick parent phases 1 and 2 with links to the #126
  and #137 evidence, rewrite the parent's native-matrix items to the Linux-only scope
  above, and link each parent acceptance item to its evidence (child PR, test or
  native record).
- **Documentation.** AGENTS.md "Current status" describes the implemented ownership
  (Root, frame, PlotView, overlays, standard controls, removed splitter) instead of the
  plan; drop sentences that only described migration steps. README and CHANGELOG are
  checked for claims #124 made obsolete; CHANGELOG gains nothing unless a shipped,
  user-visible change is missing from it.
- **Native evidence map instead of a rerun (owner, 2026-09-29).** This stage changes
  no shipped behaviour, so the owner does not repeat the native matrix.
  `docs/ui/124-integration/linux-results.md` maps every parent oracle (R1 to S1) and
  the toolbar, menu and minimap checks to the stage and PR where the owner verified
  them natively on Linux. IME, clipboard and undo as editing behaviour, and the
  responsiveness comparison against the baseline `c176ad2`, were never exercised; the
  owner waived them, and the map records the waiver.
- **Handoff.** Claude edits #108 to replace its no-Root constraint with the
  standard-infrastructure contract (PinnedHint, `Root`, standard inputs), without
  implementing it, and closes #124 after this PR merges with a comment mapping its
  acceptance criteria to evidence.
- **Cleanup outside the repository.** The ignored Kilo worktree from #130
  (`.kilo/worktrees/breezy-reptile`) is removed with `git worktree remove`.

## Rejected alternatives

- Keep the compatibility example as a fixture: it is a second UI outside the product
  that nobody runs, which is what the Issue asks to remove.
- Three-platform native matrix: the owner has Linux only and waived the rest.

## Implementation steps

Kilo:

- [x] Inventory audit: `docs/ui/124-inventory.md` and a final disposition in every
      parent inventory row.
- [x] Remove the compatibility example and its references; keep or add the headless
      in-window `NumberInput` / `Select` test.
- [x] Reconcile the parent plan: phases 1 and 2, Linux-only native scope, evidence
      links for each acceptance item.
- [x] Update AGENTS.md "Current status", README and CHANGELOG as decided.
- [x] Full local gate.

Claude and owner:

- [x] Map the native evidence in `docs/ui/124-integration/linux-results.md`
      (owner waived a rerun and the performance comparison).
- [ ] Review rounds with Codex `gpt-6-sol` high.
- [ ] Reconcile #108; remove the Kilo worktree.
- [x] Move this plan and the parent plan to `docs/plans/completed/` before final
      review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [x] Native evidence mapped to the stages that verified it; the waived cases are
      recorded as not exercised.
- [ ] `ci/full` on Linux, Windows and macOS for the final revision.

## Review round 2

Three accepted items, all in documents the planner owns. The evidence map claimed F3
and S1 whole where the stages recorded only parts: F3 now separates the natively
verified menus from the select list and settings editor, which only the headless
test proves, and S1 separates the verified preview, cancel, persistence and file
replacement from the label agreement and range reset, which were never recorded
natively; both uncovered parts are listed as waived. The parent plan's ticked native
items now name only the documented actions and list the waived ones. The approved
architecture links in the completed #126 and #128 plans are fixed by moving this plan
and the parent plan to `docs/plans/completed/`, with every link to either plan
updated and no broken relative link left in the documentation.

## Post-completion

- Close #124 with the evidence map.
- #108 can start on the standard infrastructure.

## Review round 1

External review, Codex `gpt-6-sol` high. Four findings: one major, three minor. Three
were accepted and fixed on this branch; the fourth was addressed by the planner before
this section was written.

### Finding 1 (rejected, planner's evidence map)

`docs/ui/124-integration/linux-results.md` was a rerun log that the planner had already
reworked into a native evidence map. Not reopened. The owner's Linux-only decision of
2026-09-29 stands, and the file is left exactly as the planner wrote it.

### Finding 2 (accepted, major): parent phases 1 and 2 were unticked

The final report claimed phases 1 and 2 were ticked while
`docs/plans/124-standard-ui-architecture.md` still had every box open. All eight boxes
are now ticked, each with its evidence.

Phase 1, delivered by #126 in PR #134
([completed/126-ui-compatibility.md](126-ui-compatibility.md),
[ui/126-compatibility/](../../ui/126-compatibility/)):

- The native baseline and the finished inventory point at that directory's
  `native-results.md` (build hashes, Wayland/Sway run, display scale) and `inventory.md`.
- The bounded fixture records the file #133 later deleted, and names
  `settings_editor::standard_input_tests` as what replaced it.
- The event matrix points at the reproduced `Select`-inside-`Popover` deferred-draw
  panic and at the `occlude()` wheel observation, which is the passive-hint distinction.
- The limitations and the go/no-go record the owner's 2026-09-22 gate-termination
  decision, so the "do not migrate on a frame failure" condition was resolved, not
  skipped.

Phase 2, delivered by #137 in PR #138
([completed/137-gpui-kit-migration.md](137-gpui-kit-migration.md)):

- The single facade entry and the committed graph, read from `Cargo.toml` and
  `crates/app/Cargo.toml`.
- Startup and asset adaptation, read from `shell.rs:118` and `shell.rs:121`, with the
  production root still `Shell` until #127.
- The borderless case, which #127 then adopted natively and #133 replaced with a
  headless test.

Phase 2's last box asks for the local gate, the release build, the license inventory,
the performance comparison and native/three-platform validation. The plan now states
exactly that: the gate, the release build and the license inventory are recorded in the
#137 plan; the production borderless-`Root` window was verified natively on Linux by
#127 in PR #139 (`native_decorations_do_not_get_a_second_frame` in `chrome_tests.rs`,
oracles R1 to R3 in the evidence map); the three-platform native half is replaced by the
owner's Linux-only decision, with Windows and macOS covered by `ci/full` builds and
tests. The cold-start and plot-interaction comparison against the `c176ad2` baseline is
not claimed: the #137 plan records it as undelivered.

### Finding 3 (accepted, minor): the one-level Escape test could not fail

`settings_editor::standard_input_tests` asserted that Escape closed the select list and
not the window, but its form had no outer Escape handler, so an Escape that leaked
outward would have passed unnoticed. The test form now carries the real editor's
`AnalysisEditor` key context and the real `CloseSettings` action, so `init` binds escape
to it, and it counts the escapes that arrive. The test asserts the first Escape leaves
the count at zero while the list closes and the keyboard returns to the trigger, and
that a second Escape raises it to one.

The `Select` behaviour this proves is recorded in the inventory as a locked-toolkit
observation: `Select::escape` stops at its own open list and propagates when it is shut
(`src/select.rs:404` to `:415`).

### Finding 4 (accepted, minor): the input-entry-point audit was incomplete

The inventory claimed a complete list of explicit input entry points but missed the
`.tooltip` registrations, because the search results were carried across by hand. The
audit was re-run against the source and the document now carries the exact `rg` command
and the `awk` filter that drop `#[cfg(test)]` scaffolding, so the list is reproducible
rather than asserted.

Six production registrations were missing and are now rows I35 to I38: the application
button (`app_menu_ui.rs:383`), the orientation segment and the grid toggle
(`app_menu_ui.rs:496`, `app_menu_ui.rs:528`), a zoom half (`plot_ui.rs:568`), and a
start-page recent row and the chooser (`shell.rs:1298`, `shell.rs:1328`). They are
passive hints through `hints::passive` or `shortcut_tooltip`, and the R9 disposition now
names them. The command returns 101 entry points at this revision, and the one
production-shaped hit that is absent, `hints.rs:471`, is inside `#[cfg(test)] mod tests`
and is documented as such.

While re-running the audit, the frame's own render method was found not to be an
`impl Render` block, so the document gives the separate `rg -n 'pub fn render'` that
finds `chrome.rs:135`.

### Pre-existing, not fixed here

`completed/126-ui-compatibility.md` and `completed/128-plot-view.md` each carry a link
to a sibling that does not resolve from their directory. Both predate this stage and
neither concerns this reconciliation, so they are left for their own issue.
