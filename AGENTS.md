# AGENTS.md — Argand

Argand is a cross-platform native signal editor and analyzer for recorded real and
complex I/Q signals, inspired by ocenaudio. Its focus is viewing, navigation,
selection-based editing and spectrum analysis; audio playback is not a goal.

Before repository work, read this file and [CONTRIBUTING.md](CONTRIBUTING.md), then
follow the task routing below. Apply these standing instructions without waiting
for the owner to repeat them. Keep current architecture documentation synchronized
when an invariant changes; completed plans remain historical records.

## Global invariants

- Support Linux, Windows and macOS; keep cold startup and first display fast.
- Real signals have a one-sided spectrum, 0..Fs/2; complex signals have a two-sided
  spectrum, -Fs/2..+Fs/2, with fftshift. I/Q uses interleaved f32 values I,Q,I,Q.
  One complex sample is an indivisible pair for editing and sample coordinates.
- Use the capture's true RF sample rate and physical hertz/seconds, including its
  reference frequency. Megahertz captures must never be treated as audio rates.
- Core, DSP, I/O and editing crates must not depend on GPUI; only the app converts
  toolkit-neutral view models into GUI resources. Keep expensive work off the UI
  thread and memory bounded for multi-hour captures.
- Preserve immutable capture/history ownership, source identity checks, safe staged
  publication and cancellation. Never weaken protection of open/read source files;
  read the detailed I/O and processing contracts before changing these paths.
- Only Shell owns textures; retirement waits through the documented intervening
  frame on the originating window. Generation/display-revision checks protect
  asynchronous delivery; do not bypass them.
- Take the compatible toolkit graph through the single gpui-kit dependency and the
  committed Cargo.lock. Follow dependency policy in CONTRIBUTING; do not introduce
  parallel toolkit dependencies or registry patches without the documented reason.
- Use only standard toolkit controls or existing shared project UI elements.
  Each new nonstandard widget needs separate owner approval before implementation;
  follow [UI component selection](CONTRIBUTING.md#ui-component-selection) and
  [UI acceptance](CONTRIBUTING.md#ui-acceptance).

## Language and communication

Direct conversation with the owner is in Russian. Repository content and external
project communication are in English: code comments, UI messages, logs, documentation,
branch and commit names, Issues, Pull Requests and releases. Never state or imply in
commits, Pull Requests, Issues or releases that an AI tool was used in development.

## Agent roles and model selection

The owner has approved these defaults; do not ask for model approval again for
each Issue. A different role assignment requires the owner's agreement.

| Role | Model and reasoning effort | Responsibility |
| --- | --- | --- |
| Coordinator | `gpt-6-astra` medium | Discuss analysis, architecture, plans, acceptance and disagreements directly with the owner; assess the final result. |
| Executor | `gpt-6.1-sol` medium | Perform repository investigation and operations, code and documentation edits, Git/GitHub operations, tests and builds. |
| Reviewer | `gpt-6.1-sol` high | Review read-only in a separate agent and context. |

The coordinator delegates investigation, edits, Git/GitHub operations, checks and
builds to the executor, including framework evidence before a plan relies on it.
The coordinator remains accountable for the plan, Issue and Pull Request text,
architecture, acceptance and final assessment. The executor maintains the plan and artifacts. The reviewer is
read-only in a separate context; the same model at different effort is intentional.
The independent handoff, dispositions, three-round limit, owner decisions and
current-revision evidence are governed by [External review](CONTRIBUTING.md#external-review).

Record the class and role assignments in a required plan before execution begins.
The class describes risk and the required scrutiny, not a different reviewer model:

| Class | Applies to |
| --- | --- |
| A | Concurrency and work scheduling, analysis generations, retention, caches, GPU texture lifetime, DSP correctness, public `argand-core` / `argand-dsp` API, security or data safety; or an expected diff above roughly 400 lines or 5 code files. |
| B | A feature or fix in one or two GUI modules with local, known invariants and concrete acceptance criteria. |
| C | Documentation, README, configuration, renames or single-file changes with no behavioural consequence. |

Any class A signal makes the work class A. File counts consider code only. A
configuration, CI or security-policy edit can affect behaviour without changing
code; classify its actual risk rather than assuming class C from its extension.

## Local Rust and build resources

Before any Rust command, dependency change or worktree operation, follow
[the local resource policy](CONTRIBUTING.md#local-rust-and-build-resources): one
owner-approved installed Rust, one agreed Argand/Wireroom toolkit graph, a common
intermediate build-dir outside worktrees, workspace-local final artifacts in
target-dir, and the prescribed worktree location. Stop on a missing or conflicting
setup; never auto-install a toolchain. Follow the policy's explicit owner approval
for initial dependency compilation and dependency rebuilds beyond the agreed
debug/test, release and check/Clippy baseline. The owner alone controls disk budgeting.

## Workflow and access

[CONTRIBUTING.md](CONTRIBUTING.md) is the canonical development workflow: scoped
non-code decisions, Issue/branch/plan, Draft/Ready, local gate and current full CI,
review, lint policy, safe merge/cleanup, changelog and release authority. Follow it;
a short routing file does not waive those requirements. Preserve unrelated changes.

Use gh for GitHub operations with the existing user/keyring authentication. If the
sandbox cannot access it, use the permitted normal-user execution path; do not start
an auth-login flow. The Agent Host GitHub MCP server is disabled; do not enable it
or request MCP authentication.

## Task routing

Read only the relevant detailed documents in addition to the two mandatory files.
They describe current contracts; Issue-number headings identify provenance, not
pending work. Inspect the current code before relying on a mutable implementation detail.

| Task | Read before changing it |
| --- | --- |
| Workspace, crate/API boundaries, config, session, delivery | [Architecture overview](docs/architecture/overview.md) |
| Analysis scheduling, generations, caches, minimap, GPU resources | [Analysis and rendering](docs/architecture/analysis-rendering.md) |
| Axes, navigation, orientation, gestures, selection and badges | [Navigation and selection](docs/architecture/navigation-selection.md) |
| Frame, Root/Shell/Plot, focus, overlays, controls, theme and settings | [UI architecture](docs/architecture/ui.md), [control inventory](docs/ui/124-inventory.md), [component selection policy](CONTRIBUTING.md#ui-component-selection) |
| UI acceptance planning, running-app scenarios and evidence | [UI acceptance policy](CONTRIBUTING.md#ui-acceptance), [project-local workflow](.agents/skills/argand-ui-acceptance/SKILL.md) |
| Readers, editing/history, clipboard, saving and source/file safety | [Editing and I/O](docs/architecture/editing-io.md) |
| Extraction, suppression, inversion, normalization and encoding semantics | [Signal processing](docs/architecture/processing.md), [Editing and I/O](docs/architecture/editing-io.md) |
| Independent review preparation, snapshot and findings handoff | [External review policy](CONTRIBUTING.md#external-review), [project-local handoff workflow](.agents/skills/argand-review-handoff/SKILL.md) |
| Implementation scope and progress | [Plan guide](docs/plans/README.md), active Issue plan and [roadmap](docs/plans/IMPLEMENTATION_PLAN.md) |

The detailed spectrum window remains a requirement; the plugin/worker design is
planned and deferred in the roadmap. Do not infer implemented support from a plan.
