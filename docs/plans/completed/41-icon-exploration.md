# Issue #41: B0 application artwork

Related issues: #41 and #76.

## Objective

Use the owner-approved B0 closed cyan/coral I/Q ribbons in the application,
desktop launcher and README. Keep the dedicated 16- and 24-pixel optical
versions, which the owner accepted after the application trial.

## Retained assets

- `crates/app/assets/icons/source/argand.png`: selected full-size raster source.
- `crates/app/assets/icons/source/argand-16.svg` and `argand-24.svg`: optical sources.
- `crates/app/assets/icons/generate.py`: reproduces PNG, ICO, ICNS, scalable SVG, monochrome SVG
  and tray PNG assets using Pillow and rsvg-convert.
- Generated application assets remain in `crates/app/assets/icons/`.

The owner explicitly requested removal of all design explorations after selecting
B0. Alternate concepts, intermediate attempts, comparisons, preview pages and
notes have been removed. Only the selected sources and application artwork remain.
Local VS Code build-shortcut settings are preserved and excluded from this PR.

## Implementation

- [x] Obtain owner approval of B0 and its optical small sizes in the application.
- [x] Install colored assets and use the optical 24-pixel PNG in the toolbar's 22-pixel slot.
- [x] Include the 24-pixel icon in the desktop installer; preserve the application id and Icon key.
- [x] Validate the installed desktop entry and launch the current release binary.
- [x] Restore the README logo using the tracked B0 PNG, retaining centered 160-by-160 layout and alt text (#76).
- [x] Verify the GitHub-rendered README image and add `Closes #76` to PR #100.
- [x] Keep selected sources beside application assets and remove all design explorations.
- [x] Retain the approved full-size raster and regenerate monochrome/tray artwork from the optical SVG paths.
- [x] Validate the desktop installation and cross-platform icon containers; keep Windows resource/macOS bundle packaging in #40.
- [x] Move this plan to completed before final review.

## Rendering and delivery

16 and 24 pixels use the optical SVG redraws; larger sizes retain the selected
raster. The scalable SVG embeds that approved raster so desktop theme lookup
uses the same design. Retaining the selected image avoids replacing it with a
different full-size vector tracing. Monochrome and tray artwork use the optical
SVG paths with transparent gaps at crossings. The desktop
entry uses `io.github.o_kos.argand` for its icon and application identity. The
local installer points Exec at the current release binary and refreshes caches.

## Validation

- [x] Verify generated application assets are byte-identical after source relocation.
- [x] Validate PNG dimensions and alpha, ICO optical frames and every ICNS representation.
- [x] Validate the desktop entry before and after installation and compare installed assets.
- [x] Verify README dimensions/alt text and the GitHub image blob against the local PNG.
- [x] Run formatting, strict Clippy and the full test suite.
- [x] Rebuild the release workspace after the standard checks.
- [x] Review the icon integration, README update and source relocation independently.

The prior review found one stale instruction about downloading the design folder
alone; it was corrected, and follow-up review was clean. That preview page and
its instructions have now been removed with the explorations at the owner's
request. No findings were declined. The application artwork remains unchanged.

The focused review of the exploration removal found no substantive issues:
selected sources and generated artwork are unchanged, and no references or build
dependencies point to deleted files.

## Acceptance and merge validation

The owner accepted PR #100. Final preparation completed the previously unchanged
monochrome/tray variants without changing any accepted colored PNG. The generator
reproduces the complete set byte-for-byte; tray images have valid RGBA dimensions
and transparency. The monochrome rendering was inspected on a light background.

The original spectrum-baseline proposal in #41 was superseded by the owner's
B0 selection. A full-size vector tracing is not needed to ship the accepted
artwork; the retained raster is the source of truth for larger sizes. Native
Windows resources and macOS bundle integration remain part of packaging #40,
not a claim made by this asset change. Before merge, `ci/full` must explicitly
succeed for Linux, Windows and macOS on the current PR revision.

Final acceptance review validated the monochrome/tray completion. It found only
stale PR wording about pending work and the old plan path; both were corrected.
The follow-up round found no substantive issues. No findings were declined.
