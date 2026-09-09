# Status and settings interaction revision

The cancellation and file-specific range follow-up is recorded in
[the next interaction report](32-settings-cancel.md). The screenshots below show
the earlier editor revision.

Native validation on 2026-09-09 for the owner feedback on Draft PR #64 (#32).
This supersedes the UI descriptions in the [first validation report](32-status-settings.md).
The earlier worker timing measurements are retained there; this revision does not
claim those measurements as timings for the new settings window.

## Implementation

File details use aligned label/value rows, grouped counts, readable sizes with exact
bytes, and separate I/Q extrema in decoded sample units. The analysis summary is
muted normally and uses a brighter foreground and background on hover. Its hover
hint shows the effective settings without a click and offers editing and range advice.

The separate 440 × 520 settings window uses standard selects and numeric inputs,
with Transform and Display sections. It owns one toolkit Root for input handling;
the main Shell retains its existing frame. The recommendation and error stay in the
footer outside the form scroller. Applying the recommendation focuses the numeric
range field, which replaces the noneditable readout.

Yellow specifically marks a nonzero signal whose spectral peak is in the lower half
of the absolute full-scale colour window. A narrower recommended range alone does
not trigger it. Fixed/automatic modes, silence, failed and superseded results do not
warn. The recommended value is still the measured aspec value. The warning decision
is cached on analysis delivery rather than scanning the grid on each UI redraw.

## Checks

- Formatting, strict Clippy and all 441 tests passed; release rebuilt afterward.
- Native Linux checks used a separate Sway output, GLES2 and the real Intel GPU,
  with isolated configuration/state/cache directories. No owner preferences changed.
- Dark and light themes, 1000 × 700 and 640 × 400 main windows were inspected.
  Both hints fit, including complex FLAC metadata with two extrema rows.
- Ctrl+, opens the form and focuses FFT. Tab and Shift+Tab traverse controls.
  Enter and arrows select FFT/window/aggregation/palette/mode; typed numbers commit
  on Enter or blur, and arrows step them. Escape first dismisses a dropdown, then
  closes the form. Reopening restores the controls; closing the main window closes
  the settings window too.
- Keyboard edits covered FFT 2048 → 1024, Hann → Hamming, overlap 75 → 50,
  Peak → Mean power, palette changes and all range modes. Invalid range 0 kept the
  previous effective range and reported an error; correcting the value committed it.
- A weak float WAV fixture triggered the 110 dB warning. Keyboard traversal reached
  the footer recommendation; Enter applied 30 dB and focused the range input, then
  Up changed it to 31 dB. Restart preserved that value. An ordinary complex FLAC
  remained at 110 dB without a yellow warning.
- Choosing an FFT larger than the weak fixture was rejected. The select returned
  to 2048, the footer explained the error, and the existing picture remained.

Native Windows/macOS interaction was not exercised; the Ready PR matrix remains
required before merge.

## Independent review

The review accepted one P2 finding: a rejected dropdown choice could remain visible
while effective settings retained the old value. The editor now restores all selects
after rejection. No findings were declined in this UI revision. Subsequent review
rounds checked window creation, keyboard focus, recommendation handling and hover;
the final round had no substantive findings.

## Current screenshots

- [File details, light](32-ui-data/file-light.png)
- [File details, dark](32-ui-data/file-dark.png)
- [File details in a narrow window](32-ui-data/file-narrow.png)
- [Analysis hover hint](32-ui-data/analysis-hover.png)
- [Standard settings controls, light](32-ui-data/settings-light.png)
- [Recommendation reached by keyboard](32-ui-data/recommendation.png)
- [Range stepped after applying the recommendation](32-ui-data/settings-dark.png)
- [Rejected FFT selection restored](32-ui-data/invalid-fft.png)
