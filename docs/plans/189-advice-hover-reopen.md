# Issue #189: Reopen the same advice callout on hover

Continues Draft PR #224.

## Roles and risk

- Class B: local shared advice interaction and its rendered regressions.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, separate context.

## Contract and implementation

The earlier dismissal path deliberately used a stock Tooltip, so it did not reopen the yellow anchored callout. Share a small hover lifecycle in the existing advice composition instead: close hides immediately; a real leave and re-entry reopens the same balloon while room permits; the pointer can cross its gap and use its standard close Button. Leaving both anchor and balloon hides transient advice. A new automatic warning resets dismissal. Select suppression clears transient hover and prevents stale reopening. Only constrained viewports retain the stock Tooltip fallback. Range action and Ctrl+R remain unchanged.

Use GPUI hover handlers and deferred state reconciliation over the existing measured composition, not a replacement control. Record this retained interaction in the UI inventory. No native GUI launch or input emulation.

## Validation

- [ ] Implement shared lifecycle and retain placement/action behavior.
- [ ] Pointer regression asserts the same balloon ID, geometry and Close Button after leave/re-entry, transient leave, Select suppression, renewed automatic advice and constrained fallback.
- [ ] Normal application check, formatting, all-targets Clippy and focused coverage.
- [ ] Move finished plan before final independent review and address findings.
- [ ] Full locked gate, fresh release and existing Draft PR publication.
