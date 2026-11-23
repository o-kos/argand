# Issue #242: Tagged releases from main

Resolves #242 (https://github.com/o-kos/argand/issues/242), together with
[the Linux/export plan](259-linux-display-stack.md) in PR #267.

## Accepted scope — 2026-11-23

The owner accepted PR #267 and its reported limits, authorizing Ready, full CI and
protected squash merge. Class A; Coordinator/Executor remain `gpt-6.1-sol` medium
and independent Reviewer `gpt-6.1-sol` high under the current
[role policy](../../../AGENTS.md).

- [x] Keep tag pushes as the release trigger, without a separate release CLI,
  actor-role API checks, manual dispatch or custom retry orchestration.
- [x] Accept only canonical numeric `vMAJOR.MINOR.PATCH` tags, rejecting leading
  zeroes and suffixes. Verify exact tag/event/checkout identity and main ancestry.
- [x] Bind downstream build and publication checkout to the verified immutable
  source. Retain release notes, packaging and publication steps.
- [x] Complete local YAML/shell/disposable-Git policy checks and independent
  source review. The joint final local gate and normal pre-push hook passed.
- [x] Record owner acceptance and move this plan to completed. Current full CI
  remains a protected merge condition, not a claim about live release behavior.

## Evidence limits and post-completion

No production tag or release was created to validate publication. Live release
publication is separate owner-authorized work, governed by GitHub permissions,
existing rulesets and the release workflow. Remote protections were not changed.
The earlier administrator-only/nonproduction-handoff architecture was withdrawn;
it is not an uncompleted acceptance requirement. Broad distribution work is #86.

## Historical stages

The record below preserves the earlier transfer state and its then-pending
checks. Owner acceptance and the completed scope above supersede transfer stops;
unperformed live release checks are not represented as passed.

<details>
<summary>Historical implementation and transfer record</summary>

Tracks #242 (https://github.com/o-kos/argand/issues/242).

## Transfer handoff — stopped at the owner's request

The minimal tag-driven workflow and documentation correction are implemented.
Added release Python scripts/tests and their CI/local-CLI integration are removed.
The inline shell validates canonical numeric tags, exact checkout/tag/event commit
and main ancestry; build/publish checkout the verified SHA. No actor-role APIs,
manual dispatch or custom retry orchestration remain. Existing release notes,
packaging and publication steps are retained. Cheap YAML/shell/local-Git fixtures
passed; no real tag, release or dispatch was created.

The owner explicitly authorizes this transfer commit/push without further tests.
The joint DSP/test/packaging snapshot is unfinished: its latest default app suite
was OOM-killed at 1 GiB after 554/564 tests with 135 sampled threads. No accepted
full gate, final release build, current CI or final aggregate acceptance exists.
See the current handoff in the #259 plan before resuming on another computer.
Keep this plan active; do not publish a release or mark the PR Ready from this
transfer snapshot. No further work runs on this machine after the push.

## Current owner-directed scope

Class A: release source identity and publication safety. Coordinator and Executor
use `gpt-6.1-sol` medium; independent Reviewer uses `gpt-6.1-sol` high.
The owner withdrew the separate release-script architecture and local release CLI.
Restore the existing tag-driven workflow, adding only strict canonical numeric tag
validation and binding downstream checkout to the verified commit. GitHub permissions
and rulesets authorize actors; do not inspect roles or change remote settings.
Preserve all unrelated DSP, resource-test and packaging-withdrawal changes.

## Tasks and acceptance

- Remove the added release Python scripts/tests, CI integration and local CLI docs.
- Keep only tag pushes, not manual dispatch or custom retry orchestration.
- Accept only ASCII `v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)` tags.
- Verify the exact tag commit is reachable from fetched main, including ancestors.
- Build and publish from that verified immutable SHA. Preserve existing release
  notes, tests, packaging and publication behavior without adding API orchestration.
- Validate workflow YAML, inline shell and local disposable Git cases only.
- Obtain exact-snapshot independent review before any later submission.

## Validation limits

No publication, tag creation on GitHub or dispatch is authorized. The transfer
commit/push exception above is explicit owner authorization without further tests. Existing Rust evidence keeps its original
revision limits. The old administrator-only and nonproduction-handoff requirements
are withdrawn. Full current local gate/CI and owner acceptance remain pending.

</details>
