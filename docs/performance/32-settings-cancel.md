# Settings cancellation and file-specific range

Follow-up to #32 / Draft PR #64, validated on 2026-09-09.

## Behaviour

The editor previews choices immediately without updating the saved session until OK.
Cancel, Escape and window close restore the settings present on opening. Reset to
defaults previews the configured defaults, including range, and can itself be cancelled.
Opening another file cancels and closes the editor first.

Range and its mode are file-specific. Opening a file restores the configured range;
only FFT/window/overlap/aggregation/palette choices persist. Session version 5 omits
range, reads versions 1–4 and ignores legacy range values. Configuration is not edited.

Opening the settings editor removes the analysis hover hint. Ctrl+R
(Cmd+R on macOS) applies the recommended range; the binding is shown in both
places that offer the action. The main window and editor handle it in their own
focus contexts, with an application-level fallback for popup focus. A yellow ⚠
accompanies the warning range.

## Validation

Formatting, strict Clippy and all 442 tests passed, followed by a release build.
Persistence tests cover omitted range, ignoring legacy values, configuration range
fallback, and preserving the other saved choices and geometry.

Native checks use the isolated Linux Sway/GLES2 session and real Intel GPU described
in the [earlier report](32-status-settings.md). The owner configuration and desktop
are not involved. Windows/macOS native interaction is not exercised here.

Native results:

- Clicking the status group while its hint was visible opened the editor without
  leaving the hint behind. Dark and light layouts show the warning glyph and binding.
- Ctrl+Shift+R worked in both windows. In the editor it applied 30 dB and focused
  the numeric field. Changing it to 45 and pressing Escape restored absolute 110 dB.
- Changing FFT 2048 → 1024 left the saved session at 2048 during preview. Keyboard
  navigation to OK committed 1024.
- From 1024 / fixed 30 dB, Reset previewed 2048 / absolute 110 dB. Keyboard Cancel
  restored 1024 / fixed 30 dB.
- Changing FFT 1024 → 512 and clicking the window close button restored 1024.
  Restart kept 1024 but restored absolute 110 dB; session version 5 contained no
  dynamic_range key and configuration bytes were unchanged.

[Hint and warning](32-ui-data/cancel-hint.png) ·
[Editor, dark](32-ui-data/cancel-dark.png) ·
[Editor, light](32-ui-data/cancel-light.png) ·
[Shortcut applied](32-ui-data/cancel-shortcut.png) ·
[Reset cancelled](32-ui-data/cancel-restored.png)

## Review follow-up

Two P2 findings were accepted: OK could omit text that had not been committed by
Enter/blur, and the open hint could retain stale advice after applying it. OK now
parses both numeric fields atomically, validates them and stays open on error.
The tooltip observes its owner, so visible content follows effective settings.

Native checks held the virtual keyboard alive while clicking OK: 0 dB left the
editor open with an error; replacing it with 47 and clicking OK applied 47 dB
without Enter. Clicking the recommendation while keeping the pointer inside the
hint updated it to peak-relative 30 dB and removed the warning and recommendation.

[Invalid confirmation](32-ui-data/cancel-invalid.png) ·
[Numeric text confirmed](32-ui-data/cancel-confirmed.png) ·
[Live hint after recommendation](32-ui-data/cancel-live-hint.png)

The final focused independent review was clean. No findings were declined.

The owner subsequently simplified the binding to Ctrl+R (Cmd+R on macOS).
The earlier interaction log and screenshots above retain the original binding.
