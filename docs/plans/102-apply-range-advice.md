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

## Owner follow-up after the first GPU review

Four defects seen in the running build. All are in the same status-bar element and
belong here.

- **The FFT text brightens when the pointer is over the warned value.** The value is
  a child of the settings button, whose `on_hover` sets `analysis_hovered` and lifts
  the whole summary to the foreground colour. Hovering the warned value must not
  light up the rest of the summary, because the click there does something else
  entirely.
- **The warned value does not react to the pointer.** It is a click target and must
  brighten under the pointer, in both interface themes, without turning into the
  plain foreground colour that would make it look like the rest of the summary.
- **Applying the advice flickers through an intermediate state.** `⚠ 110 dB`
  becomes `110 dB` and only then `40 dB`. The cause is in `range_recommendation`:
  it returns `None` as soon as `file.displayed_settings != Some(self.settings)`, which
  is true the instant the click applies the new settings, while the displayed value
  still comes from the old picture. The warning is tied to the requested settings
  where it should be tied to the displayed picture.
- **The settings hint appears after the click.** The summary's hover tooltip must not
  open as a result of applying the advice.

## Follow-up decisions

- **Split the warning's two readers.** Display -- the colour, the `⚠` and the click
  target -- follows the displayed picture, so it survives until the replacement
  arrives and then changes together with the number, as one visible step. The action
  keeps the existing strict check, so a click during the gap cannot request settings
  that are already requested. Do not make the value flicker back by tying display to
  the requested settings again.
- **Hovering the warned value suppresses the summary hover state** rather than
  reordering the elements. The status bar's layout and the settings button must not
  change for this.
- **The warned value gets its own hover colour**, derived from the advice colour
  rather than from the theme foreground, so it stays recognisably the warning.

## Second follow-up: split the summary into two status items

This follow-up supersedes the earlier nesting, propagation and unchanged-layout
decisions while retaining the displayed-picture and strict-action split.

The first follow-up treated the symptoms of a structure that cannot work. The warned
value is a child of the settings button, so both own the pointer: entering the value
first raises the button's hover state and then lowers it again, which reads as the FFT
text flashing white and going out. Any fix at that level is a matter of which handler
runs first.

The owner's decision is to split the group into two independent status items, side by
side, each with its own hover state, colour and click.

- **`2048 · hann`** keeps everything about the transform. It is the settings button:
  clicking it opens the settings window, hovering it raises its own foreground and
  background, and it carries the analysis hover hint.
- **`40 dB`** carries only the level. When a recommendation applies it is the advice
  colour, shows a pointer, brightens under the pointer and applies the advice on
  click. Otherwise it is plain muted text that does nothing: no pointer, no hover
  reaction, no click.
- Neither shows the other's hover state, and the hint lives only on the first.

This removes the reason for `range_advice_hovered`, for suppressing the hint after a
click, and for stopping propagation on the value: nothing contains it any more.

The brightening must be visible. The first attempt raised lightness by 0.12 from the
advice colour and the owner could not see a difference; choose a step that reads
clearly on the warning colour in both interface themes.

## Third follow-up: three states for the level item

Owner request after the split. The level item gains a hint of its own, different in
each of its three states, and behaves like a button where it acts.

The states are read from the current range, with no remembered history:

| State | Shown | Hint | Action |
| --- | --- | --- | --- |
| Warned | `⚠ 110 dB` in the advice colour | says the range should be narrowed, carries the Ctrl+R keycap | applies the advice |
| Corrected | `40 dB`, ordinary text | says this is a corrected range, carries the Ctrl+R keycap | restores the full range |
| Full | `110 dB`, ordinary text | says this is the full range, no keycap | none |

- **Corrected means any explicit range**, `DynamicRange::Fixed(_)`, whether it came
  from the advice or from the settings editor. Full means `DynamicRange::Default`.
  Nothing is remembered about how the value got there, so the three states are a
  function of the current range alone.
- **Ctrl+R becomes a toggle.** With a recommendation it applies it, as now. Without
  one, on an explicit range, it restores `DynamicRange::Default`. On the full range
  it does nothing. The click on the level item does exactly what Ctrl+R does in that
  state, since they must not diverge.
- **`auto` behaves as the full range does**: informational hint, no action.
- **The warned and corrected states hover like a button**, with the same background
  the settings button uses and brighter text; the full state does not react.
- **No pointer cursor anywhere on this item.** The hover background is the
  affordance, as it is for the neighbouring button.
- Hints use the existing `shortcut_tooltip`, which takes the text and an optional
  action and renders the keycap itself. Do not add a new tooltip mechanism.

Suggested wording, to be corrected by the owner if it reads wrong. One sentence, no
terminal period, matching the house style for explanations:

- Warned: `Spectrum peak sits low in this range`
- Corrected: `Range narrowed from the full scale`
- Full: `Full scale, nothing trimmed`

## Consolidated model after three review findings

