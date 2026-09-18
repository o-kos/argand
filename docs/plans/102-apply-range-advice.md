# Issue #102: Apply recommended range when clicking the yellow status-bar dB

Resolves #102.

Complexity class **B**: implementer `gpt-5.6-sol` at high reasoning effort,
reviewer `gpt-5.6-terra` at high reasoning effort, per "Agent roles and model
selection" in `AGENTS.md`.

## Overview

The yellow dB range in the status bar warns that the range could be improved but
offers no click action, so the advice is only reachable through Ctrl+R / Cmd+R.
Make the warned value apply the same advice on click, through the same action.

Out of scope: the analysis settings surface, the recommendation itself, the hint
shown on hover, and anything else in the status bar.

## Context

- `settings_ui.rs::analysis_control` draws one `Button` with id `analysis-settings`
  whose click calls `edit_analysis`, opening the settings window. The FFT size,
  window and range are children of that button, so a click anywhere on them
  currently opens settings.
- The range child turns yellow through `advice_color` when
  `self.range_recommendation()` is `Some`, and gains a `⚠` prefix.
- `UseRecommendedRange` already exists as an action, is bound to Ctrl+R / Cmd+R,
  and is handled application-wide in `shell.rs::bind_choose_file`. It owns the
  availability checks and the settings-preview behaviour.
- This work was previously attempted in PR #106 together with #108, which replaced
  the settings window with an in-window popover. That surface turned out to require
  hand-rolled controls, because `gpui_component`'s `Input` calls `Root::read` and
  `Root::update`, which panic through `expect` when the window root is not a `Root`,
  and wrapping the shell in `Root` would add a second window border. #108 stays on
  its own branch pending a separate decision; nothing from that popover is reused
  here.

## Decisions

- **Dispatch the existing `UseRecommendedRange` action.** The Issue requires the
  click and the shortcut to be the same path, so no recommendation is recalculated
  and no application logic is duplicated.
- **Only the warned value is clickable.** When there is no recommendation the range
  is ordinary text and the click belongs to the settings button, exactly as today.
- **The click must not reach the settings button.** The range is a child of it, so
  the handler stops propagation; without that, applying the advice would also open
  the settings window. `on_mouse_down` prevents the button's own press handling in
  addition to the click, since the button reacts to the press.
- **The pointer changes to indicate the action** only while the value is warned,
  so the affordance appears exactly where the click does something.

## Rejected alternatives

- **Recomputing the recommendation at the click site.** It would be a second path
  that can disagree with the shortcut, which the Issue rules out explicitly.
- **Moving the range out of the settings button.** It would change the status-bar
  layout and the hover behaviour for a click target, and the acceptance criteria ask
  for neither.
- **Reusing the popover work from PR #106.** It is blocked on a decision about
  `Root` and carries an unapproved custom control; this Issue does not depend on it.

## Implementation steps

- [ ] Make the warned range in `analysis_control` a click target that stops
      propagation and dispatches `UseRecommendedRange`, leaving the unwarned range
      as plain text inside the settings button.
- [ ] Give the warned value a pointer cursor, and only the warned value.
- [ ] Cover what can be tested without a window: that a recommendation is present
      exactly when the value is warned, and that the click path names the same
      action the shortcut uses.
- [ ] Update `AGENTS.md` where it describes the status bar's range warning.
- [ ] Add the user-visible entry `CONTRIBUTING.md` requires to `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Owner validation on a real GPU session with a low-amplitude capture: the yellow
      value applies the advice on click and does not open settings; the pointer shows
      the affordance only while it is yellow; clicking the rest of the summary still
      opens settings; Ctrl+R still works; and once the range is no longer warned the
      value stops being clickable.

## Post-completion

- None.
