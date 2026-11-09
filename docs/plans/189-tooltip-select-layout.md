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
Unsupported FLAC formats remain visible but stock-disabled; switching container
never silently changes the current format, Fs, reference or other draft values.

No new custom control or exception is introduced. Existing passive tooltip and
property-editor composition exceptions remain unchanged. No native GUI launch,
input emulation or portal is permitted; report automated layout coverage honestly.

## Work and validation

- [ ] Unify shared metadata tooltip typography and width measurement.
- [ ] Correct shared standard Select chrome sizing and viewport menu height.
- [ ] Give grouped sample rows concise labels and stock FLAC disabled states.
- [ ] Verify rendered width at multiple font sizes and selection/draft behavior.
- [ ] Check normal production compilation and the full formatting/Clippy/test gate.
- [ ] Move completed plan before final independent review; address findings.
- [ ] Rebuild current release after the gate and publish the existing Draft PR.
