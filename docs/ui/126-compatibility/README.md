# Standard UI compatibility checkpoint (#126)

Parent: [architecture plan](../../plans/124-standard-ui-architecture.md).
Child: [implementation plan](../../plans/126-ui-compatibility.md).
Source survey: [control inventory](inventory.md).

## Status

The production UI is unchanged. The runnable fixture and inventory exist; the
checkpoint remains incomplete. Build results and native evidence are separate.

**Decision: NO-GO for direct production Root/frame adoption on the locked graph.**
Opening the stock Select inside the stock Popover reproducibly panics with `cannot call defer_draw during
deferred drawing` on the locked GPUI 0.2.2 / gpui-component 0.5.1 graph. See the
[native reproduction](native-results.md). The stock frame also fails the owner's
acceptance gate: its resize targeting/cursor lifetime and bare-title gestures
regress behavior that works in the current production frame. The owner accepts
maximize/restore responding to both clicks of a double-click. Dark caption contrast
uses global secondary theme tokens and will be tested as a palette correction
rather than treated as a per-control API blocker.
The owner subsequently authorized #137 to migrate through GPUI Kit 0.6.x to its
aligned GPUI 0.3.x family. This report remains the before-migration oracle; it does
not authorize a toolkit patch, custom control or product-behavior change.

The owner has explicitly not approved the current frame appearance. The exact gate
is recorded in the Issue and plan; unresolved items keep this checkpoint at NO-GO.
The #126 fixture is not the approved production frame design.

The [Dialog follow-up](dialog-results.md) demonstrates a working **modal** standard
NumberInput/Select composition on the sampled Linux configuration, including
nested Escape, prior plot focus return and input isolation. This is not a fix for
Popover or an approved change from an anchored editor to a modal workflow.

## Verification environment

The sandbox reports Linux and an accessible Wayland socket but hides `/dev/dri`.
An explicitly authorized check outside the sandbox found GPU devices. Isolated
GPU-backed testing runs outside the sandbox in a separate Sway 1.9 headless
compositor. The application reported Intel Iris Xe (ADL GT2), using Blade/Vulkan
and Wayland. Windows and macOS native sessions are not available here.
No existing user windows or services have been modified for this checkpoint.

Initial GitHub reads failed; later API and Git reads succeeded and agreed on main.
The branch starts from the locally accepted planning/policy branch, stacked on
the still-unmerged PR #125. This task does not merge that parent.

## Run the fixture

The standalone Cargo example is named `ui_compatibility`. Once built, run it in a
real GPU-backed desktop session; do not run the production settings workflow to
substitute for its in-window input checks.

```sh
cargo run -p argand --example ui_compatibility --release --locked
```

Use **Modal Dialog** for the separate standard modal-composition experiment. It
retains its own NumberInput/Select state; it is not an approved production editor.
The known-crashing Popover composition is disabled in normal launches. To expose
its explicitly labelled failure probe, run:

```sh
cargo run -p argand --example ui_compatibility --release --locked -- --popover-crash-probe
```

Opening that probe and then its Reducer dropdown is expected to terminate the
fixture on the locked graph. It is not necessary for trying the Dialog candidate.

The fixture uses standard Root/frame, TitleBar, Button, NumberInput, Select,
Dialog, opt-in Popover, PopupMenu, Tooltip and resizable components. Plot counters observe probe
events, not production worker generations. Preserve the stock behavior when
recording a failure; do not patch the toolkit to make the checkpoint pass.

## Native case ledger

Record separate rows for scale/theme/orientation/backend combinations, with the
exact tested revision/build and evidence. Screenshots prove appearance only;
event traces or witnessed input sequences are required for interaction claims.

