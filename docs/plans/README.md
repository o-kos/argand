# Implementation plans

Every implementation Issue has a versioned plan. Plans make scope, progress, decisions, and validation visible during review and preserve task-level history after the Pull Request is squash-merged.

## Locations

- `docs/plans/<issue>-<short-description>.md`: active plans.
- `docs/plans/completed/`: plans completed and accepted through a Pull Request.
- `docs/plans/TEMPLATE.md`: starting point for a new plan; it is not an active plan.

Use the GitHub Issue number as the filename prefix, for example `42-iq-frequency-axis.md`.

## Rules

1. Create and commit the plan before implementation.
2. Link the plan to its Issue with `Resolves #<issue>`.
3. Break the implementation into verifiable tasks.
4. Update checkboxes in the commits that complete the corresponding work whenever practical.
5. Add newly discovered work with `➕` and blockers with `⚠️`.
6. Keep decisions, rejected alternatives, and validation results current.
7. Do not record commit hashes in the plan.
8. Move the plan to `completed/` before final review and merge.

Follow the complete lifecycle in [`CONTRIBUTING.md`](../../CONTRIBUTING.md).
