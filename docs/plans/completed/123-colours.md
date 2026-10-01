# Issues #123, #151, #140, #142: Interface colours and the theme selector

Resolves #123, #151, #140 and #142.

## Overview

The owner asked for every colour issue in one Pull Request:

- **#123** Hints and popup menus blend into panels in the dark theme. The stepper
  buttons of the analysis settings hint (`−` / `+`) are part of it (comment from #108).
- **#151** In the dark theme the window does not stand apart from adjacent dark windows
  around the status bar.
- **#140** The title bar does not show whether the window is active, and the Linux
  minimize and maximize icons do not brighten on hover or dim while pressed.
- **#142** A persisted System, Light and Dark selector in the interface.

The spectrogram palette, analysis and every geometry stay as they are.

Class A, because the expected diff exceeds five code files and changes the session
layout. Implementer: Claude in session. Reviewer: Codex `gpt-6.1-sol` high, which the
owner also made the default reviewer for class A.

## Context

- The toolkit's theme is `gpui_kit::component::theme::Theme`. `Theme::change(mode, ..)`
  applies the `light_theme` or `dark_theme` `ThemeConfig` held in the global theme on
  every switch, so colours written into the live theme are lost on the next system
  appearance change. A `ThemeConfig` carries optional colour overrides
  (`popover.background`, `border`, `title_bar.background`, `status_bar.background`,
  `window.border` and others) that `apply_config` resolves.
- Default dark values: the background and the popover are both `neutral-950`, so a hint
  and a menu have the colour of the plot and panels beneath them. The border is
  `neutral-800`, the status bar (`secondary`) `neutral-800`, the window border
  `#262626`, the title bar `#171717`.
- `shell::sync_theme` and `run` call `Theme::change`. `Shell::new` already subscribes to
  window activation and appearance.
- Hints are toolkit `Tooltip`s and our `Tooltip::element`s; the settings hint draws its
  own surface from `tokens.popover`; menus use the toolkit's `popover_style`.
- The Linux caption controls are `chrome::controls`, plain `div`s with hover and active
  backgrounds and one foreground.
- `config::Theme` (`system`, `dark`, `light`) is read from `argand.toml`. The session is
  at version 10.
- The View branch of the application menu is absent without a document (#99).

## Decisions

- **One application theme module.** A new `theme.rs` derives Argand's light and dark
  `ThemeConfig`s from the toolkit defaults with the overrides this PR needs, and installs
  them into the global theme once at start-up, before the first `Theme::change`. Every
  later switch then applies them by itself. No call site sets colours of its own for a
  surface the theme can carry.
- **Surfaces (#123).** Hints and menus get a popover background and border that stand
  out from the background, panels and status bar in the dark theme. The light theme
  keeps its white popover with a border that stays visible. The settings hint and the
  range balloon read the same tokens, so they follow. The stepper buttons get resting,
  hover and pressed colours that read against that surface. Exact values are chosen with
  the owner on screenshots.
- **Window edge (#151).** In the dark theme the window border and the status bar get
  enough contrast against a neighbouring dark window. Values chosen with the owner.
- **Title bar (#140).** An inactive window paints the title bar's background, border
  and foreground in a muted palette, decided at render time from
  `Window::is_window_active()`, which the existing activation subscription already
  repaints. Linux minimize and maximize icons brighten on hover and dim while pressed;
  close keeps its danger colours with distinct normal, hover and pressed states. Windows
  and macOS controls are not restyled.
- **Theme selector (#142).** Owner's choice: View is shown with and without a document,
  and carries a `Theme` submenu with System, Light and Dark as checked rows bound to
  three actions. Without a document the rows that need one (Fit time, Fit frequency) are
  disabled, while the session preferences (grid, scale controls, orientation, time scale
  format) stay available, since they are remembered for the next document.
- **Persistence.** Session version 11 adds an optional theme. A session without one
  (fresh, or versions 1 to 10) takes the theme from `argand.toml`, whose default is
  System. A choice in the menu is written to the session, never to `argand.toml`. The
  future-version write protection stays as it is.
- Changing the theme updates every window, keeps the spectrogram palette and requests no
  analysis.

## Owner review of the colours (2026-10-01)

- The first proposal was rejected: buttons kept one ink for every state, Reset to
  defaults barely showed, the settings hint, menus, the status bar and hints shared one
  colour, and the hot row of a menu was hard to see.
- The owner asked for a gallery before any further code. Every theme in the toolkit's
  repository (21 sets, 40 variants), the toolkit default and an Adwaita palette were
  captured with every surface and state. No theme was free of the problems, and the
  owner chose to keep the current theme and fix what the review found.
- Fixed: the minimap stands on the palette's darkest colour in both themes; the
  steppers and Reset to defaults change their ink under the pointer and while held,
  Reset has a frame; surfaces step up from window to bars, settings sheet and popovers;
  the hot row uses a raised accent.
- Left to a separate issue: the toolkit paints the list's keyboard row and the row under
  the pointer alike, and only a custom row rendering, which needs the owner's agreement,
  could tell them apart.

## Rejected alternatives

- Writing colours into the live theme after each `Theme::change`: every switch path
  would have to remember to repeat it.
- Painting surfaces at each hint and menu call site: #123 asks for one shared treatment.
- A theme row in File or a separate top-level branch: the owner chose View.
- Writing the chosen theme back to `argand.toml`: the configuration belongs to the user.

## Implementation steps

- [x] Agree the class A default reviewer in `AGENTS.md`.
- [x] `theme.rs` with Argand's light and dark configurations, installed at start-up.
- [x] Hint, menu and settings-hint surfaces and the stepper colours (#123).
- [x] Window border and status bar in the dark theme (#151).
- [x] Title bar active and inactive palettes, Linux caption icon states (#140).
- [x] Session version 11 with the theme preference, migration and defaults (#142).
- [x] View shown without a document, its Theme submenu, actions and disabled rows (#142).
- [x] Tests: defaults, all three selections, persistence and migration, configuration
      compatibility, separation from the spectral palette and from analysis, View without
      a document.
- [x] Screenshots of both themes, active and inactive, hover and pressed, hints and menus
      over rulers, plot, panels and status bar, checked with the owner.
- [x] Update `AGENTS.md`, `README.md`, `CHANGELOG.md`, `crates/app/assets/argand.toml`
      comments if the theme key's meaning changes, and `docs/ui/124-inventory.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Native evidence on Linux in both themes; Windows and macOS follow the owner's rule
      that platform problems become separate issues.

## Post-completion

- Close #123, #151, #140 and #142 through the merge.

## Review round 1

Codex `gpt-6.1-sol` high. Two findings, both accepted.

- **Accepted: the inactive title bar was muted on Linux only.** Windows and macOS draw the
  stock `TitleBar`, which ignores window activation. It now takes the same palette for its
  background, border and text, keeping the native caption controls.
- **Accepted: no test covered the installed theme.** The menu test ran without
  `theme::install`. A test now installs Argand's themes and checks the popover, status bar
  and sheet colours after Light, Dark and Light switches, and the menu tests run with the
  installed theme and check the status bar colour of each choice.

## Review round 2

Codex `gpt-6.1-sol` high confirmed both fixes, including that the theme and menu tests
fail when the configurations are not installed or not applied on a switch, and found
nothing else. The round is clean.
