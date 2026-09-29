# Issue #133: UI architecture integration and handoff

Resolves [#133](https://github.com/o-kos/argand/issues/133).
Parent: [#124](https://github.com/o-kos/argand/issues/124),
[approved architecture](124-standard-ui-architecture.md), sections 5 (inventory
closure) and 6 (integration and handoff). Predecessor
[#132](completed/132-fixed-minimap-height.md), merged in PR #162.

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
  (`completed/126-ui-compatibility.md`, `137-gpui-kit-migration.md`).
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
- **Linux native matrix (owner).** Claude prepares
  `docs/ui/124-integration/linux-results.md` with one row per parent oracle and
  parameter (R1 to S1, both orientations, dark and light, start and loaded states,
  progressive updates, file replacement, settings preview and cancel, persistence)
  in the format `case | build | environment | input | expected | observed | result`.
  The owner runs it on the release build of this branch; Claude records the results.
- **Performance comparison (owner, Linux).** Claude builds the baseline `c176ad2`
  and the branch in release. The owner compares window resize, zoom and pan
  responsiveness on the same capture, and Claude reads
  `argand::ui_latency=trace` for both. A regression is investigated before merge.
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
- [ ] Update AGENTS.md "Current status", README and CHANGELOG as decided.
- [ ] Full local gate.

Claude and owner:

- [ ] Prepare `docs/ui/124-integration/linux-results.md`; owner runs it; record
      results.
- [ ] Baseline and branch release builds; performance comparison recorded.
- [ ] Review rounds with Codex `gpt-6-sol` high.
- [ ] Reconcile #108; remove the Kilo worktree.
- [ ] Move this plan and the parent plan to `docs/plans/completed/` before final
      review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Linux native matrix recorded, every row pass or an accepted, tracked issue.
- [ ] Performance comparison recorded with no uninvestigated regression.
- [ ] `ci/full` on Linux, Windows and macOS for the final revision.

## Post-completion

- Close #124 with the evidence map.
- #108 can start on the standard infrastructure.
