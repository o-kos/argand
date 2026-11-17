# Exact-commit release initiation

The release PR is the only version-selection input. It updates the workspace
version, inherited workspace package versions in Cargo.lock, and the matching
nonempty dated changelog section, leaving a fresh empty Unreleased section.
PR CI validates consistency without invoking Cargo.

## Authority and source identity

After merge, the administrator approves the **full 40-character merged main commit
SHA**, not a branch name or a prior PR head. Use that same SHA for preflight and
execution. Main can advance without substituting another commit. A stale branch
SHA is rejected because it is not in merged main history or has no canonical main
push CI. An approved merged commit stays the explicit target until the owner
chooses another SHA. Re-running a prior command does not discover a newer main.

Run locally with existing `gh` user/keyring authentication, without GH_TOKEN,
GITHUB_TOKEN or an Actions environment. The script reads repository permissions,
main protection and release-tag rulesets. It requires administrator privileges,
strict required CI enforced for administrators, and the existing admin-only v*
tag creation/update/deletion ruleset. It never changes those controls or installs
credentials. The git SSH identity must also be authorized to create the tag.

The script fetches merged main history into FETCH_HEAD without moving local
branches or tags. It reads manifest, workspace member manifests, lockfile and
changelog from the approved SHA. That commit must contain the guarded release
workflow. The latest canonical `.github/workflows/ci.yml` main push run for the
exact SHA must succeed. Its own check suite must satisfy all currently required
checks with their required app identities, including GitHub Actions `ci/full`.
A missing, running, cancelled or failed run/check cannot borrow a result from a
different SHA, suite, workflow or app.

## Tag creation and workflow handoff

```sh
python3 .github/scripts/initiate-release.py --commit <approved-full-sha>
python3 .github/scripts/initiate-release.py --commit <approved-full-sha> --execute
```

Preflight prints the derived tag, exact commit, successful CI run and operation.
Execution repeats validation, creates an annotated tag object and pushes it without
force directly to the remote tag ref. No local release tag is created or changed.
A conflicting local or remote tag fails. A concurrent remote tag creator wins;
this script cannot replace that ref. Any existing Release, including a draft,
fails before tag creation.

The existing administrator git push starts the tag-push Release workflow. This is
an ordinary authenticated push, not a GITHUB_TOKEN push subject to Actions'
recursive-trigger suppression. Release verifies the tag/version/changelog/lockfile
again, exact source CI, and administrator permissions for both the original actor
and the triggering actor. The publish job repeats the actor checks so a direct
publish-job rerun cannot skip them. Archive BUILD-INFO.json records the checked-out
source commit and binary version/hash. Existing equality checks remain in place.

## Interrupted retries

If the tag exists but no Release was created, inspect the completed failed run
before retrying. An active run must finish first. Retry only the same approved SHA:

```sh
python3 .github/scripts/initiate-release.py --commit <approved-full-sha> --retry
python3 .github/scripts/initiate-release.py --commit <approved-full-sha> --retry --execute
```

Retry requires a matching immutable remote tag, no active run for that tag and no
existing Release, including drafts. It uses the supported `gh workflow run
release.yml --ref v<version>` dispatch at the tag, so validation/build/publication
consume the same immutable source. Missing tags and conflicting tags fail.
Release runs serialize per tag without cancelling earlier runs. A write-scoped
publish check sees drafts too, and `gh release create` rejects a concurrent Release.
Retries never edit/delete existing Releases, assets or tags. Recovery of an
already published or draft Release requires separate administrator judgment.

## Validation boundaries

Policy tests use disposable Git repositories and mocked API responses to cover
consistent/inconsistent metadata, exact revision reads, canonical CI identity,
failed/missing/running CI, tag conflicts, explicit retries and draft/public release
refusal. They cannot prove that a remote GitHub event starts a workflow.

Operational handoff acceptance requires a non-production authenticated tag-push
and retry-dispatch run with approved SHA, tag, workflow run and artifact source
identity recorded together. Never publish a production release as a development
test. Where no isolated remote test repository is authorized, keep this acceptance
criterion explicitly pending rather than claiming that fixtures demonstrate it.
