# Issue #101: Hide Analysis time after active mouse or keyboard input

Resolves #101.

Complexity class **A**: implementer `gpt-6-astra` at xhigh reasoning effort,
reviewer `gpt-5.6-sol` at xhigh reasoning effort, per "Agent roles and model
selection" in `AGENTS.md`. Raised from B before implementation began, after the
owner added the cursor-readout work and five planning rounds showed the input
model is harder than it looked.

## Overview

The completed-analysis timing stays on screen for as long as the state is ready, so
it reads as a permanent property of the file rather than feedback about the work that
just finished. Treat it as transient: show it when an analysis completes, and drop it
on the first deliberate input after that.

The whole ready status goes, not only its tooltip: the `ready in 1.234 s` text, the
`Analysis time` hint behind it, and the bare `ready` word. A new completed analysis
shows its own timing again, so opening a file runs the cycle from the start.

The cursor readout appearing also dismisses it: the two share the status bar and
compete for the same glance.

The readout itself is re-laid out at the owner's request. Today it is one field
reading `0.123 s · 1234.5 Hz · -42.3 dBFS`, where a negative level and a long
frequency both sit behind a `·` and are hard to pick out. Frequency comes first,
then time, and the level moves to a field of its own.

Out of scope: the `opening...`, `analysing... N%` and failure states, which are not
timing and must keep showing; the status bar's other groups; the cursor readout; and
the status shown when no file is open.

## Context

- `crates/app/src/document.rs` holds `Status`. `Status::Ready { elapsed }` is the only
  variant with timing; `Status::hint` returns the `Analysis time` `MetadataHint` for
  it and `None` for the others, and `Status::message` renders `ready in ...`.
- `crates/app/src/document.rs:231` is where `Status::Ready` is set, on
  `Update::Ready`. That is the one place a new completed analysis becomes visible.
- `crates/app/src/settings_ui.rs` draws the `#analysis-status` element: the text from
  `status().message()` and, when present, a tooltip from `status().hint()`. When no
  file is open it substitutes the literal `ready`.
- GPUI dispatches mouse events in two passes over the window's listeners
  (`window.rs:3694`): a capture pass front-to-back over every listener, then a bubble
  pass. Either stops early only if something in that same pass clears
  `propagate_event`. `div`'s `on_scroll_wheel` and `on_mouse_down` register bubble
  handlers, so a `stop_propagation` in them cannot suppress a capture-phase listener.
- `Window::on_mouse_event` registers a window-level listener that receives the
  `DispatchPhase`, and must be called during paint. It is generic over one concrete
  event type and downcasts to exactly that type (`window.rs:3421`), so a registration
  for `MouseDownEvent` never sees a `ScrollWheelEvent`. A generic wrapper cannot be
  written in this crate: `MouseEvent` extends a sealed `InputEvent`
  (`interactive.rs:9`).
- Key dispatch resolves bound actions *before* the key event reaches any listener.
  When an action consumes the keystroke, `dispatch_key_event` returns at
  `window.rs:3841` without ever calling `finish_dispatch_key_event`, so
  `Window::on_key_event` never runs. F10 opens the application menu through
  `OpenApplicationMenu` and would be missed.
- `App::observe_keystrokes` (`app.rs:1628`) is the one keyboard hook that sees
  everything. `dispatch_keystroke_observers` is called on all three paths: after an
  action consumed the keystroke (`window.rs:3837`), after ordinary key dispatch
  (`window.rs:3869`), and on replay (`window.rs:3959`). It returns a `Subscription`,
  which the shell must retain, as it already does for the window appearance. An
  incomplete chord is the one keystroke it does not report: that path returns at
  `window.rs:3829` before the observers.
- The wheel is stopped in two separate bubble handlers: `Self::wheel`
  (`navigation_ui.rs:419`) after it navigates, and the menu overlay
  (`app_menu_ui.rs:402`) which swallows it outright. Neither is reachable from a
  handler placed on the shell root with `div`.
- `navigation_ui.rs:566` builds the readout as a single string,
  `"{time} s · {frequency} Hz · {level}"`, and `settings_ui.rs` draws it in one
  `#cursor-readout` element. It returns `None` unless the pointer is inside the
  navigation area, time only outside the spectrum, and `—` where no level is known.
- `self.pointer` is assigned at `navigation_ui.rs:519` and `:778`, and cleared at
  `:820`, `shell.rs:445` and `app_menu_ui.rs:137`.
- All 35 key bindings are single keystrokes, so the pending-chord path that skips the
  keystroke observers (`window.rs:3829`) is unreachable here.
- `gpui_component::PopupMenu` handles its keys through `on_action`, not
  `on_key_down`, and the action path notifies the keystroke observers at
  `window.rs:3837`. The main window is not wrapped in a `Root` and carries no other
  widget that consumes keys, so no third-party code hides a keystroke from us.
