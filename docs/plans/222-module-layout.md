# Issue #222: Align application files with module ownership

Resolves [#222](https://github.com/o-kos/argand/issues/222).

## Overview and roles

Finish the production-layout portion of #222 without changing behavior, logical
module ownership, visibility, public APIs, dependencies or the toolkit graph.
The owner requested starting this Issue after its two test phases had merged.

- Class A: mechanical changes across more than five code files, including files
  owning asynchronous work, texture lifetime and file safety.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, read-only in a separate context; at most three rounds.

## Existing evidence and remaining scope

Merged PR #237 extracted inline test bodies/support and removed repeated fixture
work. Merged PR #238 audited tests by product contract and retired low-value
layers. Their completed plans remain historical evidence, not work to repeat:
[extraction](completed/222-separate-test-files.md),
[audit](completed/222-remove-redundant-checks.md), and
[benchmark](completed/222-conversion-cost-benchmark.md).
Use the accepted audit dispositions and refresh discovery for this branch. No
additional test removal, consolidation, fixture optimization or runtime benchmark
is proposed. Remaining inconsistent unit-test filenames are normalized here.

The inspected module declarations establish real ownership: Shell has 21 child
modules; axes owns cursor guides and selection badges; analysis owns progress;
numbers owns platform locale; Shell's settings UI owns its editor. These owners
currently attach physically unrelated sibling files through production `#[path]`.
Other root modules deliberately provide shared toolkit-neutral services or
rendering adapters; their filenames alone do not establish a new hierarchy.

## Selected tree and decisions

Use ordinary Rust directory modules for existing owners, preserving logical paths:

```text
crates/app/src/
  main.rs
  analysis/
    mod.rs                 # existing analysis.rs
    progress.rs            # existing analysis_progress.rs
    analysis_test.rs, analysis_support_test.rs, progress_test.rs
  axes/
    mod.rs                 # existing axes.rs
    cursor_guides.rs, selection_badges.rs
    axes_test.rs, axes_support_test.rs, cursor_guides_test.rs, selection_badges_test.rs
  numbers/
    mod.rs                 # existing numbers.rs
    system.rs              # existing numeric_locale.rs; logical name already system
    numbers_test.rs, system_test.rs
  shell/
    mod.rs                 # existing shell.rs
    app_menu_ui.rs, shortcuts.rs, backdrop.rs, badge_entry.rs
    conversion_ui.rs, disk_change_ui.rs, editing_ui.rs, hints.rs, levelling_ui.rs
    navigation_ui.rs, plot_ui.rs, plot_view.rs, replace_ui.rs
    numeric_editor.rs, property_advice.rs, saving_ui.rs
    selection_gestures.rs, selection_panel.rs, statistics_ui.rs, suppression_ui.rs
    settings_ui/
      mod.rs               # existing settings_ui.rs
      editor.rs            # existing settings_editor.rs
      settings_ui_test.rs, settings_ui_support_test.rs
      editor_hint_test.rs, editor_support_test.rs
    ...existing child-owned *_test.rs and *_support_test.rs
    shell_analysis_hint_test.rs, shell_frame_observer_test.rs
  ...remaining existing root production modules and their sibling *_test.rs
```

Each test/support file moves with its production owner. Normalize remaining
`*_tests.rs` to `*_test.rs`; keep every logical test module identifier, test name,
cfg/platform/feature gate and private access. Move `conversion/tests.rs` to the
sibling `conversion_test.rs` without changing the logical `conversion::tests`.
Replace include-only test module bodies with cfg-gated sibling module declarations
where equivalent. Retain cfg-only support includes where original-scope associated
implementations or instrumentation require them; document those concrete exceptions.
Keep plot-view nested context-menu test scope and original fixture lifetimes.

`#[path]` remains appropriate for descriptive sibling test filenames that differ
from logical module identifiers. Remove production path redirections made obsolete
by the ordinary directories. Review include paths and rustdoc links individually,
especially analysis's FLAC fixture and config's distributed default TOML.

Do not group toolkit-neutral workers under Shell merely to place similarly named
model/UI files together. Extraction, suppression, conversion and levelling share
saving job/source services and document ownership across multiple consumers;
changing their logical parents would introduce a different architectural refactor.
Likewise, shared settings/navigation models remain independent of their GUI views.
The GPUI Kit guide's capability-crate recommendation does not justify creating
new crates or moving feature ownership in this strictly mechanical Issue.

## Rejected alternatives

- Generic `ui/`, `models/` or `views/` directories scatter capability ownership.
- New prefix-based processing/settings/selection namespaces require new logical
  boundaries, import/visibility changes and invented ownership beyond this scope.
- One directory per leaf module adds artificial nesting without owned children.
- Renaming every `_ui` module or splitting large files does not improve this
  ownership mapping and increases the non-mechanical review surface.
- Repeating #238's audit or reducing test counts adds no evidence for file moves.

## Implementation steps

- [x] Inspect current Issue, clean up-to-date base, existing completed phases,
  module declarations, relevant architecture and GPUI Kit Coding Guides.
- [x] Verify installed-toolchain selection and shared build configuration without
  invoking a Rust proxy or compiling dependencies.
- [ ] Commit this plan and open an early Draft PR before implementation.
- [ ] Record old-to-new file/module/test/include mapping and current executable
  discovery, including platform-gated source declarations, outside the worktree.
- [ ] Move production children and their tests/support to the selected owner
  directories; preserve all function bodies, ownership and logical paths.
- [ ] Normalize remaining app sibling test filenames and equivalent test wiring.
- [ ] Verify content/token equivalence with an explicit path/wiring-only allowlist;
  compare exact executable test-name and ignored sets, not just counts.
- [ ] Update current architecture, UI inventory and AGENTS routing references to
  resulting paths; leave completed historical plans unchanged.
- [ ] Complete required local validation and exact-revision independent review;
  resolve findings within three rounds and obtain coordinator assessment.
- [ ] Move this plan to completed and request current-head full CI through Ready.

## Resources and validation

Read-only normal-user inspection found one installed Rust toolchain 1.98.1 matching
the repository pin/default, with rustup auto-install disabled. Use its direct
Cargo/Rustc/Rustdoc executables; do not change PATH, pins, defaults or toolchains.
The configured common build-dir is `/home/xokc/.cache/cargo-target`; preserve it and
keep final outputs in this workspace's local `target/`. Completed #237/#238 plans
record owner approval of debug/test, release and check/Clippy dependency baselines.
Use the existing external compilation guard without enabling its optional external
dependency allowance. Stop and explain any requested dependency rebuild before it
occurs. Configuration evidence alone does not establish artifact reuse.

The initial plan-only commit changes no executable behavior; inspect its diff and
links, omit Rust checks, and skip the pre-push hook for that documentation-only
publication. Subsequent code pushes require the full unchanged gate and hook.
No new worktree, cache, dependencies, features, flags, profiles or benchmarks.

| Criterion | Verification | Result / limits |
| --- | --- | --- |
| Physical tree matches actual ownership | Per-owner old/new mapping, no obsolete production path overrides | Planned |
| Production behavior/API/lifetimes unchanged | Exact logical paths, visibility and body/token equivalence; independent review | Pending |
| Existing audit coverage preserved | Accepted #237/#238 dispositions plus exact before/after discovery, ignored and platform source sets | Prior audit accepted; branch evidence pending |
| Toolkit-neutral boundary preserved | Same imports/dependency graph and ownership; worker/model files stay independent | Planned |
| Current docs and routing match tree | Current-path/link searches; no historical plan rewrites | Pending |
| Cross-platform validation | Local gate and current Ready Linux/Windows/macOS ci/full | Not run |

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, last after the checks above
- [ ] Exact-revision independent review and coordinator assessment
- [ ] Current-head Linux, Windows, macOS and `ci/full` explicitly successful

This mechanical layout change has no intended UI or DSP semantic change. No native
UI acceptance, performance improvement or server-CI success is inferred from
source equivalence or a test count. If inspection reveals a behavior change is
needed, report it and apply the existing triage/owner-decision policy first.

## Post-completion

After current full CI and review pass, complete the protected PR merge/conditional
branch cleanup under owner authorization. Preserve unrelated refs, stashes and
worktrees; no cleanup of earlier accepted branches is part of this Issue.
