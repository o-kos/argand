# Issue #189: Consistent property tooltips and sample choices

Resolves #189; follow-up in Draft PR #224.

## Roles and risk

- Class B: local GUI composition and presentation over standard controls; no DSP,
  document lifecycle, codec or scheduling changes.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, independent read-only context.

## Contract and toolkit evidence

Metadata tooltip headings and body share one small text size. Headings are
semibold; both body explanations use the same muted foreground. Measurement must
match rendering. Shared frameless Select width includes text, both borders,
small-control padding, its gap and rem-sized caret rather than a fixed allowance.

Locked gpui-component 0.6.6 exposes `Select::menu_max_h` and grouped
`SearchableListItem::{title, render, disabled}`. Keep the stock Select and list.
Use viewport-bounded menu height so all ten formats and both group headings fit
when space permits; retain scrolling on smaller viewports. Rows omit redundant
domain prefixes, while the collapsed and accessible title retains Real/IQ.
FLAC menus contain only the four supported real/IQ U8/I16 formats; switching
container never silently changes the current format, Fs, reference or other draft
values. An unsupported current choice remains visible through the stock placeholder
and existing validation until a valid choice is made. Its retained value exists
only while absent from the Select, avoiding a second copy of an active choice.

The stock disabled-item alternative was tested first: the locked adapter paints
disabled rows but its outer List click still confirms them. A real GPUI pointer
regression reproduced this. Use the owner-approved filtering alternative rather
than patching the toolkit or introducing custom interaction handling.

No new custom control or exception is introduced. Existing passive tooltip and
property-editor composition exceptions remain unchanged. No native GUI launch,
input emulation or portal is permitted; report automated layout coverage honestly.

## Work and validation

- [x] Unify shared metadata tooltip typography and width measurement.
- [x] Correct shared standard Select chrome sizing and viewport menu height.
- [x] Give grouped sample rows concise labels and eligible FLAC choices.
- [x] Verify rendered width at multiple font sizes and selection/draft behavior.
- [ ] Check normal production compilation and the full formatting/Clippy/test gate.
- [x] Move completed plan before final independent review; address findings.
- [ ] Rebuild current release after the gate and publish the existing Draft PR.

## Focused evidence and boundary

Normal production compilation and all-targets Clippy pass. The conversion UI suite
passes all 22 tests. A rendered GPUI regression at rem sizes 16 and 24 checks the
FLAC trigger against independently measured glyph width plus stock control chrome,
all ten WAV rows within a 1000-pixel viewport, exclusion of unsupported FLAC rows,
actual pointer and keyboard choices, unsupported committed-value retention and
WAV restoration without frequency draft changes. Native visual acceptance is not
claimed; no desktop window or input automation is used.
