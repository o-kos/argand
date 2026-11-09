# Issue #227: Consolidate project guidance and acceptance workflows

Resolves [#227](https://github.com/o-kos/argand/issues/227).

## Overview and authorization

Consolidate current project guidance without changing production behavior, then
make acceptance and review workflows reproducible. The owner requires discussion
of every step before implementation and a report afterwards. Step 1 was accepted after its report, and the owner authorized step 2, including
the stronger UI-control and running-application acceptance rules. After accepting
step 2, the owner authorized trying the project-local skill in step 3. After accepting
step 3, the owner approved the review-handoff skill and final consistency work in step 4.

The initial owner-requested separate worktree used the then-current main. During
steps 1–4, #189 conversion was unmerged and was deliberately not imported. By
Draft publication, #224 had merged that work; after publication the owner authorized
the integration phase below to preserve the now-upstream architecture while resolving
conflicts.
Keep historical completed plans and the existing UI inventory unchanged. The coordinator defers commits, pushes and a Draft Pull Request until the
after-step discussion; this does not waive the eventual delivery workflow.

## Roles and risk

- Class C for step 1: architecture and policy relocation, with no intended change
  to product behavior, permissions, required gates or approval boundaries.
- Class C for step 2: an owner-approved instruction change affecting the future
  UI workflow, with targeted policy review; no runtime, security or gate relaxation.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate read-only context.

Independent review is warranted because this is an extensive relocation of
standing policy. Before later steps, assess the actual instruction/skill risk
under the existing non-code workflow rather than assuming that all prose is low risk.

## Decisions

- Keep global invariants, language, canonical roles/risk and task routing in
  [AGENTS.md](../../../AGENTS.md).
- Keep the complete development workflow, gate, review, lint policy, release
  authority and custom-control approval in [CONTRIBUTING.md](../../../CONTRIBUTING.md).
  The short root file points to it without introducing additional approval gates.
- Move detailed current architecture into six thematic documents; keep the
  [toolkit exception inventory](../../ui/124-inventory.md) as the detailed control audit.
- Preserve requirements during relocation. Correct demonstrated stale descriptions
  against current main code; do not copy future feature behavior into this branch.
- No skills, new UI-reuse requirements or plan-template changes belong to step 1.

## Coverage map for step 1

| Previous AGENTS section or detail | Canonical destination |
| --- | --- |
| Purpose, stack, requirements, workspace/crate responsibilities | [Overview](../../architecture/overview.md); concise global invariants in AGENTS |
| Rendering boundary, performance, analysis delivery, FFT policy, minimap scan, texture retirement | [Analysis and rendering](../../architecture/analysis-rendering.md) |
| Time/frequency navigation, ruler modes, selection/badges/entry, grid, orientation, waveform/axis layout | [Navigation and selection](../../architecture/navigation-selection.md) |
| UI structure, window composition, status/settings, theme, Plot ownership, overlays, menus/toolbar, standard buttons | [UI architecture](../../architecture/ui.md) |
| Saving, editing/history, source stamps, replacement and changed-on-disk | [Editing and I/O](../../architecture/editing-io.md) |
| Band/rectangle extraction, suppression, inversion/normalization and computed encoding | [Signal processing](../../architecture/processing.md) |
| Config defaults, session merging, assets/identity, platform restore limits, deferred roadmap | Overview |
| Build/dependency/error/comment/branching conventions | CONTRIBUTING engineering/dependency section |
| Full gate, scoped non-code workflow, Git/CI/backlog/review/merge/release and custom-control approvals | Existing CONTRIBUTING lifecycle sections; references replace duplicated AGENTS detail |
| Language, communication, role assignments and A/B/C risk classification | AGENTS |
| Existing gh/keyring and disabled Agent Host GitHub MCP standing access rules | AGENTS workflow/access summary |

## Four implementation steps

- [x] Step 1: consolidate documentation and correct verified stale current descriptions.
  Owner approved the six destinations and canonical ownership before the work.
  The owner accepted the reported step 1 result after the clean independent review.
- [x] Step 2: implement the owner-approved shared UI reuse and strengthened control/
  acceptance rules, plus conditional DSP examples and criterion-to-evidence mapping
  in the plan template. The owner accepted the reported result after round 2 review.
- [x] Step 3: add the project-local UI acceptance skill and exercise three desk-check
  scenarios; the owner accepted the reported project-local result.
- [x] Step 4: add and exercise the owner-approved review handoff skill and perform
  final consistency validation. Final independent review and owner discussion remain
  pending below; implementation completion does not claim acceptance.

## Step 1 semantic corrections and evidence

- Frequency navigation is implemented: `PlotGeometry::scroll` in
  [navigation_ui.rs](../../../crates/app/src/navigation_ui.rs) routes Shift/ruler wheel
  events to frequency pan/zoom; ruler gesture and cursor tests cover the available
  pan range. Remove descriptions saying the frequency ruler is still reserved for #80.
- The current session supports the versions declared by `VERSION` and `READABLE` in
  [session.rs](../../../crates/app/src/session.rs), currently 11 and 1 through 11.
  Reference these symbols instead of retaining an obsolete current-status version 9.
  Feature-introduction version notes remain history, not claims about today's layout.
- Selection readouts are badges, not status-bar selection fields:
  `Shell::band_badge` in [plot_ui.rs](../../../crates/app/src/plot_ui.rs) uses
  `Scale::band_texts`, while `Shell::status_bar` in
  [settings_ui.rs](../../../crates/app/src/settings_ui.rs) contains no band readout.
- `ProgressiveOptions::frames` in [progressive.rs](../../../crates/dsp/src/progressive.rs)
  bounds decoded input to 2^20 f32 values: 4 MiB, allowing one oversized FFT.
  Correct the stale 1 MiB summary to agree with the detailed policy and implementation.
- Cursors, selection and scrolling are implemented; the detailed spectrum window
  remains planned. Remove the old future-tense grouping and ambiguous binary count
  from the workspace summary without adding new product goals.

## Validation for step 1

- [x] Verify all original detailed architecture sections and current-status blocks
  have a destination; inspect remaining rewritten/de-duplicated policy separately.
- [x] Check local Markdown links and heading anchors in changed documents.
- [x] Check the complete diff for whitespace errors and unintended code changes.
- [x] Verify historical completed plans and the UI inventory are unchanged.
- [x] Independent read-only review of policy preservation and the exact local diff.
  Final review was clean after restoring the missing Recent files paragraph and
  correcting architecture cross-links. It verified all 18 current-status blocks
  and 55 local links/anchors. The reviewed snapshot preceded only these plan
  metadata updates: attribution of the Git deferral to the coordinator and recording
  the completed review. No architecture or policy text changed afterwards.
- [x] Report step 1 to the owner and discuss the next step before implementing it.

No Cargo checks or release build are warranted for this prose-only relocation;
no application behavior is demonstrated. No native UI acceptance is claimed.
The hook, required remote checks and protected-main merge requirements are unchanged.
Later skill validation is outside this authorized step and will be scoped beforehand.

## Step 2 scope, decisions and validation

Step 2 deliberately strengthens future UI acceptance and component-selection policy,
so it is not merely a wording correction or an automatic class C exemption. It is a
scoped non-code policy change with explicit owner agreement and targeted independent
review. No runtime behavior, security boundary, local gate, remote checks or release
requirements change or become weaker. The approved role assignments remain unchanged.

- CONTRIBUTING is canonical for using only standard controls or existing shared UI
  elements, per-widget approval before new nonstandard implementation, shared
  behavior/style reuse and affected-consumer checks. Extending a shared element
  cannot hide a custom replacement. Domain drawing remains the existing exception.
- UI acceptance now requires the current running production binary and the affected
  appearance/interaction scenario. An inability to run or interact leaves acceptance
  incomplete and the Pull Request in Draft, with evidence limits reported.
- A common-root-cause rule covers related formats and action paths while retaining
  existing scope and owner-decision boundaries.
- The template adds criterion-to-verification/result/limits mapping and conditional
  numerical DSP examples, including save/reopen; test count alone establishes no
  acceptance claim. The concrete cosine example changes no production behavior.
- AGENTS contains only a short aligned control rule and canonical links. No skills,
  production code, historical completed plans or UI inventory are changed.

Validation is scoped to policy consistency, local links/anchors, the exact step 2
prose delta and whitespace checks; no Cargo or native application run is warranted
because this step changes instructions, not the application. No UI acceptance claim
is made about the unchanged product by this documentation work.

- [x] Capture the pre-step-2 text and exact delta outside the worktree for review.
- [x] Verify changed local links/anchors and whitespace; confirm the changed scope.
- [x] Targeted independent review of the exact step 2 delta (round 2 of at most 3).
  The final review found no substantive issues; 35 local links/anchors and the
  whitespace diff check passed. Before review, the coordinator clarified the DSP
  example's -0.1 dBFS target and Save As to a new float32 WAVE path preserving the
  original source. The checked snapshot preceded only this review-record metadata
  update; the policy and template text are unchanged afterwards.
- [x] Report step 2 and discuss step 3 with the owner before creating any skills.

The step 1 review was round 1. Reserve round 3 for the final complete skill/guidance
change; if substantive findings would require another round, reconsider the task
and plan with the coordinator rather than silently exceeding the limit.

## Step 3 scope and evidence

The owner approved a project-local skill for this repository, not a global skill
installation. The single [SKILL.md](../../../.agents/skills/argand-ui-acceptance/SKILL.md)
uses normal implicit discovery and no extra scripts, UI metadata or global settings.
The coordinator plans acceptance, the executor follows the skill and the reviewer
assesses evidence; the owner does not need to invoke the skill by name. Short
CONTRIBUTING/AGENTS links connect this workflow to the canonical policy.

This is a scoped class C instruction change affecting future workflow, with no
runtime/code or authority/gate relaxation. The skill requires current production
build identity, verified isolation, actual affected scenarios and evidence limits;
it claims no native access and does not authorize denied input or external actions.
No GUI was launched for this instruction-only step. The following are **simulated
desk checks of instructions**, not actual Argand acceptance or independent review:

| Simulated input | Expected workflow outcome | Desk-check result |
| --- | --- | --- |
| Current build and isolated data are available; actual pointer/keyboard scenarios and comparison are supplied as passing observations for both affected panels | Record criterion-level native evidence and exact environment/build; no claim beyond those observations | The instructions lead to passed supplied criteria, retaining limits and no automatic publication |
| Supplied observation: Escape dismisses a Select but also closes a panel whose agreed transaction requires it to remain open | Keep the agreed expectation, report failure/reproducer, inspect affected consumers and route the defect in scope | The instructions lead to a failed criterion rather than redefining success or marking Ready |
| Only screenshots/headless checks are available; required input is denied | Do not inject input by another route or infer interaction success; mark the criterion incomplete and retain Draft | The instructions lead to incomplete acceptance and explicit remaining verification |

- [x] Validate the skill frontmatter/name with the skill-creator validator.
- [x] Check local links/anchors, exact step 3 scope and whitespace.
- [x] Perform the three simulated desk checks above with explicit evidence limits.
- [x] Report step 3 and discuss step 4 with the owner before implementation.

Final external review remains reserved for round 3 after step 4; these desk checks
are executor validation and do not consume or replace that independent review.

## Step 4 scope and evidence

The owner accepted step 3 and approved this final project-local workflow. The single
[review handoff skill](../../../.agents/skills/argand-review-handoff/SKILL.md) packages
base/head, included staged/unstaged/untracked content, a stable snapshot identity,
neutral task and raw evidence. CONTRIBUTING remains canonical for review policy;
its short mandatory linkage and AGENTS routing make the skill part of preparation.
The coordinator records finding dispositions, while the retained reviewer challenges
rejected reasoning. No new gate, waiver or publication authority is introduced.

This is a scoped class C instruction change affecting future review preparation,
with no runtime, security, data-safety or mandatory-gate relaxation. Both skills
remain local to this repository. The plan moves to completed before final review
as required; that location does not mean final review or owner acceptance has occurred.
No existing historical completed plan or UI inventory is modified.

The following are **simulated desk checks**, not independent review or actual Argand
acceptance; no GUI was launched for instruction-only work:

| Simulated input | Expected workflow outcome | Desk-check result |
| --- | --- | --- |
| A supplied clean review identifies an earlier patch; an included file subsequently changes | Invalidate that snapshot and refresh affected review/evidence before claiming the new result | The instructions reject carrying the earlier clean conclusion onto the changed diff |
| Supplied status contains staged and unstaged task changes, a new untracked skill and unrelated dirty work | Include the task patch and untracked contents in the manifest/digest; identify and preserve unrelated work | The handoff covers the actual submitted state rather than HEAD alone without altering unrelated work |
| A finding was rejected by the coordinator with evidence | Include the finding, rejection and evidence in the next neutral handoff; request a challenge to the rejection | The finding remains visible for independent scrutiny; no automatic waiver or merge follows |

- [x] Validate both local skills with the skill-creator validator.
- [x] Check local links/anchors across all changed documentation and whitespace/scope.
- [x] Recheck coverage of all 18 original current-status blocks.
- [x] Perform the three simulated handoff desk checks above.
- [x] Move this plan to completed before final review and fix relative links.
- [x] Final independent read-only review of the complete frozen snapshot (round 3).
  The final round was clean: all 12 files, 18 original current-status blocks and
  72 local links/anchors were checked; both skill validators and the whitespace
  diff check passed. Earlier accepted corrections restored the missing Recent
  files block, fixed cross-links and clarified the DSP target and new Save As
  path. No findings were rejected. The reviewed snapshot predates only this
  review-result, coordinator-assessment and owner-acceptance metadata updates at
  initial publication. That review does not cover the later upstream integration
  recorded below.
- [x] Coordinator assessment of the exact step 4 delta and validation evidence;
  implementation is consistent with the approved scope.
- [x] Report the result and complete the after-step discussion with the owner.
  The owner accepted the final result and authorized normal delivery as a Draft
  Pull Request; moving to Ready or merging is outside this delivery step.

## Upstream integration after Draft publication

After the owner accepted the four steps, Draft PR #228 conflicted with the newly
merged conversion work from #224. The owner authorized resolving the conflicts and
updating the Draft. Rebase onto current main; preserve upstream source, inventory
and completed plans, adapting only this task's documentation split. The initial
no-import decision above describes the original base, not an instruction to discard
subsequently merged behavior. The stronger approved UI rules and both local skills
remain unchanged; no new product semantics or approval rules are introduced.

The upstream AGENTS delta contains one revised Current status block and three new
Capture conversion paragraphs. Preserve each text fragment at these destinations:

| Upstream paragraph/fragment | Destination |
| --- | --- |
| Revised status: whole-capture Peak/RMS, bounded source statistics, shared numeric editor and distinct panel transactions | [UI architecture and ownership](../../architecture/ui.md#ui-architecture-and-ownership) |
| Conversion paragraph 1: whole-capture operation, standard panel composition, ownership/defaults/draft transaction | [File information and conversion](../../architecture/ui.md#file-information-and-conversion-189) |
| Conversion paragraph 1: input-local bounds, reconstruction, anti-aliasing and identity copy | [Capture conversion](../../architecture/processing.md#capture-conversion-189) |
| Conversion paragraph 2: Preserve and internal Normalize amplitude policy | Capture conversion |
| Conversion paragraph 2: EncodedWave, safe publication/adoption and container fallback | [Conversion publication and saving](../../architecture/editing-io.md#conversion-publication-and-saving-189) |
| Conversion paragraph 2: shared property advice and Select/hover behavior | File information and conversion |
| Conversion paragraph 2: first Save, scratch/backing ownership, cancellation and normalization-disabled opening | Conversion publication and saving |
| Conversion paragraph 3: carrier/baseband metadata, input-local bounds and restoring drafts | Capture conversion |
| Conversion paragraph 3: disabled shared NumericEditor traversal | File information and conversion |

The earlier three independent review rounds apply to the pre-integration snapshot.
They are not evidence of independent review of this rebase. Do not start a fourth
round automatically; the coordinator assesses the exact integration delta and any
need for further owner discussion. Native UI acceptance is not claimed.

- [x] Verify clean local/remote PR identity and report upstream delta before edits.
- [x] Preserve every newly merged guidance fragment with the mapping above.
- [x] Validate documentation, both skills and exact source equality with main.
  Both validators passed; all 82 local links/anchors and whitespace checks passed.
  All 18 current-status blocks and all nine new upstream fragments have destinations;
  runtime source, upstream inventory and other completed plans match main exactly.
- [x] Coordinator assessment of integration delta and validation evidence.
  The exact semantic delta and fragment mapping preserve upstream text within the
  approved documentation split; no product decision or runtime change is introduced.
  The prior independent review remains historical. No fourth round is required for
  this scoped verbatim documentation synchronization; this is not a code-review waiver.
- [x] Update the Draft branch with an explicit lease after current full hook checks.
  Report the integration and current review/validation boundaries in the Draft.
- [ ] Complete the after-integration discussion with the owner.

## Post-completion

After the full authorized work, use the ordinary Draft/Ready and protected-main
workflow. Any post-merge branch cleanup follows CONTRIBUTING. No post-merge action
is authorized by the current local step approval alone.
