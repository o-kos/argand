# Issue #41: Application icon exploration

Related issue: #41.

## Overview

Preserve the icon exploration for review from another machine. The owner has
chosen direction B for further exploration but has not approved a final icon.
Production artwork remains unchanged while the design is discussed.

## Context

The owner restated the requirements: recognizable at 16, 24 and 32 pixels, clear
on both light and dark backgrounds, and preferably without an enclosing tile.
These goals reopen the scope of #41; its earlier baseline shift and axis-tail
choices are on hold. The initial A–D concepts and subsequent B variants must
remain available together with their comparisons and known limitations.

## Decisions

- Store the complete visual archive under docs/design/app-icon/.
- Keep PNG comparison sheets viewable directly on GitHub and provide a local
  HTML page using relative image paths for actual-size inspection.
- Preserve intermediate attempts, including defective ones, with explicit labels.
- Keep application assets and the independently tracked README logo fix (#76)
  outside this preservation step.

## Implementation steps

- [ ] Archive all concepts, B iterations, comparison sheets and discussion notes.
- [ ] Validate image integrity, references and portability; push a Draft PR.
- [ ] Agree the final shape, colour treatment and small-size adaptation with the owner.
- [ ] Produce deterministic vector artwork and the complete application icon set.
- [ ] Validate the selected icon in the application and supported delivery paths.
- [ ] Update relevant documentation and move this plan to completed before final review.

## Validation

- [ ] Verify archived images match the original preview files.
- [ ] Check relative HTML/Markdown/SVG references and inspect comparison sheets.
- [ ] cargo fmt --all -- --check
- [ ] cargo clippy --all-targets --locked
- [ ] cargo test --locked
- [ ] Independent review of the archive and its claims.

No release rebuild is needed for the archive-only step: no application sources or
production assets change, and no new application behavior is demonstrated.
