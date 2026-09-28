# Issue #147: Revisit the implementation and review model policy

Resolves #147.

## Overview

"Agent roles and model selection" in `AGENTS.md` and "External review" in
`CONTRIBUTING.md` fix the `codex` CLI as implementer and reviewer and name
`gpt-5.6-*` models per class. Practice has moved on since #127: the implementer has
been chosen per Issue, reviewers have been agreed with the owner before each Pull
Request (#146 onwards), and the GPT-6 family has replaced the table's models. This
change writes the practiced policy into both documents and refreshes the model table.

Class C: documentation only, no code. Implementer: Claude in the session. Reviewer:
`gpt-6-luna` medium, proposed under the new table and agreed with the owner before the
first round.

## Context

Codex models listed by codex-cli 0.158.0 (`~/.codex/models_cache.json`):
`gpt-6-astra`, `gpt-6-sol`, `gpt-6-luna`, `gpt-5.6-sol`, `gpt-5.6-terra`,
`gpt-5.6-luna`, `gpt-5.5`.

Median total tokens per `codex exec` run in this repository, from
`~/.codex/sessions` up to 2026-09-28:

| Model and effort | Role | Runs | Median |
| --- | --- | --- | --- |
| `gpt-6-sol` high | review | 10 | 1.42M |
| `gpt-6-sol` medium | review | 2 | 0.67M |
| `gpt-5.6-terra` high | review | 22 | 1.01M |
| `gpt-5.6-sol` high | review | 188 | 1.00M |
| `gpt-5.6-sol` high | implementation | 15 | 2.32M |
| `gpt-6-astra` high | implementation | 4 | 1.71M |

Totals include cached input, which dominates every run; they measure work, not price.

## Decisions

Agreed with the owner on 2026-09-28.

- The implementer is chosen per Issue before the handoff and recorded in the plan:
  Claude in the session, the `codex` CLI with a named model and effort, or another
  agent the owner names.
- The class table proposes a reviewer; Claude proposes it before the first round,
  adjusted for the actual diff, and the owner confirms or replaces it. The agreed
  model and effort stay for every round of the Pull Request.
- Proposed reviewers: A `gpt-6-sol` high, B `gpt-6-sol` medium, C and changes without
  code `gpt-6-luna` medium. `gpt-6-astra` only on the owner's agreement. A Claude
  subagent of a model other than the implementer's is an alternative for any class.
- The reviewer never shares the implementer's model.
- Only the owner waives a review round, and the waiver is recorded in the Pull
  Request (as for #153).

## Rejected alternatives

- Reviewer fixed by class without agreement, as before #146: the owner wants to pick
  the reviewer from the actual diff.
- Agreement per review with no table: leaves every round without a starting proposal.
- `gpt-6-astra` for class A: `gpt-6-sol` high reviewed #129 and #130, and astra
  stays available on request when a change warrants it.
- A codex implementer column in the table: the implementer is chosen per Issue, so a
  default model per class would state a rule nobody follows.

## Implementation steps

- [x] Rewrite "Agent roles and model selection" in `AGENTS.md`.
- [x] Update the external-review bullets in "Git workflow" of `AGENTS.md`.
- [x] Rewrite "External review" in `CONTRIBUTING.md`: reviewer agreement, the `codex exec`
      invocation with an agreed effort, the Claude subagent alternative, and the owner
      summary.
- [x] External review of this change: one round with `gpt-6-luna` medium, run with the
      new `CONTRIBUTING.md` command, returned no findings.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` does not apply: no code changes, and no behaviour
      is shown to the owner.
- [x] Every model slug in the new text exists in the codex model cache.
- [x] No remaining rule in `AGENTS.md` or `CONTRIBUTING.md` names codex as the only
      implementer or reviewer.

## Post-completion

None.
