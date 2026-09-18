# Issue #99: Simplify menus and unify time/frequency navigation commands

Resolves #99.

## Overview

The application menu carries entries that duplicate obvious gestures
(pan and zoom commands reachable through keyboard, wheel and drag) and
splits recent files into a submenu nobody needs. Time and frequency
fit/zoom commands are named and grouped inconsistently. This change
rewrites the menu tree, inlines recent files into File, adds a Settings
entry, moves fit commands to symmetric shortcuts, and puts translucent
zoom button pairs on the spectrogram's corners so pointer users get the
same discoverable zoom the keyboard has.

Issue class: **A** — the plan first declared **B**, which was a
misjudgment: the class-A thresholds (roughly 400 lines or five code
files) were already foreseeable, and `AGENTS.md` lets any class-A signal
decide. Reclassified to **A** once the diff crossed them. Process notes,
recorded here and in the Pull Request: the class-A implementer model
(`gpt-6-astra` xhigh) was not used because the owner directed Claude to
implement this issue in session. The review model also deviated: the
first review loops ran on `gpt-5.6-terra` high because the implementer
worked from a stale `AGENTS.md` text and dismissed the reviewer's
correction; per the class table the class-A reviewer is
`gpt-5.6-sol` xhigh, which runs the closing round.

## Context

- Menu contents live in `Shell::application_items` /
  `application_view_items` in `crates/app/src/app_menu_ui.rs`; menu rows
  resolve shortcut keycaps from registered bindings
  (`Kbd::binding_for_action_in`), so menu hints follow `bind_keys`
  automatically.
- Recent rows already come availability-filtered from
  `Shell::recent_entries` (`recent.rs` snapshot, refreshed on menu open);
  `RECENT_LIMIT` is 10. Recent rows open files by `Origin`, so they show
  no keycaps (Alt+1..9 work only on the start page).
- The plot keystroke interceptor in `navigation_ui.rs`
  (`plot_shortcut`) exists because GPUI consumes Shift for symbol keys;
  any new symbolic binding needs an interceptor arm and tests.
- The pairs sit on the spectrogram itself: the time pair in its
  bottom-left corner, the frequency pair in its top-right one,
  identically in both orientations, held clear of the picture's edges.
  An earlier revision placed them on the rulers with a reserved
  axis-frame zone; that reservation was reverted when the owner moved
  the pairs onto the picture.
- Axis layout stays toolkit-neutral (`axes::Frame`, tested in
  `axes_tests.rs` with the fixture font); the corner zones are computed
  from the measured plot rectangle in `plot_ui.rs` and tested there.
- The settings window opens through the `EditAnalysis` action
  (`ctrl-,` / `cmd-,`), shared by the status bar control.

## Decisions

Owner decisions from 2026-09-17:

