# Issue #189: Select and tooltip interaction

Resolves #189; continues Draft PR #224.

## Roles and risk

- Class B: local property-editor presentation and draft choices; no DSP or document lifecycle changes.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate context.

## Decisions

- Keep standard Select, Tooltip, and supported styling. Suppress property-panel tooltips while a child Select is open, including an already visible tooltip.
- Size the closed trigger for its current title; retain the widest menu width. Verify actual bounds rather than infer glyph clipping from the screenshot.
- Explain floating sample scales briefly, without reader or codec implementation details.
- Selecting FLAC converts a floating draft format to I16 in the current draft domain. Preserve Fs and carrier; show a warning, never conceal other validation errors. Returning to WAV keeps I16. An explicit format/container choice or starting conversion clears the warning. I32 keeps its existing incompatibility error.

## Toolkit evidence

Locked Select exposes its list focus handle while open and trigger handle while closed. Existing editors already retain closed handles for supported synchronous Cancel. Reuse this lifecycle, without a replacement control or toolkit patch. Stock grouped FLAC choices remain filtered to supported types.

## Implementation

- [x] Shared Select trigger/menu geometry and tooltip interaction.
- [x] Concise, bounded tooltip text/layout.
- [x] Float-to-FLAC draft transition and warning.
- [x] Meaningful combined pointer/keyboard and draft-state regressions.
- [x] Independent review; move completed plan before final review.

## Validation

- [x] Normal `cargo check -p argand --locked`.
- [x] Focused GUI regressions.
- [x] `cargo fmt --all -- --check`.
- [x] `cargo clippy --all-targets --locked`.
- [x] `cargo test --locked`.
- [x] Fresh `cargo build --release --locked` after the gate.

No native GUI launch or input emulation. Report automated layout evidence separately from native validation.

## Verified toolkit behavior

GPUI prepaint explicitly removes an active tooltip when its element no longer declares a tooltip builder. The combined pointer regression verifies disappearance and restoration, and selecting a shorter title verifies a smaller trigger with the same right anchor. Both property editors observe standard Select notifications so open-state changes invalidate their tooltip composition.

## Completion and review

The normal application check, formatting, all-targets Clippy and full locked suite pass: 1,142 tests, zero failures and one ignored manual benchmark. The release build completed after the gate. Independent review of the runtime and final tests-only geometry adjustment returned no substantive findings; none were rejected.

The existing Analysis regression keeps its full open-menu bounds assertions while walking options. After confirmation it now checks the invariant right anchor, top and height, because the approved trigger width follows the selected title.

Native desktop visual validation was not run. Automated GPUI pointer tests prove the combined open-list/tooltip transition and selected-trigger geometry; they do not establish native Wayland interaction.