- PR #106 (`fix/102-status-range-click`, open) rewrites most of `settings_ui.rs`.

## Decisions

- **Only deliberate actions dismiss it**: a mouse button press, a key press and a
  wheel event. Pointer motion, window activation and focus changes do not. The owner
  settled this: counting motion would remove the number before it can be read, since
  the pointer is usually already over the window when an analysis completes.
- **The whole ready status is dismissed together**, text and hint, as the owner
  directed. Hiding only the tooltip would leave the number on screen permanently,
  which is the complaint.
- **Dismissal is reset in `Shell` after `Document::apply` leaves `Status::Ready`**,
  not on file opening. Only `Update::Ready` leaves that status, so every completed
  analysis gets to show its timing once while the presentation flag stays in `Shell`.
- **The decision is a method on `Status`** taking the dismissed flag and returning
  what to draw, so it is tested without a window and `settings_ui.rs` gains one
  condition rather than a branch. That also keeps the footprint in `settings_ui.rs`
  small, which matters while PR #106 is rewriting that file.
- **The mouse is observed at window level in the capture phase**, with one
  `Window::on_mouse_event` for `MouseDownEvent` and one for `ScrollWheelEvent`,
  registered together during paint. Each acts only when the phase is `Capture` and
  never touches `propagate_event`, so nothing it sees is consumed or altered. The
  capture pass runs over every window listener before any bubble handler, so
  `Self::wheel` and the menu overlay cannot hide an event from them.
- **The keyboard is observed with `App::observe_keystrokes`, not
  `Window::on_key_event`.** A keystroke bound to an action never reaches a key
  listener, and F10 is exactly that case. The keystroke observers are notified on the
  action path, so one subscription covers every bound shortcut, including the arrow
  keys that pan the spectrum and the zoom commands.
- **Our two own interceptors call the same dismissal directly.** `intercept_keystrokes`
  (`navigation_ui.rs:63`, Ctrl+G, Ctrl+T and the zoom keys in the Plot context) and
  `application_menu_key` (`app_menu_ui.rs:160`, the keys that drive an open menu)
  both stop propagation before the observers run. Each gains one call. They are the
  only two, they are ours, and the context notes above establish that nothing else in
  the main window swallows a keystroke.
- **The readout appearing dismisses the timing too**, set where `self.pointer` is
  assigned rather than while drawing, so rendering stays free of side effects and the
  timing does not come back when the pointer leaves.

  This replaces two earlier versions of this decision, both wrong. The first assumed
  a wheel event handled by a plot bubbles to the shell root; `Self::wheel` stops it.
  The second added a second observation point in `Self::wheel` itself, which still
  missed the menu overlay swallowing the wheel outright. Chasing each element that
  stops propagation is the wrong shape: it needs a new call site for every future
  handler and fails silently when one is forgotten. Observing before the bubble pass
  begins is complete by construction.

- **The readout becomes two fields**: `#cursor-readout` keeps frequency and time in
  that order, and the level moves to its own `#cursor-level` field beside it. A field
  boundary separates the level far better than another `·`, which is the complaint.
- **Nothing about the measurement changes.** `Status::Ready { elapsed }` keeps its
  value, the analysis result is untouched, and the status is still `Ready` — only its
  presentation is suppressed.

## Rejected alternatives

- **Clearing `elapsed` from the status.** It would destroy the measurement to hide
  it, and the Issue requires stored timing to survive.
- **A timeout.** Not asked for, needs a UI timer, and makes the moment the number
  disappears depend on how long the person was reading it.
- **Dismissing on pointer motion.** Ruled out by the owner; the number would rarely
  survive long enough to be read.
- **Keeping the `ready` word after dismissal.** The owner asked for the whole status
  to go.
- **`Window::on_key_event` for the keyboard.** It never runs for a keystroke a bound
  action consumed, so every shortcut in the application -- F10 among them -- would
  fail to dismiss the timing.
- **Adding a dismissal call to each handler that stops the wheel** -- `Self::wheel`
  and the menu overlay today. It works, but it needs a new call site whenever another
  handler starts consuming input, and forgetting one fails silently.

## Owner follow-up after the first GPU review

Three changes requested after seeing the running build. They belong here rather than
in a new Issue: two of them correct the readout this plan just re-laid out.

- **No `ready` before a file is open.** `settings_ui.rs` substitutes a literal
  `ready` when there is no file. Nothing has been analysed at that point, so the
  status group shows nothing at all until a file is opened.
- **The status bar reads the same as the Alt guide badges.** Those badges already
  format both coordinates correctly, in `cursor_guides.rs`: `Readout::at` asks
  `time_ruler::Ruler::readout` for the time, which honours the clock, seconds and
  `#`-prefixed sample modes, and derives the frequency unit from
  `axis::caption(AxisKind::Frequency, ..)`, dividing by the matching factor and
  choosing precision from the span per device pixel.
