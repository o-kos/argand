# Issue #261: Unify plot navigation and selection gestures

Resolves #261.

## Overview

Use one physical-axis gesture scheme on the spectrogram and both rulers. Preserve
capture data, history, keyboard/menu commands, coordinate guides and the minimap.

## Roles and risk

- Class: A, because the interaction and regression-test change may exceed 400 lines.
- Coordinator (primary): `gpt-6.1-sol` medium, owner communication and final assessment.
- Executor: `gpt-6.1-sol` medium, implementation, validation and PR operations.
- Reviewer: `gpt-6.1-sol` high, read-only in a fresh context.
- Consultant: `gpt-6.2-astra` high through the installed read-only CLI, only if a concrete unresolved question warrants it.

## Context and decisions

Existing PlotView owns transient gestures and emits toolkit-neutral selection and
viewport intents. Reuse it and existing canvas interactions without new controls,
new dependencies or changes to DSP, sample ownership, analysis or rendering lifetime.
The prerequisite role-policy documentation commit remains preserved in this branch.

Choose the gesture at mouse-down. Space/middle pan wins. On the spectrogram,
Shift or Ctrl alone starts a new band and Ctrl+Shift a rectangle before edge/interior grabs.
Alt adjusts existing nearest boundaries, with no selection yielding no gesture.
Unmodified presses retain direct edge resizing and interior movement. Rulers select,
resize or adjust their own physical axis. Wheel modifiers never redirect a ruler.

The owner approved Ctrl+left drag as an alias for Shift+left drag during Draft
feedback. Plain left remains time selection. Refresh the affected gesture,
cursor and discoverability evidence in both orientations, including release of
modifiers over an existing selection. Preserve the reviewed minimap behavior.

Draft feedback also reproduced a pan dead zone after overshooting either capture
bound: the original pointer/view anchor retained the overshoot. Rebase only the
bounded axis to its actual view and current pointer, preserving other-axis movement,
fitted axes, ruler ownership and minimap mapping. Verify immediate reversal for
middle and Space+left in both orientations, including pointer travel off canvas.

Further owner acceptance failed. A production reproduction established a separate
failure away from capture bounds: crossing a tick-label digit threshold widens the
right ruler, and Shell's layout feedback cancelled the held pan unconditionally.
The previous PlotView-only regression did not include this Shell feedback path.
Preserve gestures on geometry-only measurement when actual bounded views remain
unchanged. Rebase changed pan scales from the last requested view and input position,
without relying on a possibly older rendered snapshot. Preserve cancellation for
actual bounds corrections and capture/settings changes. No analysis scheduling,
generation, retention or texture-lifetime contract changes are planned.

The owner's precise two-gesture report prompted a further production diagnosis:
vertical time zoom, drag down to zero and beyond, release, then a new drag up.
Rapid input reproduced a discontinuous jump because the new press read a previous
frame's time view after Shell had already accepted the zero clamp. Paced input
did not reproduce the jump, and sustained jitter or reversed movement remains
unconfirmed. Refresh only current accepted navigation and coherent extents in the
existing Shell capture-phase mouse-down observer before PlotView handles the press.
Do not rebuild presentation resources or introduce a second navigation owner.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits |
| --- | --- | --- | --- |
| Spectrogram wheel selects time or frequency correctly | Default/Shift pan and Ctrl/Ctrl+Shift zoom in both orientations | Passed automated tests and production Xvfb run | Native platforms separate |
| Rulers operate on their own axis | Modifier combinations for wheel, selection, resize, Alt and pan | Passed automated modifier matrix and production Xvfb scenarios | Native platforms separate |
| Selection creation and modification | Time, band, rectangle, no selection, edge/interior, Alt, modifier release | Passed automated tests and fitted/zoomed production Xvfb scenarios | Disposable capture matches regenerated original bytes |
| Ctrl and Shift band aliases | Existing rectangle, modifier release, Ctrl+Shift precedence, rulers, cursor and help in both orientations | Passed refreshed automated and production Xvfb checks | Owner acceptance remains pending |
| Pan reversal at capture bounds | Both bounds, spectrum and each ruler, both orientations, middle/Space, fitted axes and off-canvas travel | Regression failed before correction and passed after it; refreshed production Xvfb scenarios passed | No native desktop or platform-wide claim |
| Existing navigation/focus paths remain usable | Keyboard/menu zoom, fit, arrows, Escape and Alt guides | Passed automated tests and production Xvfb scenarios, including Edit menu selection panel | Window-manager interception remains untested |
| Production UI matches the agreed scheme | Isolated Xvfb production release run with disposable fixtures and settings | Passed scoped scenarios on 2026-11-16 | Does not establish Wayland, Windows or macOS input behavior or owner acceptance |
| Discoverable help matches gestures | README/current navigation documentation and existing help surface | Updated and production selection tooltips checked | No new custom widget |

