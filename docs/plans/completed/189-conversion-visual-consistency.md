# Conversion property panel visual consistency

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

## Roles and risk

Class A: cumulative GUI refactoring exceeds roughly 400 changed lines and shares
property-panel ownership and deferred event routing; conversion/document semantics remain unchanged.
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

The two status editors share one explicit `StatusPanel` owner, common status button /
popover composition and one opening/closing route for hover, click and keyboard commands.
Choose a new owner before closing its predecessor; stale Closed events cannot clear it.
Keep Analysis preview/commit and File draft-discard callbacks separate. Reuse existing
PinnedHint hover, focus and Select handling; do not introduce another overlay framework.

## Implementation and validation

- [x] Share existing Analysis presentation builders and use them for File information.
- [x] Verify both actual panels in the same window/theme/scale using the isolated
      diagnostic launcher on the second monitor, without input emulation or portals.
- [x] Retain focused input/Select/Escape regression coverage.
- [x] Verify exclusive ownership in both directions through hover and keyboard paths,
      including draft discard, focused Select, stale close notifications and actual rendering.
- [x] Complete independent review of numeric width, shared ownership and deferred request ordering.
- [x] Pass formatting, all-targets Clippy and the complete locked workspace suite.
- [x] Rebuild the current release workspace after the gate.
- [x] Move this plan to completed before final review; obtain coordinator visual
      acceptance of the shared presentation and document diagnostic capture limits.

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

The eleven conversion/panel semantic regressions pass after the refactor, including
both-direction hover/keyboard switching, focused Select draft discard and stale close events.
No tests restate visual constants; actual paired rendering is required. One behavior
regression covers shared ownership across both hover and keyboard directions.

The coordinator accepted paired actual File/Analysis captures from the same inactive
window, theme and scale. Numeric Input widths are capped by the shared property
column geometry; long finite or invalid drafts remain inside the panel. Conversion
uses the stock Button's keyboard focus ring; the Analysis reset action additionally
retains its existing externally managed focus border.

## Final validation and review

Formatting, all-targets Clippy and the complete locked workspace suite pass:
1,110 tests, zero failures, one ignored manual DSP benchmark. The current release
workspace was rebuilt after this gate. The focused ownership regression covers
both hover directions, F11/Ctrl+, draft discard with an open Select, stale Closed
notifications and an actual opening superseding an older deferred command.

Independent review accepted the deferred-request ordering finding; actual Opened
now invalidates older requests. The correction and final runtime review were clean.
No findings were rejected. Keyboard focus was verified against the locked standard
Button implementation; numeric drafts are bounded to the shared value column.

Paired actual rendering established the shared presentation. A diagnostic transition
from File to Analysis displayed only Analysis. For the reverse transition, production
Shell and File editor render callbacks ran with File as sole owner before any forced
refresh; native X11 window captures retained an old image even after a diagnostic
refresh. A resize of only the diagnostic window refreshed its native backbuffer;
the resulting reverse capture displayed File alone. Neither diagnostic rendering
nor this capture workaround establishes native Wayland pointer acceptance.
No desktop input emulation or RemoteDesktop requests were used. Diagnostic processes
were terminated after capture. Conversion and document lifecycle remain unchanged.
