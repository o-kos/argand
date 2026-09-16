# Issue #101: Hide Analysis time after active mouse or keyboard input

Resolves #101.

Complexity class **B**: implementer `gpt-6-astra` at high reasoning effort,
reviewer `gpt-5.6-sol` at high reasoning effort, per "Agent roles and model
selection" in `AGENTS.md`.

## Overview

The completed-analysis timing stays on screen for as long as the state is ready, so
it reads as a permanent property of the file rather than feedback about the work that
just finished. Treat it as transient: show it when an analysis completes, and drop it
on the first deliberate input after that.

The whole ready status goes, not only its tooltip: the `ready in 1.234 s` text, the
`Analysis time` hint behind it, and the bare `ready` word. A new completed analysis
shows its own timing again, so opening a file runs the cycle from the start.

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
- GPUI offers `capture_any_mouse_down` and `capture_key_down` on `div`, which run in
  the capture phase before the event reaches any child. A child's `stop_propagation`
  happens in the later bubble phase and cannot suppress them.
- There is no `capture_scroll_wheel`. The only wheel handlers are
  `shell.rs:1099`, which calls `Self::wheel`, and a menu overlay in
  `app_menu_ui.rs:402`. `Self::wheel` ends with `cx.stop_propagation()`
  (`navigation_ui.rs:419`), so a wheel event over a plot or the time ruler never
  reaches a handler on the shell root. It returns early, without stopping
  propagation, for the minimap and when there is no geometry.
- PR #106 (`fix/102-status-range-click`, open) rewrites most of `settings_ui.rs`.

## Decisions

- **Only deliberate actions dismiss it**: a mouse button press, a key press and a
  wheel event. Pointer motion, window activation and focus changes do not. The owner
  settled this: counting motion would remove the number before it can be read, since
  the pointer is usually already over the window when an analysis completes.
- **The whole ready status is dismissed together**, text and hint, as the owner
  directed. Hiding only the tooltip would leave the number on screen permanently,
  which is the complaint.
- **Dismissal is reset where `Status::Ready` is assigned**, not on file opening. Every
  completed analysis then gets to show its own timing once, which is what the Issue
  asks for and which covers opening a file as one case among several.
- **The decision is a method on `Status`** taking the dismissed flag and returning
  what to draw, so it is tested without a window and `settings_ui.rs` gains one
  condition rather than a branch. That also keeps the footprint in `settings_ui.rs`
  small, which matters while PR #106 is rewriting that file.
- **Input is observed without ever being consumed.** The handlers only clear a flag
  and must not call `stop_propagation`, so no gesture, shortcut or action changes
  behaviour. Mouse buttons and keys are observed on the shell root in the capture
  phase, which a child cannot suppress.
- **The wheel needs two observation points, because GPUI has no capture variant for
  it.** `Self::wheel` stops propagation once it has navigated, so a wheel event over
  a plot would never reach the root. The dismissal is therefore invoked from
  `Self::wheel` itself, before it navigates, and from an `on_scroll_wheel` handler on
  the shell root that catches everything `Self::wheel` returns from early or never
  sees. Both call one method, so the two paths cannot disagree.

  This corrects an error in the first version of this plan, which assumed a wheel
  event handled by a plot still bubbles to the root. It does not.
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

## Implementation steps

- [ ] Add the presentation decision to `Status` in `document.rs`: given a dismissed
      flag, answer with the message and hint to draw, or nothing for a dismissed
      ready state, leaving every other state unaffected.
- [ ] Cover it in `document_tests.rs`: a dismissed ready state draws nothing while
      `Ready { elapsed }` keeps its value; `Opening`, `Analyzing` and `Failed` are
      unaffected by the flag; an undismissed ready state is unchanged from today.
- [ ] Hold the dismissed flag in `Shell` and clear it where `Status::Ready` is
      assigned, so each completed analysis shows its timing once.
- [ ] Set it from capture-phase mouse-down and key-down handlers on the shell root,
      without consuming any event.
- [ ] ➕ Set it from `Self::wheel` before it navigates, and from an `on_scroll_wheel`
      handler on the shell root for the events `Self::wheel` does not stop, both
      through one method.
- [ ] Draw through the new decision in `settings_ui.rs`, keeping the change to the
      `#analysis-status` element minimal.
- [ ] Update the status-bar paragraph of `AGENTS.md` to state that the ready status
      and its timing hint are transient and dismissed by deliberate input.
- [ ] ➕ Add the user-visible entry `CONTRIBUTING.md` requires to `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Owner validation on a real GPU session: open a file, read the timing, then
      click and confirm the whole ready status goes; confirm a keystroke and a wheel
      scroll do the same; confirm moving the pointer alone does not; confirm the
      gesture that dismissed it still did what it was meant to do — a pan still pans,
      a shortcut still fires; open another file and confirm the timing appears again;
      confirm `analysing...` and a failure message are never suppressed.

## Post-completion

- Reconcile with PR #106 if it merges first, since it rewrites `settings_ui.rs`.
