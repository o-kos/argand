# Application icon exploration

Design work for [#41](https://github.com/o-kos/argand/issues/41), preserved for
review from another machine. **B0 and its optical small sizes are approved for an application trial.**
Final release artwork remains pending.

## Start here

- [Initial A–D comparison](initial-comparison.png)
- [Development of B: original, closed loops and two links](b-comparison.png)
- [Local comparison page](index.html): after cloning or downloading this directory,
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

The owner selected **B, the intertwined I/Q direction**, for further development.
In B0 the coral ribbon is unintentionally open: its returning segment is missing.
B1 restores two closed loops; B2 simplifies the composition to two chain links.
The owner rejected B2 and currently prefers B0, requesting its missing coral
return segment. B1 remains an alternative with mutual over-under crossings
requested for its cyan and coral loops. No final drawing is approved. The dense center at 16 pixels
still needs work. The topology and crossing geometry also need deliberate
construction in the eventual vector master.

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

## Revised B0 and B1

- [Experimental B0 optical sizes: 16 and 24 pixels](small-b0-v3/index.html), with original/revised comparisons on both backgrounds and 8× pixel enlargements

The optical-size experiment redraws the rounded loops as closed vector paths,
uses flat colors, and adjusts ribbon and outline widths separately for 16 and
24 pixels. It is a new approximation of B0, not an exact trace or an approved
final master. The owner accepted this redraw for the application trial. Its wider
openings trade some visual weight for separation.

- [All size previews on light and dark backgrounds](size-preview-v2/index.html)
- [Comparison sheet, 16–256 pixels](size-preview-v2/comparison-16-256.png)
- [Download all previews](size-preview-v2/previews.zip)

Sizes: 16, 20, 24, 32, 40, 48, 64, 128, 256, 512 and 1024 pixels.
These are Lanczos reductions of the v2 sketches, without optical corrections.
The archive includes transparent PNGs and both opaque background variants.

- [B0 v2: closed coral return](images/b0-closed-return-v2.png) preserves the rounded B0 silhouette and adds its missing return
- [B1 v2: mutual weave](images/b1-mutual-weave-v2.png) alternates cyan over coral at the top and bottom, and coral over cyan at the left and right

B0 is now used in the application trial; B1 remains an archived alternative.
The original comparison sheets remain historical snapshots.

## Intermediate attempts

These files are retained to preserve the complete discussion, not as approved
alternatives. Painted checkerboards are actual opaque image content, not alpha.

| Attempt | Known defect | Original image |
| --- | --- | --- |
| 01 | Missing coral return segment; painted checkerboard background | [PNG](images/b-intermediate-01-checkerboard.png) |
| 02 | Transparent background restored, but coral return segment still missing | [PNG](images/b-intermediate-02-transparent.png) |
| 03 | Extra internal strips and painted checkerboard background | [PNG](images/b-intermediate-03-extra-strips.png) |

## Next discussion

Choose how to develop direction B, then refine small-size readability and agree
the final silhouette, colours, outline and crossing geometry before creating
the final vector master and monochrome assets. The application trial already
uses B0 PNG, ICO and ICNS artwork under `crates/app/assets/icons/`. The scalable
SVG is temporarily a self-contained raster wrapper. Run
`python docs/design/app-icon/install-trial-artwork.py` to reproduce the trial
assets with Pillow. The historical comparisons remain unchanged.
