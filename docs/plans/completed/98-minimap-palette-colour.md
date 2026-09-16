# Issue #98: Match the minimap waveform colour to the spectrogram colour scheme

Resolves #98.

Complexity class **B**: implementer `gpt-6-astra` at high reasoning effort,
reviewer `gpt-5.6-sol` at high reasoning effort, per "Agent roles and model
selection" in `AGENTS.md`.

## Overview

`waveform.rs` paints the full-capture minimap with two literal colours, `0x78c8ff`
inside the requested time viewport and `0x243c4d` outside it. Both were chosen for
the Oceanic ramp and never change, so selecting Inferno or Viridis leaves a blue
waveform above an orange or green spectrogram.

Derive both colours from the displayed colormap and the interface theme instead.
Nothing about the minimap's content, geometry, caching or navigation changes: this
is ink only.

Out of scope: the CLI's fixed `Theme::TRACE`, the spectral image itself, the
interface theme palette, and the spectral dynamic range.

## Context

- `crates/app/src/waveform.rs` holds both literals in `Waveform::paint`, inside the
  per-column loop. Its `Spans` cache stores geometry only -- columns, rows and
  min/max pairs -- so ink can change without touching the cache.
- `crates/app/src/plot_ui.rs::minimap_panel` builds `waveform::Panel` on every
  render and already reads `cx.theme().border` for the separator, so it is the
  place that already knows both the theme and the shell.
- `crates/app/src/shell.rs` keeps `settings` (requested, including an editor
  preview) and `file.displayed_settings` (the settings the drawn picture was
  produced with, set at line 558 when a result is accepted).
- `crates/core/src/colormap.rs` owns `Colormap`, its stops and its HSL gradient
  baking. It already depends on the `hsl` crate.
- `gpui_component::ThemeMode::is_dark` reports the resolved interface theme;
  `shell.rs` already re-renders on window appearance changes.

## Decisions

- **A signature colour per ramp, with lightness derived.** Each `Colormap` names one
  recognisable colour; hue and saturation come from it, lightness is computed for the
  interface theme and for inside/outside the viewport. Sampling the gradient instead
  was measured and rejected below. Six constants behind an exhaustive `match` cannot
  fall behind a new ramp, because the match stops compiling.
- **Signatures:** Oceanic `0x4DA4D5`, Grayscale `0x9AA3AD`, Inferno `0xF98E09`,
  Viridis `0x5EC962`, Synthwave `0xA537FD`, Sunset `0xE03A22`. Grayscale carries a
  slight blue cast rather than a pure grey so the muted span stays distinguishable
  from the panel background at low lightness. Sunset takes its red rather than its
  orange so it does not collide with Inferno.
- **Saturation is clamped to 0.85** before lightness is applied, so a fully saturated
  ramp cannot produce a vibrating edge against the panel.
- **Lightness constants:** dark theme, active `0.68` and muted `0.26` at half
  saturation; light theme, active `0.36` and muted `0.76` at half saturation. Derived
  Oceanic ink is `#7BBBE0` / `#2E4857` against today's `#78C8FF` / `#243C4D`, so the
  dark theme keeps the look it has now.
- **The colour follows `displayed_settings`, not the requested settings.** The two
  panels then change together and the mismatch the Issue reports cannot appear in a
  transient form of its own while a preview is being computed. Before any picture
  exists -- the minimap preview can land first -- fall back to the requested
  colormap, where no mismatch is possible because there is no spectrogram yet.
- **The derivation lives in `argand-core`.** It is a property of the ramp and names
  no toolkit type; `plot_ui.rs` converts the returned RGB into `gpui::rgb`. This is a
  pure addition to the public API, not a change to an existing contract.

## Rejected alternatives

- **Sampling the baked gradient** (at a fixed position, or at its most chromatic
  mid-lightness point). Measured over all six ramps: Inferno, Sunset and Viridis
  converge on one yellow-orange (`#F3E868`, `#F3D368`, `#F3E568` at the chromatic
  peak), and Synthwave loses its magenta to blue. The rule produces a compatible
  colour for any future ramp automatically, but the minimap stops saying which ramp
  is selected, which is the point of the Issue.
- **A full manual table** of 6 ramps x 2 themes x active/muted. Maximum control over
  every pixel, but 24 values to keep coherent, and readability against the panel
  background would rest on someone having checked it rather than on the construction.
- **Changing the colour immediately on selection**, before the recoloured spectrogram
  arrives. Faster feedback, but it creates exactly the mismatch #98 reports, only
  briefly.
- **Giving the CLI the same ink.** `crates/cli/src/render.rs` paints its waveform with
  a fixed `Theme::TRACE` inside its own dark theme, and `aspec` output is expected to
  stay byte-identical. It is consistent to do, but it is not this Issue and it would
  change every rendered capture.

## Implementation steps

- [x] Add `Colormap::waveform_ink` to `crates/core/src/colormap.rs`, returning active
      and muted RGB for a given interface theme, with the signature colours and
      lightness constants above as named items.
- [x] Cover it in `crates/core/src/colormap_tests.rs`: every ramp answers for both
      themes; active and muted differ from each other in lightness by a stated
      minimum in both themes; the derived Oceanic dark ink stays within a stated
      distance of the colour it replaces.
- [x] Replace the two literals in `crates/app/src/waveform.rs` with ink carried on
      `waveform::Panel`, leaving the `Spans` cache and its invalidation untouched.
- [x] Resolve the ink in `crates/app/src/plot_ui.rs::minimap_panel` from the
      displayed colormap, falling back to the requested one when no picture has been
      displayed yet, and from `cx.theme().mode.is_dark()`.
- [x] Add a toolkit-free test that the resolved colormap follows the displayed
      settings while a preview with a different ramp is pending, and follows the
      requested settings before any picture exists.
- [x] Update the "Full-capture waveform minimap (#79)" section of `AGENTS.md` to
      state that minimap ink derives from the displayed ramp and the interface theme.
- [x] ➕ Add the required user-visible fix entry to `CHANGELOG.md`.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [ ] Owner validation on a real GPU session, in both interface themes: open a
      capture, step through all six ramps in the settings editor, confirm the minimap
      follows the spectrogram rather than leading it, confirm Cancel restores both,
      and confirm the dimmed outside-viewport span stays visible after zooming in.

Local validation passed with 560 tests, zero failures and zero ignored tests.
Cargo reported a future-incompatibility notice for the unchanged dependency
`proc-macro-error2 v2.0.1`. Owner GPU validation remains pending, so the overall
validation task stays open. The owner requested that the plan remain here until
the later review stage.

## Post-completion

- Decide whether `aspec` should take the same ink for its waveform strip, which would
  change every rendered capture and needs its own Issue.
