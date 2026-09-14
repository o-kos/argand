# Issue #41: Application icon exploration

Related issues: #41 and #76.

## Overview

Preserve the icon exploration and integrate the selected B0 design. The owner
accepted B0 and its optical small sizes after the application trial and requested
the README logo fix (#76) in this same PR. The final vector master and remaining
delivery work are still pending.

## Context

The owner restated the requirements: recognizable at 16, 24 and 32 pixels, clear
on both light and dark backgrounds, and preferably without an enclosing tile.
These goals reopen the scope of #41; its earlier baseline shift and axis-tail
choices are on hold. The initial A–D concepts and subsequent B variants must
remain available together with their comparisons and known limitations.

## Decisions

- The owner rejected B2 and currently prefers B0 with a closed coral ribbon.
  B1 remains archived, with mutual alternating cyan/coral crossings.
- The owner approved trying B0 and the 16/24-pixel optical versions in the application,
  including the installed desktop launcher.
- Store the complete visual archive under docs/design/app-icon/.
- Keep PNG comparison sheets viewable directly on GitHub and provide a local
  HTML page using relative image paths for actual-size inspection.
- Preserve intermediate attempts, including defective ones, with explicit labels.
- The application trial replaces the colored application assets. Following owner
  acceptance, #76 is included in this PR: the README uses the tracked B0 PNG with
  its centered 160-by-160 layout and descriptive alt text preserved. Monochrome/tray
  artwork remains pending.

## Implementation steps

- [x] Archive all concepts, B iterations, comparison sheets and discussion notes.
- [x] Validate image integrity, references and portability; push a Draft PR.
- [x] Agree B0 and its small-size adaptation for an application trial.
- [x] Obtain final design approval after the application trial.
- [x] Install B0 colored assets in the source tree and add the 24-pixel desktop size.
- [x] Validate and install the local desktop entry, rebuild, and launch the trial.
- [x] Restore the README logo using the current B0 asset (#76).
- [x] Remove redundant session exports and place retained artwork sources beside application assets.
- [ ] Verify the updated README image on GitHub and link #76 for automatic closure.
- [ ] Produce deterministic vector artwork and the complete application icon set.
- [ ] Validate the selected icon in the application and supported delivery paths.
- [ ] Update relevant documentation and move this plan to completed before final review.

## Validation

- [x] Verify archived images match the original preview files.
- [x] Check relative HTML/Markdown/SVG references and inspect comparison sheets.
- [x] cargo fmt --all -- --check
- [x] cargo clippy --all-targets --locked
- [x] cargo test --locked
- [x] Independent review of the archive and its claims.

The earlier archive-only step required no release rebuild. The application trial
requires the standard gate, independent review and a current release build.

## Small-size experiment

B0 has experimental 16- and 24-pixel vector redraws in
`crates/app/assets/icons/source/`, compared with unmodified raster reductions
on light and dark backgrounds. Flat colors and size-specific ribbon/outline
widths open the gaps. The owner accepted these variants for the application trial. The final-master
task remains open.

## Application trial

`crates/app/assets/icons/generate.py` reproduces the colored PNG, ICO and ICNS assets.
16 and 24 pixels use the optical redraws; larger sizes retain the selected B0
raster sketch. The toolbar embeds the 24-pixel artwork for its 22-pixel slot.
The scalable theme SVG temporarily embeds the same raster to prevent desktop
lookup from selecting the old icon; it is not a vector master. Monochrome and
tray variants are deferred to the final artwork step. The desktop application
id and `Icon` key stay `io.github.o_kos.argand`; installation points `Exec` at
the current release binary and refreshes the desktop/icon caches.

## Trial validation

- Formatting, strict Clippy and the full test suite passed.
- The release workspace was rebuilt after those checks.
- Independent review of the full working tree found no substantive issues; no
  findings were declined. It verified source selection, icon containers, archive
  integrity, desktop identity and the explicit limits of the trial.
- The desktop entry passed `desktop-file-validate` before and after installation.
  Every installed PNG and the scalable SVG matches the source tree byte-for-byte.
- The fresh release binary was launched successfully for owner inspection.
  The owner accepted the appearance and requested only the README logo update
  in this feedback round.

## Session cleanup

At the owner's request, the two preview directories, per-background PNG copies,
ZIP archive, preview generators and prompt notes were removed. The selected
raster and optical SVG sources now live beside the application artwork under
`crates/app/assets/icons/source/`. The colocated generator reproduces every
existing application asset byte-for-byte. Two comparison sheets and the original
concept archive remain in `docs/design/app-icon/`; the HTML page references
retained images directly. Local VS Code build-shortcut settings are preserved
and excluded from this PR.

## Cleanup validation

- Formatting, strict Clippy and the full tests passed; the release build followed.
- Every generated application asset remains byte-identical after moving the sources.
- Independent review found one stale instruction to download only the design
  directory. It now requires the full repository. No findings were declined;
  the follow-up round found no substantive issues.
- All retained design links resolve. README preserves centered placement,
  160-by-160 display dimensions and descriptive alt text.

## Preservation checkpoint

The archive contains all nine sketch files, a snapshot of the current icon, two
PNG comparison sheets, their portable SVG layouts, and a relative-path HTML page.
The original sketch bytes are preserved. Every image was decoded successfully;
all local links resolve, and both SVG layouts render pixel-identically to their
archived PNG sheets. The standard gate passed for the plan push and is repeated
by the repository hook for the archive push. Final design and integration remain
open for a later owner discussion.

Independent review identified one ambiguous statement implying that B1 had been
selected. The archive now distinguishes the proposal to continue B1 from the
owner decision to explore direction B; choosing between variants remains open.
The follow-up review found no substantive remaining issues. No findings were
declined.
