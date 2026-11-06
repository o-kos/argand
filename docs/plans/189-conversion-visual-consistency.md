# Conversion property panel visual consistency

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

## Roles and risk

Class B: local GUI presentation, preserving conversion and document semantics.
Coordinator: gpt-6-astra medium; executor: gpt-6.1-sol medium;
independent reviewer: gpt-6.1-sol high.

## Scope and decisions

The file status button and file information editor must reuse the existing Analysis
settings presentation, rather than approximating its appearance. Share the status
button builder, aligned property rows, measured frameless Select presentation,
editable-value underline, pointer ink and compact outlined action Button. Keep all
controls standard toolkit components and retain the approved conversion lifecycle.
Numeric drafts use existing locale-aware input formatting; units remain separate
muted labels. No unrelated FFT summary spacing change is included.

## Implementation and validation

- [x] Share existing Analysis presentation builders and use them for File information.
- [ ] Verify both actual panels in the same window/theme/scale using the isolated
      diagnostic launcher on the second monitor, without input emulation or portals.
- [ ] Retain focused input/Select/Escape regression coverage and obtain independent
      review of the exact focused diff.
- [ ] Pass formatting, all-targets Clippy and the complete locked workspace suite.
- [ ] Rebuild the current release workspace after the gate.
- [ ] Move this plan to completed before final review; obtain coordinator visual
      acceptance before publishing the existing Draft PR update.

Only rendering is demonstrated by the diagnostic launcher. Native Wayland hover
interaction is not claimed. Temporary hooks open the existing surfaces without
changing their rendering; no user documents or processes are used.

## Implementation evidence

Both status summaries use `settings_ui::status_control`, preserving the existing
Analysis XS text, muted resting ink, hover group and compact geometry. Both property
editors use `settings_editor` builders for aligned rows, framed editable marks,
frameless numeric Input/unit composition, measured right-aligned Selects and compact
outlined action Buttons. These builders derive colors and typeface from Theme;
they do not replace toolkit controls or alter global component defaults.

The ten existing conversion/panel semantic regressions pass after the refactor.
No tests were added to restate visual constants; actual paired rendering is required.
