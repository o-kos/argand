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

- [x] Implement shared lifecycle and retain placement/action behavior.
- [x] Pointer regression asserts the same balloon ID, geometry and Close Button after leave/re-entry, transient leave, Select suppression, renewed automatic advice and constrained fallback.
- [x] Normal application check, formatting, all-targets Clippy and focused coverage.
- [x] Move finished plan before final independent review.
- [ ] Address independent review findings.
- [ ] Repeat the full locked gate on the final runtime.
- [ ] Rebuild the release after the final runtime passes the gate.
- [ ] Existing Draft PR publication after independent review.

## Review corrections

Accepted two narrow consistency/ownership observations: preserve the triangle's original absolute offset after extending the gap hit region, and stop deferred hover reconciliation when Select suppression has disabled it. Rendered pointer bounds verify the original three-pixel tip offset, one-pixel body overlap and unchanged geometry on reopening.

Accepted the Range lifetime finding: applying the recommendation removes advice while a fixed numeric range is shown, so callbacks cannot clear its old hover. Callers now explicitly deactivate absent advice while retaining the user's dismissal. Reactivation arms a fresh entry only when the pointer is outside the measured anchor; an anchor reappearing under the pointer waits for a real leave. A deterministic pointer regression failed before the correction and passes through Close, hover reopening, clicking the recommendation, leaving while fixed, Ctrl+R restoration and fresh re-entry.
