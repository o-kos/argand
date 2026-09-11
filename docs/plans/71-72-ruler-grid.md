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

- [ ] Separate optional edge-label clipping from tick/grid visibility.
- [ ] Increase tick length with matching label clearance on both rulers.
- [ ] Add the checked grid action and backward-compatible session persistence.
- [ ] Cover edge ticks, stable spacing, both saved values and older sessions.
- [ ] Run the full local gate and rebuild release.
- [ ] Inspect native rendering and verify toggling does not restart analysis.
- [ ] Resolve independent review findings and complete the plan.
