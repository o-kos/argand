# Ruler marks and grid visibility (#71, #72)

## Design

Keep the selected tick spacing and label placement policy. When a GUI time label
would overflow the plot, hide its text but keep the valid tick and grid line.
Expose this behavior explicitly through label metrics so CLI output retains its
existing layout contract. Lengthen both GUI ruler ticks and leave clear space
before their labels.

Add a checked View > Show grid command. Persist the choice in session state;
older sessions default to visible. Toggle only the grid over the spectrogram:
baselines, ticks, labels and the independent minimap remain readable. A grid
change requests a redraw, without reading samples or running FFTs.

## Implementation

- [x] Separate optional edge-label clipping from tick/grid visibility.
- [x] Increase tick length with matching label clearance on both rulers.
- [x] Add the checked grid action and backward-compatible session persistence.
- [x] Cover edge ticks, stable spacing, both saved values and older sessions.
- [x] Run the full local gate and rebuild release.
- [x] Inspect native rendering and verify toggling does not restart analysis.
- [x] Resolve independent review findings and complete the plan.

## Results

Formatting, strict Clippy and all 529 tests passed; release rebuilt. Native Linux
GPU checks on the 19-hour 1 GB capture showed the previously missing rightmost
hour tick/grid line with its overflowing label hidden. Grid toggling retained
ruler ticks and labels and left analysis counters unchanged. Both values were
saved; a restart restored the hidden grid, with sampled plot pixels matching the
prior hidden-grid view. Unit tests cover tick retention and unchanged readable
labels across window widths and all three time formats. Older session versions
retain their geometry/state and default to a visible grid.

Independent review found no substantive issues. The parent frequency-navigation
change remains isolated in #87; this PR changes ruler/grid behavior only.
