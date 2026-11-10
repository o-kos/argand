# Issue #222: Align application files with module ownership

Resolves [#222](https://github.com/o-kos/argand/issues/222).

## Overview and roles

Finish the production-layout portion of #222 without changing behavior, effective
visibility, public APIs, dependencies or the toolkit graph. Internal module
namespaces/imports may change to represent coherent subsystems.
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

## Revised subsystem layout and decisions

The first mechanical owner-directory stage alone left 69 root Rust files and the
same 37 root module declarations. It made existing parent-child ownership visible
but did not adequately group related root services or the large Shell plot cluster.
The owner requested continuing. This revision supersedes the earlier categorical
rejection of new internal namespaces; #222 explicitly permits import updates.

```text
src/
  processing/
    mod.rs
    conversion.rs, extraction.rs, suppression.rs, levelling.rs, saving.rs
    ...sibling tests and support
  navigation/
    mod.rs                    # existing time-navigation model
    frequency.rs, orientation.rs, time_ruler.rs, selection_entry.rs
    ...sibling tests and support
  shell/
    mod.rs
    plot/
      mod.rs                  # subsystem declarations, no compatibility facade
      plot_view.rs, plot_ui.rs, navigation_ui.rs
      selection_gestures.rs, selection_panel.rs
      ...sibling tests and support
    settings_ui/{mod.rs,editor.rs,...sibling tests and support}
    ...remaining Shell-owned operation views/shared controls
  analysis/{mod.rs,progress.rs,execution.rs,cpu.rs,...sibling tests and support}
  axes/{mod.rs,cursor_guides.rs,selection_badges.rs,...sibling tests and support}
  numbers/{mod.rs,system.rs,...sibling tests and support}
  ...explicitly retained root services below
```

Processing groups actual computed-capture/publication workflows: conversion uses
extraction and levelling, suppression/levelling share saving Job/Update and source
helpers, and saving coordinates conversion/extraction. These remain toolkit-neutral.
Consumers import `crate::processing::{...}`; no old root reexports hide the new tree.

Navigation groups physical/sample viewport models, orientation, ruler coordinates
and typed selection entry. Selection entry owns Field/Scale parsing and formatting,
uses numbers/time-ruler units, and is consumed by plot/selection UI; it does not
perform history edits. Models remain independent of GPUI; views remain in Shell.

Shell's plot group owns the retained plot entity, snapshot/drawing, navigation
input, selection gestures and selection overlay. Existing Shell/PlotView impls,
state, actions, focus and texture ownership do not change. Plot child modules use
explicit ancestor imports for Shell context, rather than reexporting Shell through
a compatibility facade. Visibility originally restricted to Shell remains exactly
`pub(in crate::shell)` where one extra nesting level requires spelling the original
boundary; no `pub(crate)` widening is authorized for those GUI internals.

Remaining root responsibilities are intentional: document/open_file/editing and
source-change state have multiple analysis/UI/processing consumers; minimap and
sample_statistics provide independently owned full-capture tasks/caches; scratch
is shared by editing and processing. Execution/cpu belong under analysis: execution owns the per-document compute
pool/batch/affinity policy, analysis is its only runtime consumer, and cpu is used
only by execution. Config stores `analysis::execution::Settings`; the setting and
serialized configuration remain unchanged. Config/session/settings/recent remain distinct
configuration, shared persistence, effective analysis settings and history-menu
contracts. Bootstrap/assets/theme/chrome/profiling/release are shared application,
platform or resource services. No requirement eliminates every root leaf module.

Unit tests stay beside their owner in sibling `*_test.rs`; no test removal,
consolidation, algorithm/fixture change or integration-test conversion. Preserve
case names, gates, private access and fixture lifetimes. Fully qualified names
intentionally change with subsystem namespaces: record an explicit old/new module
mapping and compare discovery through that mapping. Source-only checks are not
compiled-discovery evidence. Preserve cfg-only associated support includes and the
plot context-menu nested test scope. Review include paths, source-location macros,
module visibility and Windows/macOS cfg branches explicitly.

## Rejected alternatives

- Generic ui/models/views dumping grounds scatter capability ownership.
- Keeping all logical root namespaces through path overrides or compatibility
  reexports merely hides the flat tree and defeats ordinary Rust module layout.
- Moving neutral models beneath GPUI views would weaken the toolkit boundary.
- Splitting production algorithms/types, creating feature crates or changing
  ownership/lifetimes is unnecessary for this physical/module organization task.
- One directory per leaf or renaming all UI files adds churn without a boundary.
- Repeating #238 test pruning or using a target count provides no layout evidence.

## Implementation steps

- [x] Inspect current Issue, clean up-to-date base, existing completed phases,
  module declarations, relevant architecture and GPUI Kit Coding Guides.
- [x] Verify installed-toolchain selection and shared build configuration without
  invoking a Rust proxy or compiling dependencies.
- [x] Commit this plan and open early Draft PR #239 before implementation.
- [ ] Record old-to-new file/module/test/include mapping and current executable
  discovery, including platform-gated source declarations, outside the worktree.
- [x] Complete revised production subsystems and move their tests/support to the selected owner
  directories; preserve behavior, ownership and effective access through mapped module paths.
- [x] Normalize remaining app sibling test filenames and equivalent test wiring.
- [ ] Verify content/token equivalence with an explicit path/wiring-only allowlist;
  compare executable test-name and ignored sets through the explicit namespace
  mapping, not just counts.
- [x] Update current architecture, UI inventory and AGENTS routing references to
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
| Production behavior/API/lifetimes unchanged | Mapped logical paths, effective visibility and body/token equivalence; independent review | Pending |
| Existing audit coverage preserved | Accepted #237/#238 dispositions plus mapped before/after discovery, ignored and platform source sets | Prior audit accepted; branch evidence pending |
| Toolkit-neutral boundary preserved | Same toolkit boundary/Cargo graph and runtime ownership; worker/model files stay independent | Planned |
| Current docs and routing match tree | Current-path/link searches; no historical plan rewrites | Pending |
| Cross-platform validation | Local gate and current Ready Linux/Windows/macOS ci/full | Not run |

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, last after the checks above
- [ ] Exact-revision independent review and coordinator assessment
- [ ] Current-head Linux, Windows, macOS and `ci/full` explicitly successful

This mechanical layout change has no intended UI or DSP semantic change. No native
UI acceptance, performance improvement or server-CI success is inferred from
source equivalence or a test count. If inspection reveals a behavior change is
needed, report it and apply the existing triage/owner-decision policy first.

## Current evidence and validation

The revised tree retains all 135 original application Rust files and adds only two
module declaration files (processing and Shell plot). A source module-declaration
inventory maps all 122 old nodes to 124 new nodes through explicit namespace/file
mapping. All original cfg/test attribute lines remain unchanged per mapped file;
all 75 literal module/include references resolve. These are focused source checks,
not a full Rust parser, name-resolution check or compiled test discovery.

Standalone installed rustfmt parsed/formatted the workspace and formatting check
passed without compilation or shared-cache use. A narrow lexical audit verifies
113 original files after path/import/module/visibility normalization; 22 additional
files have reviewed rustfmt punctuation/closure wrapping changes: replacing
include-only test wrappers by ordinary module declarations exposes previously
include-hidden test files to cargo fmt traversal. These existing files therefore
receive standard formatting, so strict whole-
tree token equality is not claimed. Review the complete rename-aware patch and
mapped baseline, including relative imports and effective Shell visibility.

The initial guarded Cargo test-discovery preparation was rejected by automatic
approval review before execution because current-transcript compilation/cache
authority was missing. The owner subsequently explicitly authorized completing all
operations within this Issue while away, including the guarded local gate, shared
cache use, publication and protected merge/cleanup. This resolves that approval
block; it does not authorize unrelated toolchain/toolkit/cache changes. Keep the
existing dependency guard and explain concrete invalidation before any exceptional
rebuild. No rejected action was retried before this owner authorization.

Independent round one of the initial owner-directory stage was clean. Round two
of the broadened subsystem tree accepted the layout/preservation and found one
P2: navigation's own two test files still used parent-directory paths. Move both
beside navigation/mod.rs and retain their logical test modules. This finding is
accepted and fixed; a scan found no other parent-directory test redirections.
Round three must inspect the final snapshot and current local validation evidence.
No findings were rejected.

### Current local gate and exact baseline comparison

Formatting and strict Clippy pass. Clippy initially found five unused inherited
plot glob imports and a line-count increase from a long qualified selection-panel
call; remove the redundant imports and use an explicit scoped selection-panel
import, preserving behavior without suppressions. Full workspace tests pass:
870 passed, zero failed/ignored, with unchanged package counts (App501, CLI50+24,
core36, DSP97, edit12, I/O150). Current executable discovery records those870 names,
ignored sets and binary hashes. Release remains pending.

Existing temporary discovery records were1158/1162-test earlier snapshots rather
than the accepted870-test baseline, so do not compare against those stale records.
For exact evidence, first commit all current work and verify a clean checkout;
temporarily detach the exact accepted base in this same checkout (no new worktree
or cache), run guarded workspace test no-run with unchanged inputs, and capture
actual baseline executable lists. Return to the original branch and verify its
expected committed head before rebuilding/rerunning current tests once. Compare
through the explicit module namespace mapping. Release runs last after the other
checks. Retain raw baseline/current artifact and binary identities externally.

## Post-completion

After current full CI and review pass, complete the protected PR merge/conditional
branch cleanup under owner authorization. Preserve unrelated refs, stashes and
worktrees; no cleanup of earlier accepted branches is part of this Issue.
