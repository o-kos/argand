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

## Owner feedback: configuration-only numeric locale override

- [x] Add measured hover hints for time/frequency unit captions, preserving ruler
  navigation and the time context menu; validate native interaction and review.
- [x] Move the frequency unit beside the minimap, remove the four-pixel spectral
  top gap, and show hand cursors only where panning is available.
- [x] Use "Time in samples" and "Frequency in kHz" (with analogous frequency
  units), with localized resolution per physical screen pixel that updates live.

Ruler follow-up validation: formatting, strict Clippy and all 516 tests pass;
the release was rebuilt after the gate. Native Linux checks on a real Intel GPU
confirm the gap is gone, the frequency caption sits beside the minimap, all three
time hints show localized resolution, and right-clicking the time unit still
opens the format menu. With a stationary pointer, keyboard zoom updates the
sample resolution from 291,7 to 145,8 to 72,92 samples/px. Format switching and
hovering do not trigger analysis. Full-capture rulers retain arrow cursors;
the time ruler becomes draggable after zoom, while frequency navigation awaits #80.

Review caught stale popup weak references disabling hints after File/View menu
dismissal; only live popup entities now suppress hints. Native checks confirm
hints return after dismissal. The follow-up review found older documentation
still promising hands on both rulers; those descriptions now match the actual
pan availability. Both findings were accepted, with none rejected or deferred.

- [x] Default to `theme = "system"`, follow OS appearance changes while running,
  preserve explicit light/dark overrides, and update the template and owner config.
- [x] Validate theme configuration and appearance handling, rebuild release, and
  complete independent review of the theme-default follow-up.

Theme-default validation: 513 tests, formatting and strict Clippy pass, followed
by a fresh release build. Native Linux checks on an Intel GPU use an isolated
Settings portal to exercise light/dark changes without changing desktop settings.
The default follows light/dark/light transitions, including the open settings
editor; explicit light and dark overrides stay fixed, and an explicit system
configuration starts dark when the portal already reports dark. The owner's
configuration now uses `theme = "system"` while retaining `number_format = "ru-RU"`.
Independent review of the theme-default change found no substantive issues;
no findings were rejected or deferred.

- [x] Supply the complete commented default configuration as an installation
  asset, document its location and check it against the actual application defaults.
- [x] Create the owner's user configuration with Russian numeric formatting;
  track tag-driven OS distributions and shipping the template in a linked issue.
- [x] Validate and review the configuration-template follow-up.

The canonical template is `crates/app/assets/argand.toml`; it includes every
current key, including the documented legacy panel value. Independent exact
key-set and parser/default agreement tests catch missing keys and invalid or
changed defaults. The full gate passes all 511 tests and the release build is current.
`cargo package --list` includes `assets/argand.toml`. The owner's configuration
uses the same settings with only `number_format` changed to `ru-RU`.

Issue [#86](https://github.com/o-kos/argand/issues/86) tracks tag-triggered
Linux/Windows/macOS distributions and package-manager delivery, including this
template and preservation of user configuration. It coordinates with #40;
the current release workflow still ships aspec archives only. Distribution
templates must not shadow user configuration through the portable-first search
order. No release or package availability is claimed by this template change.

Review identified that deserialization fills omitted keys with defaults, leaving
the initial equality test unable to detect an incomplete template. Accepted:
an independent exact key-set test now covers the root and each section. The
follow-up review found no substantive issues; no findings were rejected or deferred.

- [x] Add top-level `number_format` to `argand.toml`, defaulting to `system`,
  with explicit BCP 47 locale tags and C/POSIX supported without an editor control.
- [x] Initialize numeric presentation after configuration loading; keep system
  locale precedence and canonical configuration/session serialization unchanged.
- [x] Repair invalid locale strings independently, document the option, and
  verify overrides under a C numeric environment, including native UI and input.
- [x] Run the full local gate, rebuild release, and obtain a clean external review.

Validation: all 509 tests, strict Clippy and formatting passed, followed by a
fresh release build. Native Linux checks on a real Intel GPU used
`LANG=ru_RU.UTF-8`, `LC_NUMERIC=C.UTF-8` and no `LC_ALL`. With no override,
file hints and sample labels retain C formatting. With `number_format="ru-RU"`,
sample labels and file hints gain grouping, fractional time/frequency labels use
commas, and the settings editor accepts 45,5 dB and displays it in the analysis
hint. The independent review of this iteration found no substantive issues;
there were no rejected or deferred findings.

## Owner feedback: scale presentation and numeric locale

- [x] Rename the format submenu and HMS entry; share it with a time-ruler context menu.
- [x] Show s/# once at the right of the time ruler and reserve caption space.
- [x] Format all GUI numeric output with the numeric locale, including axes,
  coordinate badges, status/file/analysis hints and settings; parse settings in
  the same locale without changing configuration/session serialization.
- [x] Measure localized axis labels before selecting ticks and cover decimal,
  grouping, integer precision, settings input and menu behavior in validation.
- [x] Complete local gate, current release, native checks and external review.

Numeric presentation defaults to system regional/numeric settings. On
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

## Owner feedback: stable Alt coordinate badges

- [x] Measure fixed time/frequency badge widths for the current ruler extents,
  format and pixel precision, then center each cursor readout inside its badge.
- [x] Extend guide lines to the rounded badge backgrounds without a ruler gap.
- [x] Validate width bounds, native motion/edge behavior, the full gate, a fresh
  release build and independent review.

Validation: formatting, strict Clippy and all 519 local tests pass, followed by a
fresh release build. Range sweeps cover all three time formats, clock/decimal
rounding boundaries, grouping, large integer sample indices, signed frequencies
and display scales 1, 1.25 and 2. Native Linux checks on a real Intel GPU with
Russian numeric formatting cover real and I/Q files, all three formats, short
and long readouts, positive/negative frequencies, and panel edges. Screenshots
confirm stable badge widths, centered text and the time line reaching the badge
across the ruler padding. Mode changes and pointer motion leave analysis counts
unchanged.

Review identified repeated endpoint measurement on every canvas redraw, including
without Alt. Accepted: the shell now retains a shared lazy cache keyed by the
ruler extents, plot geometry, display scale and font, accessed only while Alt
guides are visible. Numeric locale is immutable after startup. A counting-measure
test covers reuse across clones and invalidation of each changing input. The
follow-up review found no substantive remaining issues; no findings were rejected
or deferred.

The rebuilt cache implementation also passed native mode/motion checks. Zoom
changes second-readout precision and the reserved width; subsequent pointer
motion retains that new width.