## Implementation steps

- [x] Correct wheel routing and mouse-down selection priorities.
- [x] Add time-ruler edge/Alt support and retain latched gesture semantics.
- [x] Add focused event-level and geometry regression coverage.
- [x] Update discoverable help, current navigation documentation and changelog.
- [x] Complete local gate and isolated production UI acceptance.
- [x] ➕ Preserve modified minimap navigation and Alt edge resizing after the independent review finding, then refresh affected validation.
- [x] ➕ Add the owner-approved Ctrl band-selection alias, matching cursor/help, focused regressions and current production evidence.
- [x] ➕ Correct and validate immediate pan reversal after overshooting either axis boundary.
- [x] ➕ Correct tick-label growth cancelling held gestures and validate real Shell layout feedback, resize continuity and genuine bounds cancellation with focused tests.
- [ ] ➕ Correct a new gesture using a previous frame's navigation anchor and refresh the exact two-gesture production reproduction.
- [ ] ⚠️ Refresh affected production acceptance after the remaining code gate. The earlier owner acceptance failed, and the new source has not been built as a production binary.
- [ ] ⚠️ Reconsider review coverage after this failed acceptance. Three review rounds have already completed, so another independent round requires an explicit owner waiver of the limit.
- [ ] Complete independent review and coordinator assessment.
- [ ] Move this plan to completed before final review, once acceptance is complete.

## Validation

