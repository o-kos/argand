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
| Primary | `gpt-6.1-sol` medium | Discuss the task directly with the owner; handle analysis, investigation, edits, Git/GitHub operations, checks, plans and final assessment. |
| Reviewer | `gpt-6.1-sol` high | Review read-only in a fresh, separate context when review is required. |
| Consultant | `gpt-6-astra` medium | Give targeted advice on difficult architecture, DSP, data safety or unresolved technical disagreements. |

One primary agent owns an ordinary task end to end. Delegate only justified,
independent work; do not create a mandatory coordinator/executor relay. The primary
maintains the plan and artifacts, evaluates review findings and checks critical
invariants. The primary decides whether consultation is needed after focused
investigation leaves a concrete uncertainty or technical disagreement unresolved.
Before calling the consultant, briefly tell the owner what remains unclear and
the specific question to be asked. No extra approval is needed under these defaults
unless the owner requests it. Class A alone does not trigger consultation, which
does not replace independent review or owner acceptance.
The independent handoff, dispositions, three-round limit, owner decisions and
current-revision evidence are governed by [External review](CONTRIBUTING.md#external-review).

These instructions define role defaults, not the client's selected model. Select
the primary model and effort in the client when starting a task; do not claim that
editing these rules switches a running session or global settings.

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
owner-approved installed Rust, one agreed Argand/Wireroom toolkit graph,
workspace-local intermediate and final artifacts in `target/`, a separately
installed shared SDK, and the prescribed worktree location. Stop on a missing or conflicting
setup; never auto-install a toolchain. Initial dependency compilation and ordinary
rebuilds within the agreed setup do not require separate owner approval.
The owner alone controls disk budgeting.

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

Application source directories follow subsystem responsibilities; see
[Application module layout](docs/architecture/overview.md#application-module-layout).
Keep sibling `*_test.rs` files and test support beside their production owner.
