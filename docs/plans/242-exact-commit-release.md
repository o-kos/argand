# Issue #242: Tagged releases from main

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
