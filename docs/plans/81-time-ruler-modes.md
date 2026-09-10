# Issue #81: Time ruler display modes

Resolves #81: https://github.com/o-kos/argand/issues/81.

## Overview

Offer the existing clock format, elapsed seconds and zero-based sample numbers
on the time ruler. Keep the capture range unchanged when switching modes and use
the selected format for the Alt time badge. Complex samples count I/Q pairs.

## Context

Shared tick layout lives in `argand-core::axis`; the GUI measures it in `axes.rs`.
Time navigation owns integer sample ranges and retains a tick scheme during pan.
The minimap from #79 remains full-capture and independent of ruler formatting.
The current PR is prepared after owner acceptance of #83 while its full CI runs;
retarget and rebase onto main after that PR's squash merge.

## Decisions

- Add decimal-seconds and integer-sample axis kinds to the shared measured layout.
  Keep existing clock formatting and CLI defaults unchanged.
- Keep presentation policy in a toolkit-neutral application module. Axis extents
  and held schemes use the selected ruler units; keyboard divisions convert back
  to samples exactly once. Mode changes reset only the held ruler scheme.
- Offer a checked Time ruler submenu in View. Default to the current clock mode;
  persist the selected presentation in a versioned session, with older sessions
  defaulting to clock. File openings still reset the time view.
- Alt time badges follow the selected mode and pointer precision; sample readouts
  use integer capture indices, including complex I/Q pairs.
- Switching formats performs no decoding or FFT work and preserves the viewport,
  minimap content, spectral settings and frequency labels.

## Rejected alternatives

- Reusing formatted seconds as sample numbers would lose sample origin and I/Q
  pair semantics. Sample coordinates come from the capture range and true rate.
- Replacing existing clock formatting would change the default promised by #81.

## Implementation steps

- [ ] Add shared axis layouts and toolkit-neutral ruler formatting with regression tests.
- [ ] Connect ruler modes, held tick navigation and Alt readouts to the GUI.
- [ ] Add the checked View submenu and backward-compatible session persistence.
- [ ] Update README, changelog and architectural invariants.
- [ ] Complete native checks, local gate, fresh release build and external review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] Default formatting, fractional seconds, integer sample ticks, capture offsets,
  I/Q pair indices, readable labels and stable held grids at different zoom levels.
- [ ] Mode switching preserves the view and avoids analysis requests; session
  round-trip and old-session defaults preserve existing navigation reset rules.
- [ ] Native real-GPU checks for the menu, three ruler modes, Alt badges, pan/zoom,
  file replacement and application restart, using real and I/Q captures.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after the gate passes

## Post-completion

Continue with #80, then the remaining grid/ruler backlog, and #82. Continuous
minimap drag latency remains in the owner-approved backlog issue #84.