- Final menu layout:

  ```
  Argand
  |- File
  |   |- Open file...                 Ctrl+O
  |   |---
  |   |- <recent files, inline>       (only when non-empty; availability-filtered, max 10)
  |   |---
  |   `- Settings                     Ctrl+,
      `- View                             (only with an open file)
          |- Show grid                    Ctrl+G  (checked)
          |- Show scale controls          Ctrl+U  (checked)
          |- Vertical orientation         Ctrl+T  (checked)
      |---
      |- Fit time                     Ctrl+0
      |- Fit frequency                Ctrl+Shift+0
      |---
      `- Time scale format >          (hms / Seconds / Samples, unchanged)
  ```

- Recent files become flat rows inside File. When the recent list is
  empty, both recent separators collapse; the separator between Open and
  Settings remains.
- `Settings` dispatches `EditAnalysis` (same action as Ctrl+, and the
  status bar).
- View drops all navigation entries: arrow pans (plain and Ctrl),
  Home/End, and menu zoom in/out. Keyboard, wheel and drag operations
  all remain unchanged.
- Labels kept as-is: `Open file...`, `Show grid`, `Vertical
  orientation`, `Time scale format`. New labels: `Settings`, `Fit time`,
  `Fit frequency`.
- `FitFrequency` rebinds from `Ctrl+Shift+Home` to `Ctrl+Shift+0`
  (top-row and keypad `0`), no alias, symmetric with `Ctrl+0` = fit time
  and the Shift-selects-frequency convention of the zoom keys. No other
  binding, gesture or action changes.
- Small +/- button pairs sit on each ruler at the end without the unit
  caption (horizontal: time pair at the left end of the time ruler,
  frequency pair at the bottom end of the frequency ruler; vertical:
  pairs follow their rulers symmetrically). The axis frame reserves the
  button zone so ticks and labels never collide. Buttons are styled like
  toolbar buttons (theme-aware, border, hover), about 20 px, with
  tooltips from `shortcut_tooltip`, and dispatch the existing
  `ZoomIn`/`ZoomOut`/`FrequencyZoomIn`/`FrequencyZoomOut` actions (zoom
  about view centre) so buttons, keys and hints cannot diverge. Buttons
  are disabled without an open file.
  **Superseded during implementation**: the owner rejected the ruler
  placement; the pairs moved onto the spectrogram's corners with a
  Show-scale-controls toggle — see the revision note under
  Implementation steps.

## Rejected alternatives

- Keeping pan/zoom entries in View with consolidated grouping: the
  keyboard, wheel and drag paths already cover them, and the issue asks
  for a concise menu; the corner scale controls restore pointer
  discoverability for zoom.
- `Ctrl+Shift+Home` alias alongside `Ctrl+Shift+0`: an alias would
  reintroduce the inconsistency the issue complains about and complicate
  menu keycaps.
- Zoom buttons implementing custom zoom logic: they must dispatch the
  registered actions so hints and behaviour agree (acceptance
  criterion).
- A `Recent` submenu: one extra level for at most ten entries the owner
  opens directly.

## Implementation steps

- [x] Restructure `application_items`/`application_view_items` in
      `crates/app/src/app_menu_ui.rs` to the final layout; inline recent
      rows with adaptive separators; add `Settings`; drop pan/zoom
      entries.
- [x] Rebind `FitFrequency` to `ctrl-shift-0` in
      `crates/app/src/navigation_ui.rs` (binding + interceptor +
      `plot_shortcut` tests); rename menu labels.
- [x] Reserve ruler-end button zones in the axis frame layout
      (`crates/app/src/axes.rs`, toolkit-neutral, tested without GPU).
      The reservation was later reverted together with the ruler
      placement.
- [x] Render the two zoom pairs (new small module or `plot_ui.rs`),
      reusing the shared button styling and tooltip helper; disabled
      without a file; both orientations.
      Revised on owner feedback: translucent `[+|-]` pairs with rounded
      corners sit on the spectrogram itself — time in the bottom-left
      corner, frequency in the top-right one, in both orientations, eight
      logical pixels clear of the picture's edges; View → Show scale
      controls (Ctrl+U, session version 10) hides or shows them; the
      ruler-end reservation was reverted. Zoom shortcuts are documented
      uniformly as Ctrl+Plus/Minus and Ctrl+Shift+Plus/Minus (Ctrl+= is the
      same key as Ctrl+Plus).
- [x] Update README (shortcut table, View menu references, Settings
      entry, Ctrl+Shift+0 IME note), AGENTS.md "Current status" menu
      description, `CHANGELOG.md` `[Unreleased]`.
- [x] Tests: menu structure without toolkit types (extend `app_menu.rs`
      tests where applicable), interceptor tests for `ctrl-shift-0`,
      corner-zone geometry and keycap-naming tests.
- [x] Full gate: `cargo fmt --all -- --check`; `cargo clippy
      --all-targets --locked`; `cargo test --locked`; `cargo build
      --release --locked` last.
- [ ] GPU screenshots of both orientations, both themes, with/without
      recent files, empty recent; present to owner for placement
      sign-off.
- [ ] External review round(s) via `codex` CLI per CONTRIBUTING (owner
      said work alone; confirm before this step whether to run it or
      waive).
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Menu-model tests, interceptor tests, corner-zone geometry tests.
- [x] Manual GPU checks: menus, corner scale controls, both orientations
      and themes, recent-empty/recent-present, Settings entry, shortcut
      hints match actual behaviour.

## Post-completion

- Owner reviews corner scale control placement on screenshots before the
  Pull Request goes Ready; placement tweaks stay in this branch.
- Windows check that `Ctrl+Shift+0` reaches the application on common
  IME setups is part of downstream platform validation.

## Risks / notes

- `Ctrl+Shift+0` may be swallowed by IME/language switching on some
  Windows setups; if it does not reach the app, frequency fit stays
  reachable through the menu and `Fit time` remains on `Ctrl+0`. Note in
  README.
- Exact button offsets are decided on screenshots; the owner reviews
  them before the Pull Request goes Ready.
- Label `Open file...` is retained (owner sketch wrote `Open`, but the
  confirmed decision was to keep the current precise labels).
