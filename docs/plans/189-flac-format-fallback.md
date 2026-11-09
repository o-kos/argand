# Issue #189: Complete FLAC sample-format fallback

Resolves #189; continues Draft PR #224.

## Roles and risk

- Class B: local draft-format choice; no codec, DSP or document-lifecycle change.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate context.

## Decisions

Use one typed FLAC support predicate for the menu and container transition. Any unsupported current draft format becomes I16 in the same draft domain when FLAC is selected, including I32. U8/I16 stay unchanged and produce no format-change warning. Preserve Fs/carrier and existing yellow warning lifecycle. Returning to WAV keeps the current format. Initial FLAC documents already use a supported encoding; repeated FLAC confirmation also applies the same predicate.

## Implementation and validation

- [ ] Share the typed support predicate and replace label-specific fallback.
- [ ] Extend the existing matrix to all ten formats, both domains and repeated FLAC selection.
- [ ] Synchronize current architecture documentation; preserve unrelated changes.
- [ ] Normal application check, targeted regression, formatting, all-targets Clippy.
- [ ] Independent review; move plan to completed before final review.
- [ ] Full locked tests and fresh release build after the gate.

No native GUI launch or input emulation. Keep PR Draft and report remote checks separately.
