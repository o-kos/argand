# #124 integration: Linux native results

Native checks of the integrated #124 architecture for [#133](../../plans/133-ui-integration.md).
Linux only, by owner decision; Windows and macOS rely on `ci/full`.

Build: release of branch `chore/133-ui-integration` at `REVISION`.
Environment: `DISTRO`, `DESKTOP` / `SESSION` (Wayland or X11), display scale `SCALE`.

Result is `pass`, `fail` (with the Issue that tracks it) or `not exercised`.
`not exercised` is outstanding coverage, not success.

## Frame and title bar

| Case | Input | Expected | Result |
| --- | --- | --- | --- |
| R1 | Look at the main and settings windows, dark and light | One frame, no second border or shadow | |
| R2a | Resize from each edge and corner | The visible edge follows the pointer; cursor matches the edge | |
| R2b | Maximize, restore, then tile left and right if the desktop supports it | No stray resize zones while expanded; restored insets are one frame's | |
| R3a | Click the application button, Grid and an orientation segment | Each acts once; the window does not move or maximize | |
| R3b | Drag and double-click the bare title | Window moves; double-click maximizes and restores once | |

## Keyboard and focus

| Case | Input | Expected | Result |
| --- | --- | --- | --- |
| F1 | With a file open, press every plot navigation key once (arrows, Ctrl+arrows, Ctrl+Plus/Minus/0, Ctrl+Shift+Plus/Minus/0, Ctrl+G, Ctrl+U, Ctrl+T) | Exactly one change each; no duplicate re-analysis in the status bar | |
| F2 | In Settings (Ctrl+,), type digits and symbols, move the caret, select, copy, paste, undo in both numeric fields | The field edits; the plot behind never pans, zooms or toggles | |
| F3a | Settings: open a dropdown, Escape, Escape | First Escape closes the list, second cancels the form; the next key reaches the plot | |
| F3b | Application menu (F10): into File with Right, Escape | The whole menu closes; the next arrow pans the plot | |
| F3c | Time-ruler context menu: open, Escape | Menu closes; Alt guides work without moving the mouse | |

## Pointer isolation

| Case | Input | Expected | Result |
| --- | --- | --- | --- |
| P1a | Open the application menu over the plot; click, wheel and drag inside it and outside it | Menu handles its own rows; outside click only closes it; no pan, zoom or window move | |
| P1b | Open the pinned FFT hint (hover the summary or right click); click, wheel over the plot | Plot frozen; the click closes the hint and starts no pan | |
| P2 | Hover a toolbar hint and a unit hint over the plot, with and without Alt | Arrow cursor, no readout or guides under the hint; they return on leaving without a mouse move | |
| P3a | Start a pan and release outside the window | Pan ends; later moves do not resume it | |
| P3b | Start a pan, then press F10, then Ctrl+O | Drag ends before the menu or chooser | |
| P3c | Start a pan, switch to another window and back | Drag ended; no stuck hand cursor | |

## Status, pictures and settings

| Case | Input | Expected | Result |
| --- | --- | --- | --- |
| O1 | While "ready" shows, click, wheel and type, including over the menu | Status disappears; the target still gets the input once; no navigation from the observer | |
| G1a | Open a long capture and resize the window during progressive refinement | Picture follows; no stale frames or growing memory; refinement not restarted | |
| G1b | Zoom and pan repeatedly during refinement | No flicker of old images; backdrop fills uncovered time | |
| G1c | Open another file while the first is still analysing | Old picture replaced cleanly; no mixed labels | |
| S1a | Settings: change FFT size and palette, Cancel | Picture and labels return to the opening values | |
| S1b | Settings: change, OK, restart the application | Accepted settings restored; range resets on open | |
| S1c | Quit and restart | Window size, orientation, grid, scale controls and time format restored | |

## Layout and appearance

| Case | Input | Expected | Result |
| --- | --- | --- | --- |
| L1 | Horizontal and vertical orientation, dark and light theme | Rulers, minimap (3 rem), zoom pairs and badges placed as accepted | |
| L2 | Start page with and without recent files, then a loaded file | Toolbar shows only the application button on the start page; recent names only, directory in the hint | |
| L3 | `[panels] minimap_size = "64 px"` in `argand.toml`, restart | Minimap 64 logical pixels in both orientations | |
| L4 | Narrow and short window, open the application menu into File | Menu stays on screen and scrolls; parent row reachable | |

## Performance against the baseline

Baseline: release of `c176ad2` (before the GPUI Kit migration). Same capture, same
window size, `RUST_LOG=argand::ui_latency=trace` for both.

| Case | Input | Baseline | Branch | Result |
| --- | --- | --- | --- | --- |
| PF1 | Continuous window resize for about 5 seconds | | | |
| PF2 | Ctrl+wheel zoom in and out across the capture | | | |
| PF3 | Drag pan across the capture | | | |
| PF4 | Time to the first picture after opening the capture | | | |