The plan grew in layers as the owner refined the interaction, and the last layer
contradicted the first. The external review found three consequences. This section
replaces the reasoning in the earlier follow-ups; where they disagree, this wins.

**One source of truth for the state, two readers with different needs.** The display
must follow the picture on screen, or applying the advice flickers through an
unmarked value. The action must follow the requested settings, or a click can request
what is already requested. Both remain true, but they may never contradict each other
in what the user is offered.

The rule: while the displayed picture does not match the requested settings, the
level item keeps its appearance from the picture and offers no action at all -- no
keycap in the hint, no click handler, no hover background. Nothing can diverge,
because nothing is offered. Once the picture catches up, the state and its action are
computed from the same range again.

**Hover tracks the pointer through every state.** Both the actionable button and the
inert item record hover entry and exit, so the flag remains accurate while analysis
changes the state. The hover effect is the flag and the state being actionable. A
pointer resting on the item therefore highlights a newly actionable state, while a
pointer that left during an inert state cannot leave a highlight behind. The action
handler does not clear the flag itself.

**The advice has exactly one application path.** `live_analysis_tooltip` still sets
`DynamicRange::Fixed(db)` directly instead of dispatching `UseRecommendedRange`. That
is the duplicate path Issue #102 rules out, and with Ctrl+R now a toggle it would
diverge at the first change to either side. It dispatches the action like everything
else.

## Implementation steps

- [x] Make the warned range in `analysis_control` a click target that stops
      propagation and dispatches `UseRecommendedRange`, leaving the unwarned range
      as plain text inside the settings button.
- [x] Give the warned value a pointer cursor, and only the warned value.
- [ ] ➕ Cover the click path through the owner's GPU validation because it requires
      a real window and has no meaningful window-free test.
- [x] Update `AGENTS.md` where it describes the status bar's range warning.
- [x] Add the user-visible entry `CONTRIBUTING.md` requires to `CHANGELOG.md`.
- [x] ➕ Tie the warning's colour, marker and click target to the displayed picture
      so applying the advice is one visible change, with no unmarked intermediate
      value. Leave the action's own check as it is.
- [x] ➕ Stop the pointer over the warned value from raising the summary's hover
      state, so the FFT text does not brighten.
- [x] ➕ Give the warned value a brighter hover colour derived from the advice
      colour, in both interface themes.
- [x] ➕ Keep the summary's hover hint from appearing as a result of clicking the
      warned value.
- [x] ➕ Split the analysis summary into two status items: the transform group as the
      settings button with the hint, and the level as its own item.
- [x] ➕ Give the level item its own warned behaviour -- advice colour, pointer,
      visible hover brightening, click applies the advice -- and make it inert plain
      text when there is no recommendation.
- [x] ➕ Remove `range_advice_hovered`, the hint suppression and the propagation stop,
      which the split makes unnecessary.
- [x] ➕ Give the level item a hint per state through `shortcut_tooltip`, with the
      Ctrl+R keycap in the warned and corrected states and none in the full state.
- [x] ➕ Make Ctrl+R a toggle: apply the advice when one exists, otherwise restore
      the full range from an explicit one, and do nothing on the full range. The
      click follows the same rule.
- [x] ➕ Give the warned and corrected states a button-like hover with background and
      brighter text, and leave the full state inert.
- [x] ➕ Remove the pointer cursor from the level item in every state.
- [x] ➕ Offer no action while the displayed picture does not match the requested
      settings: keep the appearance, drop the keycap, the click and the hover
      background.
- [x] ➕ Keep the hover flag synchronized in actionable and inert states, compute the
      hover effect from the flag and the state being actionable, and do not clear the
      flag in the action handler.
- [x] ➕ Make `live_analysis_tooltip` dispatch `UseRecommendedRange` instead of
      setting the range itself.
- [x] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Owner validation on a real GPU session with a low-amplitude capture: the yellow
      value applies the advice on click and does not open settings; the pointer shows
      the affordance only while it is yellow; clicking the rest of the summary still
      opens settings; Ctrl+R still works; and once the range is no longer warned the
      value stops being clickable. Confirm the four follow-up points: the FFT text
      does not brighten while the pointer is over the warned value, the warned value
      itself brightens, applying the advice goes straight from `⚠ 110 dB` to `40 dB`
      with nothing in between, and no settings hint appears after the click. Confirm
      the split: hovering the level never changes the transform group and hovering the
      transform group never changes the level; the level brightens visibly under the
      pointer; and an unwarned level does nothing on click and shows no pointer.
      Confirm the three states: each shows its own hint, the warned and corrected ones
      carry the Ctrl+R keycap and the full one does not; Ctrl+R applies the advice,
      then restores the full range, then does nothing; clicking does the same as
      Ctrl+R in each state; the hover background appears on the warned and corrected
      states only; and the cursor never becomes a hand anywhere on the item. Confirm
      that applying the advice on a large capture, where the new picture takes a
      moment, never offers an action that does something other than the hint says,
      and that the item never appears highlighted with the pointer elsewhere.

## Post-completion

- None.
