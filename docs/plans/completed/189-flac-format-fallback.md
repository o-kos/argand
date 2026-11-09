# Issue #189: Complete FLAC sample-format fallback

Resolves #189; continues Draft PR #224.

## Roles and risk

- Class B: local draft-format choice; no codec, DSP or document-lifecycle change.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate context.

## Decisions

Use one typed FLAC support predicate for the menu and container transition. Any unsupported current draft format becomes I16 in the same draft domain when FLAC is selected, including I32. U8/I16 stay unchanged and produce no format-change warning. Preserve Fs/carrier and existing yellow warning lifecycle. Returning to WAV keeps the current format. Initial FLAC documents already use a supported encoding; repeated FLAC confirmation also applies the same predicate.

Integer labels use Int 8-bit, Int 16-bit and Int 32-bit in rows, selected titles and the fallback warning. Format identifiers are unchanged.

## Implementation and validation

- [x] Share the typed support predicate and replace label-specific fallback.
- [x] Extend the existing matrix to all ten formats, both domains and repeated FLAC selection.
- [x] Synchronize current architecture documentation; preserve unrelated changes.
- [x] Normal application check, targeted regression, formatting, all-targets Clippy.
- [x] Independent review; move plan to completed before final review.
- [x] Full locked tests and fresh release build after the gate.

No native GUI launch or input emulation. Keep PR Draft and report remote checks separately.

## Completion evidence

The normal application check, formatting, all-targets Clippy and full locked suite pass: 1,142 tests, zero failures and one ignored manual benchmark. The release build completed after the gate. Independent review returned no substantive findings; none were rejected. The existing GUI matrix now covers every sample type, including both I32 domains, supported U8/I16 without warnings, frequency-draft preservation and repeated container confirmation. Native GUI validation was not run.

The optional Range-style warning callout remains a separate UX discussion and is not implemented by this plan.
