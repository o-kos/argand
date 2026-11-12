---
name: argand-review-handoff
description: Prepare and assess an independent Argand review handoff for an exact revision and local changes, preserving neutral context and validation limits. Use when preparing or updating a required review, not as a substitute for review or owner acceptance.
---

# Argand review handoff

Follow the canonical [external review policy](../../../CONTRIBUTING.md#external-review)
and [scoped non-code workflow](../../../CONTRIBUTING.md#roles-and-workflow-scope).
The primary agent prepares this handoff without requiring the owner to name the
skill. It adds no approval, publication or review-waiver authority.

## Fix the review snapshot

Identify the task, agreed plan and acceptance criteria, applicable repository rules,
and the exact review base and head. Inspect staged, unstaged and untracked changes;
list which files and changes are included, and identify unrelated dirty work without
altering it. A head revision alone does not identify uncommitted work. Supply the
actual tracked patch and contents of included untracked files, including deletions
and renames where relevant; a plain Git diff omitting untracked files is incomplete.

Keep the review packet outside the visible worktree. Record a stable content identity
for the complete included snapshot, such as a digest of the exact patch and included
file contents, alongside base/head and the file manifest. Put revision identifiers in
the handoff/report, not commit hashes in the plan. Check that the packet still matches
the working tree when handing it off. If included content changes, invalidate that
snapshot and refresh the affected packet, review scope and evidence; do not silently
carry a clean conclusion onto a different diff.

## Provide independent context

Give the retained reviewer a neutral task, agreed decisions/criteria, exact snapshot,
repository rules and raw validation results with commands, revision/local-diff identity
and limits. Keep the handoff focused on these artifacts and relevant decisions, not
the whole conversation history. Require read-only review of that submitted state.
Exclude the implementing agent's discussion, self-assessment and predicted findings. Do not pre-label the patch safe
or ask the reviewer to confirm the implementing agent's conclusions.

Keep the same reviewer assignment under the canonical policy. For later rounds,
include previous findings and the primary agent's explicit dispositions: accepted with
resolution evidence, or rejected with reasons and evidence. Ask the reviewer to
challenge rejection reasoning; do not omit a declined finding from the next handoff.
The primary agent resolves technical disagreements; owner decisions and the round
limit remain governed by policy. A desk check is not independent review.

## Match evidence and report

Map current validation to the reviewed snapshot and acceptance criteria. Identify
unrun checks and results from earlier revisions. Refresh evidence when later changes
affect it. For actual UI claims, verify the production binary's current code/local
diff identity and required checks/build from recorded evidence. Automated or static
evidence does not prove native interaction. Use proportionate documentation/link/skill checks
for prose-only work under the scoped workflow, not an arbitrary extra gate.

Return the snapshot identity, findings/dispositions and current validation limits to
the primary agent for assessment. A clean reviewer report does not replace that
assessment or owner acceptance, waive required checks, or trigger commits, pushes,
Ready, merge or release actions.
