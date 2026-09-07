# Issue #47: Specify reviewer selection for external review

Resolves [#47](https://github.com/o-kos/argand/issues/47).

## Overview

Record the agreed reviewer selection in the contributor and agent instructions.
This changes only the repository workflow documentation.

## Context

The policy was kept separate from #42 to preserve that Pull Request's scope.
`AGENTS.md` summarizes the workflow; `CONTRIBUTING.md` owns its detailed procedure.

## Decisions

- Claude implementations require Codex GPT-6 Astra at High reasoning effort.
- Codex GPT-6 Astra implementations require Codex GPT-5.6 Sol at High reasoning effort.
- Keep the selected reviewer model and effort for every subsequent round.
- Make model and effort explicit in the read-only CLI example.

## Rejected alternatives

- Relying on CLI defaults would leave the reviewer selection machine-dependent.
- Adding a changelog entry would describe internal workflow as application behaviour.

## Implementation steps

- [ ] Synchronize the policy in `AGENTS.md` and `CONTRIBUTING.md`.
- [ ] Update the CLI example to select the reviewer and effort explicitly.
- [ ] Complete standard validation and external review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Check both mappings, consistency between the documents, and CLI syntax.
- [ ] External review returns no substantive findings.

## Post-completion

Squash-merge after required CI checks pass, then delete the accepted branch locally
and remotely according to `CONTRIBUTING.md`.