- **The status bar follows the rulers.** That is the same requirement seen from the
  other side, and reusing the badge formatter satisfies it by construction: switch
  the time ruler to samples and the status bar says `#1234567` too.

`navigation_ui.rs::cursor_readout` currently formats its own `{time:.n$} s` and
`{frequency:.n$} Hz`, which is the second implementation that must go. It keeps the
parts the badges have no equivalent for: choosing the extents (the minimap substitutes
the full capture duration), reporting time alone outside the spectrum, and the dBFS
level with its `—` for an unknown value.

The two must agree by sharing one formatter, not by being written to match. If
`Readout::at` needs something `cursor_readout` cannot supply, widen it as little as
possible; do not copy its formatting.

## Implementation steps

- [x] Add the presentation decision to `Status` in `document.rs`: given a dismissed
      flag, answer with the message and hint to draw, or nothing for a dismissed
      ready state, leaving every other state unaffected.
- [x] Cover it in `document_tests.rs`: a dismissed ready state draws nothing while
      `Ready { elapsed }` keeps its value; `Opening`, `Analyzing` and `Failed` are
      unaffected by the flag; an undismissed ready state is unchanged from today.
- [x] Hold the dismissed flag in `Shell` and clear it after `Document::apply` leaves
      `Status::Ready`, so each completed analysis shows its timing once.
- [x] ➕ Restore the original `Document::apply` signature after review, keep the
      dismissal reset in `Shell`, and test that only ready updates leave ready status.
- [x] Observe the mouse during paint with `Window::on_mouse_event` for
      `MouseDownEvent` and for `ScrollWheelEvent`, acting only in
      `DispatchPhase::Capture` and leaving `propagate_event` untouched.
- [x] ➕ Observe the keyboard with a retained `App::observe_keystrokes` subscription,
      so a keystroke consumed by a bound action still dismisses the timing.
- [x] ➕ Call the same dismissal from `intercept_keystrokes` and from
      `application_menu_key`, the only two places that stop a keystroke before the
      observers run.
- [x] ➕ Dismiss the timing where `self.pointer` is assigned, when the pointer is
      somewhere the readout reports.
- [x] ➕ Centralize pointer assignment and readout dismissal in one helper used by
      pointer motion and time-menu dismissal, as requested in review.
- [x] ➕ Put frequency before time in the readout and move the level into its own
      status-bar field, keeping the existing precision rules and the `—` for an
      unknown level.
- [x] Draw through the new decision in `settings_ui.rs`, keeping the change to the
      `#analysis-status` element minimal.
- [x] Update the status-bar paragraph of `AGENTS.md` to state that the ready status
      and its timing hint are transient and dismissed by deliberate input.
- [x] ➕ Add the user-visible entry `CONTRIBUTING.md` requires to `CHANGELOG.md`.
- [ ] ➕ Show no status group at all before a file is open, in place of the literal
      `ready`.
- [ ] ➕ Format the status bar's time and frequency through the same code the Alt
      guide badges use, so both honour the time-ruler mode and the frequency unit of
      the axes. Keep the extents selection, the outside-spectrum case and the level
      field as they are.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Owner validation on a real GPU session: open a file, read the timing, then
      click and confirm the whole ready status goes; confirm a keystroke and a wheel
      scroll do the same; confirm moving the pointer alone does not; confirm the
      gesture that dismissed it still did what it was meant to do — a pan still pans,
      a shortcut still fires; confirm F10 dismisses it while still opening the menu;
      confirm Ctrl+G and the arrow keys that pan the spectrum dismiss it; confirm
      moving the pointer over the spectrogram dismisses it; read the new readout at a
      megahertz sample rate and confirm frequency, time and a negative level are each
      easy to pick out; switch the time ruler through clock, seconds and samples and
      confirm the status bar follows each one and matches the Alt badge for the same
      point; confirm the status bar shows nothing before a file is opened;
      confirm a wheel scroll over an open application menu
      dismisses it too, while the menu keeps swallowing the scroll; open another file
      and confirm the timing appears again;
      confirm `analysing...` and a failure message are never suppressed.

Local formatting, Clippy, tests and the subsequent release build passed again
after both round-two review fixes.
The test suites reported 563 passed and
zero failures, including the three status presentation and ready-transition tests. The five
optional real-capture fixtures and the half-hour capture are absent in this
worktree, so their end-to-end cases skip the capture checks. Cargo reports a
future-incompatibility warning for the existing `proc-macro-error2 v2.0.1`
dependency.

Owner validation on a real GPU remains pending, so the complete-validation step
is still open. The plan stays in this directory for the implementation handoff.

## Post-completion

- Reconcile with PR #106 if it merges first, since it rewrites `settings_ui.rs`.
