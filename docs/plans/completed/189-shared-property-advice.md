# Issue #189: Shared property advice callouts

Resolves #189; continues Draft PR #224.

## Roles and risk

- Class A: expected extraction diff above roughly 400 lines; behavior remains scoped to the two property editors.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate context.

## Decisions and toolkit boundary

Extract the existing Range advice state, measured side placement, standard Button close action, fallback Tooltip and retained domain pointer drawing into one shared implementation. No replacement control or new custom-input exception. Range keeps its optional recommendation action and shortcut. File format advice is informative.

After automatic FLAC format fallback, show a yellow marker beside the selected sample format and an automatic anchored callout when room permits. Closing its X leaves the marker and permits ordinary hover advice without immediately reopening the automatic callout. If neither side has room, use the existing hover fallback. Hide automatic and hovered advice while a panel Select is open. Explicit format/container choice or starting conversion clears the warning; another automatic change resets dismissal. Do not increase panel height with an inline warning row.

## Implementation and validation

- [x] Extract and reuse the Range advice lifecycle/rendering/placement.
- [x] Attach format advice to the measured sample-format value; retain warning clearing/reset rules.
- [x] Preserve Range action, Ctrl+R and keyboard/pointer behavior.
- [x] Meaningful GPUI auto/close/hover/list-suppression/clear and constrained-viewport coverage.
- [x] Update the retained UI inventory justification.
- [x] Normal application check, formatting, all-targets Clippy and focused regressions.
- [x] Move completed plan before final independent review.
- [ ] Full locked suite, fresh release after the gate, existing Draft PR publication.

No native GUI launch, desktop input emulation or portal requests. Distinguish automated rendered-layout evidence from native validation.

## Review correction

Accepted the consistency observation that the generic warning token need not equal the existing Range palette. The exact existing dark/light Range color resolver now lives in the shared advice module and is used by both panels; Range appearance is unchanged.