The checked results below are historical evidence for the Ctrl alias and first
pan-boundary correction, not validation of the subsequent layout correction.

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked` (937 passed after the Ctrl alias and pan correction)
- [x] `cargo build --release --locked`, after the preceding checks
- [x] Production Xvfb interaction scenarios, bound to exact revision and binary

The production run used a disposable 1 MHz complex chirp and isolated settings,
state, cache and X11 display. Both orientations were exercised with fitted and
zoomed axes. Fast single-move synthetic drags did not consistently establish a
selection in the vertical run, so those attempts are excluded from passing
evidence. Paced drags with intermediate moves established the recorded results.
This run does not diagnose the earlier host freeze. Independent review completed
in two rounds, with the first finding corrected and the second round clean. The
coordinator's assessment and owner acceptance remain pending; keep the PR Draft.
The approved Ctrl alias and pan-boundary refinements passed a refreshed sequential
local gate and production release run. The additional pan regression covers both
bounds, orientations, spectrum/rulers and middle/Space+left input, including travel
beyond the canvas. Production cursor observations match Ctrl to Shift selection
feedback. The third independent review round returned no substantive findings
on these refinements and confirmed the earlier minimap finding remains resolved.
No findings were declined. Coordinator assessment and owner acceptance remain
pending; the accumulated review count is three rounds.
The first review identified modifier handling that also affected the minimap.
Restored its earlier Ctrl/Shift navigation priority and Alt navigation/edge grabs.
A new regression failed before the correction and passed afterwards. The complete
local gate passed again, and the rebuilt production binary passed the affected
minimap scenarios in both orientations plus selection/Alt smoke checks. The
unaffected broad interaction record is retained separately from this refresh.

The layout-correction iteration is currently limited to focused application
regressions and Clippy. Three Shell regressions pass, covering label growth in
both orientations with middle/Space+left, both-axis resize continuity and actual
rebound cancellation. A filtered navigation/UI run passed 51 tests, including
the earlier boundary-reversal regression. Full workspace tests, the pre-push hook and a new release
build remain deferred while the test-memory diagnostic task is paused and the
coordinator determines the next validation sequence. Keep the PR Draft. Production
acceptance must cover tick-label growth while holding middle and Space+left,
continued movement and reversal without repressing, both orientations and ruler
ownership, spectrum diagonal movement, actual capture bounds and a dimension change.
Record settled frames separately from transient rendering delay. Do not infer that
the identified cancellation explains every symptom reported by the owner.

For a manual acceptance iteration, the owner explicitly requested a release
rebuild without further checks. That build completed for the layout correction.
The later stale-anchor correction was also rebuilt without tests, formatting,
Clippy or review for owner manual testing. The owner reported improvement, followed
by a remaining same-held outward reversal. Focused isolated X11 reproduction with
monotonic downward input and stationary holds did not reproduce that reversal.
Explicit one-pixel reverse input moved off the bound, but this does not establish
the owner's cause. Keep the remaining defect unresolved.

Add opt-in production diagnostics under `argand::pan_trace=trace` using the existing
per-launch `RUST_LOG` subscriber. Record raw pan input, a sequence identifier carried
into its computed/accepted Drag intent, anchors, layout and cancellation reasons,
and the CPU canvas paint model. Paint timestamps do not establish GPU completion.
The trace remains off by default, streams to stderr without an in-app event buffer,
and records no capture paths or environment. It changes no gesture behavior.
The owner's trace established six active pans cancelled by the wheel handler while
subsequent mouse-move events still reported Middle held, explaining the late loss
of dragging. Preserve the latched pan's navigation priority by consuming incoming
wheel events before they can cancel state or issue navigation. Apply this to both
middle and Space+left pan, without changing ordinary wheel or minimap selection
behavior. Keep tracing enabled only by opt-in so the next manual run can verify
the discarded events. The source of overlapping wheel events is not established.
Earlier recorded departures from zero accompany upward pointer coordinates and
matching accepted/painted views. This does not establish all reported reversal
symptoms as resolved.
The next diagnostic release is built alone for manual reproduction, with tests,
formatting, Clippy, review and production checks intentionally skipped at the owner's
request. Review rounds remain at three, with no additional round started.

## Owner handoff

The owner manually accepted the current work and requested a commit and remote
push without checks to continue on another computer. The latest source was built
successfully in release mode. The subsequent trace showed stable unclamped pan
gain across eleven gestures and no wheel cancellation of held pans. This does not
replace automated validation or establish native presentation latency.

The final layout, new-gesture anchor, pan tracing and held-pan wheel-priority
changes have not completed the full local gate or independent review. The three
earlier review rounds apply to their recorded snapshots. Skip the pre-push hook
for this owner-authorized handoff only; do not claim it passed. Keep the Pull
Request Draft, without merging or publishing a release. Resume the outstanding
validation and review decision on the other computer before final integration.

## Remote integration and validation refresh (2026-11-23)

The owner requested updating this existing feature branch from the remote,
integrating upstream changes, running all necessary tests and pushing normally.
This request supersedes the earlier one-time no-check handoff. Preserve the
published feature history with a merge of `origin/main`, without rebasing or
force-pushing. Do not merge into `main`, publish a release or change PR readiness.

Risk remains class A. The coordinator and executor remain `gpt-6.1-sol` medium.
The reviewer assignment remains `gpt-6.1-sol` high, with three historical rounds
already used. Any further required independent review needs the owner's decision
about that limit. The current consultant default is `gpt-6.2-astra` high through
the installed read-only CLI, only for a concrete unresolved technical question.

- [x] Integrate upstream while preserving feature gestures and upstream GPUI test cleanup.
- [x] Run formatting, all-target Clippy and all workspace tests with the locked graph.
- [x] Run applicable packaging, CI-policy and checkout-identity checks.
- [ ] Build release after all preceding checks pass, binding the artifact to the submitted source.
- [ ] Push the feature branch through its normal pre-push hook and verify the remote tip.

The selected immutable SDK and existing installed Rust must match the repository
pin. Keep all application artifacts in this workspace's `target/`. This refresh
adds no intended UI behavior, toolkit graph or production ownership changes.
Conflicts were confined to consultant policy, independent changelog bullets and
renamed/expanded gesture tests. Retain feature behavior together with upstream
cleanup guards, including feature-only GPUI tests, and use the upstream fake
document seam for the Shell pan fixture. No production conflict resolution was
needed. The first formatting check found only formatting debt in the final
pre-handoff gesture/trace changes. Applied `cargo fmt --all` without semantic edits.
The first Clippy run found excessive nesting in the final capture mouse-down
observer and an overlong spectrogram function after inline trace emission. Flatten
the phase guard and move that same trace emission into the diagnostics module,
without changing event behavior, trace fields or paint ordering. No lint
suppression or threshold change is used.
The saved owner policy edit is identical to the merged upstream file.
Current local results after the integration and lint refactor:

- Formatting and all-target locked Clippy passed.
- `cargo test --locked` passed all 953 tests with zero failures or ignored tests.
  This includes all three Shell layout regressions and bounded-pan reversal.
  The ordinary suite used its existing parallelism and SDK setup: elapsed 2:32.02
  including compilation, maximum RSS 2,084,160 KiB, no swaps, exit status 0.
- Packaging policy: 9 passed. CI aggregate policy: 3 passed. Ubuntu source
  isolation: 9 passed, with signed local repositories and no host installation.
- Checkout identity regression passed, including its stale compile-time negative
  control. Release-notes shell syntax passed.

These are native SDK-mode local checks, not the portable source-toolkit matrix or
native desktop/GPU acceptance. The earlier no-check waiver is no longer used.
Historical production/owner acceptance above stays scoped to its recorded source;
this task does not claim new native-platform or GPU/desktop acceptance. Keep the
PR Draft and its remaining completion/review decisions visible.

## Additional review finding (2026-11-23)

The owner explicitly authorized one additional independent `gpt-6.1-sol` high
review after the earlier three rounds. It found one P2: at the time bounds,
sub-sample inward pointer steps round back to the bound, and unconditional
boundary rebasing discards their cumulative displacement. FFT/view length 64
and a roughly 1000-pixel time axis make this reachable. The finding is accepted.
No other actionable findings were reported. That review covered the integrated
snapshot before this correction; it does not establish review of later changes.

- [x] Reproduce incremental inward panning at both bounds in both orientations and on spectrum/ruler entry paths.
- [x] Rebase only genuine boundary displacement, preserving sub-sample accumulation and immediate overshoot reversal.
- [ ] Refresh the full local gate, release build and affected isolated production acceptance.
- [ ] Coordinator assessment and review-policy decision for the corrected snapshot.

The expected interaction stays unchanged: inward movement accumulates until a
sample is reached, while outward overshoot is discarded so reversal responds
immediately. The new event-level regression failed before the correction: sixteen one-pixel
inward moves left the lower-bound time view at sample zero rather than sample one.
It passes after comparing unrounded displacement with the available samples from
the gesture anchor. All eight combinations of bounds, orientations and
spectrum/time-ruler entry paths pass. The existing overshoot/reversal and layout
regressions remain part of the required full suite. Production acceptance will
repeat deeply zoomed incremental drags and overshoot/reversal on an isolated Xvfb
display with disposable settings and capture data.
No additional control, dependency or toolkit change is needed.

## Post-completion

Merge and branch cleanup require the coordinator/owner completion decision and
current full CI. Release and tagging are outside this task.