| Case | Input / expected observation | Environment / result | Evidence |
| --- | --- | --- | --- |
| Baseline | Current production app: frame, navigation, hints, menu Escape and splitter behavior | Current release opened and rendered m39.wav; interaction matrix outstanding | See native report; startup is not a navigation/frame pass |
| R1 | One top-level Root and one frame, no duplicate border/shadow/insets | Fixture opened with one visible title/frame; remaining geometry checks outstanding | Static Root composition and limited light/dark screenshots only |
| R2 | Resize all edges/corners, maximize/restore/fullscreen/tiling; cursors match visible edges and reset immediately | FAIL in the standard Root fixture from owner use: stale cursor, displaced right edge and impractical diagonal target; the current production frame does not exhibit these failures | Stock `window_border` is private and calculates against reported `window_bounds()`; capture viewport, pointer and pre/post geometry before reconsidering |
| R3 | Bare title drags/zooms reliably; toolbar controls do not trigger title gestures | Standard fixture bare-title double-click was unreliable; maximize-button double-click toggles twice in current production but is explicitly accepted | Retain current working bare-title behavior and collect exact standard-fixture evidence; the accepted button sequence is not a blocker |
| F2 | In-popover input: caret, selection, clipboard, undo/redo, numeric validation, IME and Tab traversal | Ctrl+A/type/Enter/Escape/reopen retained 4096; remaining cases not exercised | Keyboard sequence in native report is not a full F2 pass |
| F3 | Nested Select/menu Escape, dismissal and next-key focus destination | FAIL: opening Select in Popover panics, including revised build; Escape did not restore prior plot focus | Native reproduction and revised focus sequence; nested-menu keyboard cases remain unexercised |
| P1 | Click/wheel/drag controls over the plot; plot counters must not change unintentionally | Native run not exercised | Visible probe counters required |
| P2 | Passive hint: arrow and no underlying readout/Alt guides; baseline click/wheel compatibility | Target arrow observed, but plot Alt guides remained visible; moving into tooltip position dismissed it | Limited revised-build sequence; full covered-surface/click/wheel checks remain outstanding; #122 unchanged |
| B1 | Standard Button and caption-control normal/hover/pressed/disabled/focus, release outside | Dark minimize and maximize/restore feedback is not visibly distinct in current production; owner selected a shared-theme correction | Both current and standard controls use global secondary tokens; validate the coherent palette correction across all affected controls |
| Z1 | Resizable panels: both orientations, live sizes vs callback count, release outside/focus loss | Horizontal live dimensions changed before release; vertical/lifecycle not exercised | Witnessed multi-motion drag and before/held/released screenshots in native report; not a full Z1 pass |
| Dialog comparison | Separate modal NumberInput/Select, nested Escape, keyboard/mouse isolation, retention | Limited Linux sequence passed; original Popover still fails with explicit opt-in | [Exact build, input ledger and limits](dialog-results.md); does not supersede anchored-editor F2/F3 acceptance |
| Compact fixture | Toolbar wraps; window metrics have a separate row | Observed at 720×520 content size in light mode; dark modal dropdown also opens | [Compact evidence](dialog-results.md); no frame appearance approval or R2 pass |
| Cross-platform | Repeat required cases on Linux, Windows and macOS | Windows/macOS not exercised | No cross-platform native evidence is available in this environment |

## Local checks and review

The Dialog continuation passed formatting, all-target Clippy, workspace tests and
**nine** example tests, followed by both release builds. Independent code review
of the bounded continuation found no substantive findings; the subsequent final
evidence review was also clean. No findings were rejected. Its new native ledger
is separate from the initial results below.

The initial corrected fixture passed the full local formatting, all-target Clippy and
workspace test gate, its eight example tests, and both production/example release
builds. The existing proc-macro-error2 future-incompatibility notice is not a
new lint suppression. These checks validate the fixture build, not native compatibility.

Independent review round 1 found three substantive fixture defects, all accepted:
mislabelled reported window bounds, insufficient passive-hint/Alt observability,
and no focus-owner/next-key observation. All three were corrected, along with a
fixture-owned NumberInput step/validation adapter found by direct inspection.
Round 2 found no remaining substantive code defects, but requested two documentation
corrections: current-versus-historical validation status and backend-dependent
reported-bounds terminology. Both were addressed; round 3 was clean for this
explicitly partial Draft handoff. The reproduced toolkit panic remains visible
rather than being bypassed. A clean implementation review is not a compatibility pass.

## Required handoff

Keep #126 open until the runnable fixture, local checks, independent review and
required native evidence are complete. If native frame behavior fails, provide a
minimal reproduction and carry it into #137 before production Root adoption. If
any platform cannot be exercised, record it as outstanding rather than marking
the gate passed.
