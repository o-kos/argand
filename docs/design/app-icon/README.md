# Application icon exploration

Design work for [#41](https://github.com/o-kos/argand/issues/41), preserved for
review from another machine. **The owner accepted B0 and its optical small sizes after the application trial.**
The README now uses the same B0 artwork. The final vector master and remaining
release artwork are still pending.

## Start here

- [Initial A–D comparison](initial-comparison.png)
- [Development of B: original, closed loops and two links](b-comparison.png)
- [Local comparison page](index.html): after cloning or downloading the full repository,
  open `index.html` in a browser at 100% zoom. All references are relative and no
  network access is needed. GitHub displays HTML source rather than running it.

The sheets show 88, 16, 24, 32 and 48 pixel samples on white and dark gray.
GitHub may scale preview images to the available width; open a PNG at its native
size or use the HTML page to judge small sizes. Browser sizes are CSS pixels;
physical pixels depend on display scaling. The SVG sheets are comparison layouts
referencing the PNG sketches, **not vector source artwork for the icons**.

## Requirements and current direction

- Recognizable at 16, 24 and 32 pixels through the overall silhouette.
- Clearly visible on both light and dark backgrounds, including the Ubuntu dock.
- Prefer a transparent application icon without an enclosing background tile.
  A separate backing can still be considered for a large presentation logo.
- Keep one recognizable identity across sizes; simplify small versions if needed.

The earlier #41 baseline shift and vertical-axis tail decisions are on hold while
these requirements are explored. The owner explicitly allowed radical alternatives.

The owner selected **B0 with a closed coral return**, then accepted the separate
16- and 24-pixel optical redraws after trying them in the application. B2 was
rejected; B1 remains an archived alternative. A final full-size vector master
and updated monochrome/tray artwork remain pending.

## Initial directions

![Initial concepts on light and dark backgrounds](initial-comparison.png)

| Sketch | Idea | Original image |
| --- | --- | --- |
| Current | Existing application icon, kept as a fixed comparison reference | [PNG](images/current.png) |
| A | Compact spectral crest | [PNG](images/a-spectrum.png) |
| B0 | Intertwined I/Q ribbons; missing coral return segment | [PNG](images/b0-iq-weave.png) |
| C | Signal mark built around A | [PNG](images/c-signal-a.png) |
| D | Phase pointer | [PNG](images/d-phase-pointer.png) |

## Development of B

![Development of B on light and dark backgrounds](b-comparison.png)

| Sketch | Status | Original image |
| --- | --- | --- |
| B1 | Original closed loops; crossing revision requested | [PNG](images/b1-closed-loops.png) |
| B2 | Rejected by the owner; retained only as an archive | [PNG](images/b2-two-links.png) |

## Selected artwork and comparisons

- [B0 source](../../../crates/app/assets/icons/source/argand.png)
- [Optical SVG sources: 16 pixels](../../../crates/app/assets/icons/source/argand-16.svg) and [24 pixels](../../../crates/app/assets/icons/source/argand-24.svg)
- [B1 revision: mutual weave](images/b1-mutual-weave-v2.png)
- [B0/B1 comparison, 16–256 pixels](selected-comparison.png): historical raster reductions before optical corrections
- [Optical-size comparison](optical-comparison.png): original reductions beside the accepted 16/24-pixel redraws
- [Local comparison page](index.html): uses the original assets on CSS backgrounds, including larger sizes

Production PNG, ICO, ICNS and scalable SVG assets live in
`crates/app/assets/icons/`. The selected source and optical SVGs are retained
there under `source/`; the build script uses these files directly. Redundant
per-background PNG exports, the ZIP download, preview generators and prompt
notes were removed at the owner's request. The original concept archive and
the two useful comparison sheets remain available.

## Intermediate attempts

These files are retained to preserve the complete discussion, not as approved
alternatives. Painted checkerboards are actual opaque image content, not alpha.

| Attempt | Known defect | Original image |
| --- | --- | --- |
| 01 | Missing coral return segment; painted checkerboard background | [PNG](images/b-intermediate-01-checkerboard.png) |
| 02 | Transparent background restored, but coral return segment still missing | [PNG](images/b-intermediate-02-transparent.png) |
| 03 | Extra internal strips and painted checkerboard background | [PNG](images/b-intermediate-03-extra-strips.png) |

## Rebuilding the application artwork

Run `python crates/app/assets/icons/generate.py` with Pillow and `rsvg-convert`
installed. It reproduces the current colored PNG, ICO, ICNS and scalable SVG
assets byte-for-byte. Large artwork still uses the selected raster; the scalable
SVG is a self-contained raster wrapper, not the pending full-size vector master.
The README references the tracked 512-pixel PNG at a display size of 160 pixels.
