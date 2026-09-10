# Issue #81: Time ruler display modes

Resolves #81: https://github.com/o-kos/argand/issues/81.

## Overview

Offer HMS clock time, elapsed seconds and zero-based sample numbers
on the time ruler. Keep the capture range unchanged when switching modes and use
the selected format for the Alt time badge. Complex samples count I/Q pairs.

## Context

Shared tick layout lives in `argand-core::axis`; the GUI measures it in `axes.rs`.
Time navigation owns integer sample ranges and retains a tick scheme during pan.
The minimap from #79 remains full-capture and independent of ruler formatting.
The branch includes the merged minimap implementation from #83.

## Decisions

- Add decimal-seconds and integer-sample axis kinds to the shared measured layout.
  Keep shared CLI clock formatting and CLI defaults unchanged. GUI HMS uses colons
  between fields and the numeric locale for fractional seconds.
- Keep presentation policy in a toolkit-neutral application module. Axis extents
  and held schemes use the selected ruler units; keyboard divisions convert back
  to samples exactly once. Mode changes reset only the held ruler scheme.
- Offer a checked Time scale format submenu in View and the same context menu
  on the time ruler. Default to HMS;
  persist the selected presentation in a versioned session, with older sessions
  defaulting to clock. File openings still reset the time view.
- Alt time badges follow the selected mode and pointer precision; sample readouts
  use integer capture indices, including complex I/Q pairs.
- Switching formats performs no decoding or FFT work and preserves the viewport,
  minimap content, spectral settings and frequency labels.

## Rejected alternatives

- Reusing formatted seconds as sample numbers would lose sample origin and I/Q
  pair semantics. Sample coordinates come from the capture range and true rate.
- Changing CLI clock formatting is outside this GUI presentation change.

## Implementation steps

- [x] Add shared axis layouts and toolkit-neutral ruler formatting with regression tests.
- [x] Connect ruler modes, held tick navigation and Alt readouts to the GUI.
- [x] Add the checked View submenu and backward-compatible session persistence.
- [x] Update README, changelog and architectural invariants.
- [x] Complete native checks, local gate, fresh release build and external review.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] Default formatting, fractional seconds, integer sample ticks, capture offsets,
  I/Q pair indices, readable labels and stable held grids at different zoom levels.
- [x] Mode switching preserves the view and avoids analysis requests; session
  round-trip and old-session defaults preserve existing navigation reset rules.
- [x] Native real-GPU checks for the menu, three ruler modes, Alt badges, pan/zoom,
  file replacement and application restart, using real and I/Q captures.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after the gate passes

## Results

- Formatting, strict Clippy and all 505 local tests pass; the release was rebuilt
  after the full gate. No lint policy was relaxed.
- Native Linux/Sway checks on a real Intel GPU cover all three checked menu modes,
  Alt badges, unchanged sample ranges and analysis counters on mode switching,
  one/five keyboard divisions, real and I/Q captures, same-process file replacement
  and persistence after restart. File replacement restores the full view while
  retaining Samples mode. The I/Q fixture reports 94080 sample pairs; its badge
  matches the expected integer capture index.
- Review found fractional clock badges losing the hours field on hour-wide spans.
  The formatter now retains hours and carries rounding across the hour boundary;
  regression tests cover both behavior and the zoomed minutes-only format. The
  corrected two-hour Alt readout also passed a native check. The final external
  review returned no substantive findings; no findings were rejected.

## Post-completion

Continue with #80, then the remaining grid/ruler backlog, and #82. Continuous
minimap drag latency remains in the owner-approved backlog issue #84.

## Owner feedback: scale presentation and numeric locale

- [x] Rename the format submenu and HMS entry; share it with a time-ruler context menu.
- [x] Show s/# once at the right of the time ruler and reserve caption space.
- [x] Format all GUI numeric output with the numeric locale, including axes,
  coordinate badges, status/file/analysis hints and settings; parse settings in
  the same locale without changing configuration/session serialization.
- [x] Measure localized axis labels before selecting ticks and cover decimal,
  grouping, integer precision, settings input and menu behavior in validation.
- [x] Complete local gate, current release, native checks and external review.

Numeric presentation is initialized from system regional/numeric settings. On
Linux the LC_ALL/LC_NUMERIC/LANG precedence applies; machine-readable data and CLI
formats retain their existing contracts. The decimal formatter uses CLDR data;
locale-specific grouping and digit shapes are not maintained as a hand-written
language table. Time punctuation retains its clock structure.

The locale review identified the default range text before analysis bypassing the
formatter; it now uses the same numeric path. Native checks identified retained
context-menu entities suppressing Alt guides after dismissal, and cleared hover
state leaving a stationary pointer without guides after an outside click. A
window-aware dismissal subscription clears only the matching menu and restores
the pointer through the ordinary plot geometry filter. The final external review
was clean; no findings were rejected or deferred.

Final native checks on the rebuilt release verify all three formats in both menus,
localized fractional time/frequency badges, a single right-hand ruler unit,
unchanged viewport/analysis counters, and restored Alt lines after selection,
Escape and an outside click without further pointer motion. Russian settings
accept 45,5 dB and grouped FFT choices; hints show the same values, and the
canonical saved FFT remains 4096. Arabic startup output includes localized
default range digits. Restart preserves Samples mode while restoring full view.
