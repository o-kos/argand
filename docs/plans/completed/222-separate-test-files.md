# Issue #222: Separate workspace tests and remove repeated fixture work

Refs [#222](https://github.com/o-kos/argand/issues/222). This test-only phase is authorized and follows merged #144. It must not close #222: production module organization remains a separate owner decision.

## Roles and risk

- Class A: workspace-wide extraction across more than five code files, including DSP oracles, asynchronous ownership, persistence and data-safety regressions.
- Coordinator: gpt-6-astra medium.
- Executor: gpt-6.1-sol medium.
- Reviewer: gpt-6.1-sol high, independently assigned by the coordinator in a separate context; at most three rounds for this PR.

## Authorized boundary

Move all workspace inline test bodies and test-only fixture/helper implementations into separate source files. Leave production module tree, APIs, ownership, algorithms and behavior unchanged. Preserve all existing correctness test names, platform/feature gates, numerical tolerances, random seeds and case matrices. The owner-approved manual conversion measurement has already moved from an ignored unit test into a separate explicit benchmark; it is the sole intentional discovery/ignored-set change from the historical inventory. Optimize only the four demonstrated redundant-work patterns below, without deleting tests or weakening coverage.

New extracted files use sibling `*_test.rs` names, as requested in #222. Existing separate `*_tests.rs`, nested existing test files, integration targets and existing shared testutil files remain where they are; no cosmetic rename campaign. Module identifiers stay unchanged. Unit tests stay unit tests, with their current private access; this is not conversion into Cargo integration tests.

No new controls or UI behavior, public API, production module reorganization, dependencies, lockfile changes, test ignores or lint suppressions. Keep the single GPUI Kit graph and inherited warnings-deny policy. The broad capability/crate restructuring examples in the upstream coding guide do not authorize implementing #222's production-tree half.

## Current baseline and workflow

- #144 is merged; its final code tree is unchanged by the squash merge.
- Current workspace tests after the benchmark extraction: 1162 passed, 0 failed,
  0 ignored. Historical inventories predate #144 and are source attributes only.
- Retain the existing source/cost audits as inputs; refresh changed source seams
  and executable discovery instead of repeating a costly broad audit.
- One installed Rust/Cargo 1.98.1; direct RUSTC and RUSTDOC executables avoid
  dependency-local rustup pins. Shared intermediate build-dir is configured;
  final artifacts are workspace-local. Guarded no-compile checks confirmed all
  715 release and 709 debug/test artifacts fresh after this output split.
- Existing benchmark extraction preserves the original measurement body and uses
  no new dependencies/features/profiles. Its explicit target has harness=false and
  test=false. Guarded workspace benchmark compilation passed without external dependency compilation; the manual measurement was not run.
- Existing resource-policy edits are a separate documentation commit in this branch.
  No production behavior, UI acceptance, dependencies or toolkit changes follow from it.
- Start from merged main in the normal checkout, with a versioned plan committed
  before bulk extraction. Open a Draft PR using Refs #222. Network trouble must not
  block local authorized work, but code publication still needs the required gate.
- No new worktrees. Preserve the three owner stashes and unrelated work. Do not
  operate on #107 or Wireroom.

## Mechanical extraction: syntax-aware, auditable, reversible

Use source-aware reviewed item boundaries for extraction. If an already available
Rust parser can be used without compiling dependencies, it may produce candidate
spans; otherwise make explicit reviewed per-module edits and preserve original
body slices. Do not compile a new parser helper or introduce a scratch Cargo target.
Do not use regex or brace counting as a Rust parser. Refuse unexpected source hashes,
overlapping spans and destination collisions. Review complex seams explicitly.

For a usual inline block:

```rust
#[cfg(test)]
mod tests { /* original body */ }
```

retain the original attributes, visibility and identifier, changing only wiring:

```rust
#[cfg(test)]
#[path = "owner_test.rs"]
mod tests;
```

The sibling file contains the original body, including its inner attributes/imports/comments. This retains the logical module depth and fully qualified test names. Multiple modules receive separate descriptive files, for example `shell_theme_test.rs`, `shell_analysis_hint_test.rs`, `execution_configuration_test.rs`, while retaining `theme_tests`, `analysis_hint_tests`, `configuration_tests`. Do not collapse modules into a new shared namespace. Existing include-backed modules with no inline implementation can retain their small cfg/include wiring; their already separate files are not renamed.

Audit relative module/include resolution before relocation: nested `mod`, `#[path]`, include/include_str/include_bytes, macro hygiene and source-location macros. Keep fixture bytes, origins and logical module paths equivalent; record any necessary path adjustment separately from body changes. Preserve meaningful doc/module comments.

### Special seams requiring explicit handling

- `dsp/overview.rs` has a standalone cfg(test) root-level test in addition to its inline tests. Keep that test at its original logical scope through a documented cfg-only `include!("overview_root_test.rs")`, containing the original attributes/function, instead of silently changing its fully qualified name by nesting it.
- Test-only associated functions/constants move to cfg-only sibling support files included at their original module scope. This preserves effective visibility and existing method/import paths without adding a production or support module boundary. Include wiring precedes test modules where required by Clippy. Only narrow cfg-only imports, instrumentation slots and forwarding calls remain in production files.
- Examples include analysis delivery/open/request fixtures, OpenFile record_for_test, SaveOf::whole_for_test, Rebinding::kept, BadgeEntry inspectors, Ruler::CLOCK and warned_analysis. Move the implementation and its fixture-specific imports; production retains only cfg module/import/call wiring.
- `stft::BLOCK_SAMPLES`: keep production `1 << 20` unchanged; put the test-only 4096 constant in a separate cfg support file and import it under the original test gate. Do not unify test and production values or alter app dependency versus DSP unit-test feature behavior.
- `sample_statistics::Cache` has a cfg(test) counter field and cfg-only counter increment inside a production function. Rust cannot splice a field from an external file. Retain only the cfg instrumentation slot/type reference and a cfg-only forwarding call at the existing point; move counter type/helper logic into the separate support file. Keep tuple indexing/layout, ordering and visibility unchanged. Treat this as test wiring, not an exemption for inline fixture implementation. If extraction would require changing production state representation or data layout, stop and report instead.
- Existing feature-gated shared testutil modules stay separate with the same `cfg(any(test, feature = "testutil"))`. Do not broaden feature activation or change Cargo manifests.
- Keep test-owned async workers/windows/entities, TempDir/source handles, cancellation guards and destructor lifetimes in the same scope. Moving a support body does not authorize sharing mutable fixtures or shortening teardown.

### Mechanical proof before optimization

Generate a manifest mapping every old owner/module/test/helper to its new file, original attributes/visibility and content-token digest. After extraction, compare normalized token streams for test bodies/attributes (whitespace/comments excluded), with an explicit allowlist only for reviewed wiring/visibility/path adjustments. Compare non-test implementation tokens separately, ignoring only recognized cfg(test) seams: no ordinary production function or type change is accepted. This is a mechanical audit aid, not a substitute for compiler/tests/review.

Run formatting, discovery comparison and focused affected tests before introducing optimization edits. Every discovered name/ignored set must match the refreshed baseline. Any mismatch is a defect to explain/fix, not a permissible expected count reduction.

## Minimal discovery report

Capture `cargo test --locked --no-run --message-format=json` artifacts with the same workspace selection/features as the required gate (avoid package-scoped GPUI feature-unification rebuilds). Record rustc/cargo versions, exact baseline, target triple, enabled features, source inventory and fixture manifest. For each actual libtest executable, capture `--list --format terse` and `--ignored --list --format terse`; normalize into `(package, target, fully-qualified name, kind, ignored)` sorted records. Preserve raw outputs and executable hashes outside the worktree. Track doctests separately. Do not present regex attribute counts as executable discovery.

Repeat after mechanical extraction and after optimization. The report can be small:

| Target | Before names / ignored | After extraction | After optimization | Added / removed / renamed |
| --- | --- | --- | --- | --- |
| application, CLI unit/integration, core, DSP, edit, I/O, doctests | refreshed values | exact-set comparison | exact-set comparison | expected zero |

List cfg-only/platform-specific declarations not discoverable on Linux and compare their source item/attribute inventories. Current Windows/macOS CI validates compilation and execution there; no Linux discovery claim covers those platform-only branches.

## Optimization phase: four bounded changes

1. **Progressive analysis reference reuse.** In `progressive_tests.rs`, compute plain analysis once per (real/IQ domain, Max/Mean/MeanPower reducer), then compare all five batch sizes to that immutable independent result. Plain reference calls go 30 to 6; all 30 progressive runs remain, with all FFT/frame/PSD/waveform/colour assertions, fresh/reset sources and the four-thread pool. Other helper callers retain independently computed references. Do not cache mutable readers or let the progressive path produce its own reference.
2. **Suppression paired-band measurements.** For calls using the same signal/from/to windows, perform one FFT pass and accumulate both power bands independently. Preserve FFT1024, Hann normalization, half-open band membership, per-band count, summation order within each band, stochastic seeds, ranges and thresholds. Keep other windows/times separate. First establish old/new helper results on the same deterministic fixtures; require exact values where operation order permits, otherwise stop and inspect before accepting any tolerance change (none is authorized).
3. **Conversion tone fixture allocations.** Replace per-frame `values[..channels].to_vec()` with fixed-array iteration/truncation or one preallocated output vector, retaining the exact cos/sin expression, cast order and interleaved sample order. Temporary old/new comparison checks every generated f32 bit for real and IQ cases, including existing references/rates. Preserve all domain directions, block sizes 1/17/137/4096, endpoint/tight-band cases and the separately extracted manual cost benchmark.
4. **CLI metadata-only image sizes.** Change explicit individual success cases only after recording that their asserted metadata does not depend on image geometry/defaults. Start with fixed/auto range-mode report cases. Choose a smaller size that satisfies actual layout validation and PNG checks; keep real subprocess launches, success/exit assertions, JSON fields and publication. Never inject a global -i into run/run_json. Retain dedicated no-image-option default rendering coverage, ensure the existing default-options test checks the direct default-dimension contract if it does not already, and retain all long-capture/real fixture cases. Record each changed case and its unaffected invariant.

Apart from the explicitly approved ignored-measurement extraction, no correctness test removals or ignored-state changes are supported by the audit. Keep worker polling bounds, real 400ms hover-grace crossings, sparse cache-budget thresholds, high-overlap FFT262144/hop1 cancellation, read-block carries, source identity, staged publication and persistence/retirement/multiwindow regressions. File size or one slow suite does not prove redundancy.

## Targeted measurement and equivalence evidence

Before editing, retain baseline test executables outside target after the shared workspace build. After optimization retain corresponding new executables. Benchmark filtered exact test names by alternating before/after runs (AB then BA, at least three paired observations per candidate) with identical test-thread setting, Rayon pool, fixture inputs and environment. Warm once; record each wall-time sample and host load/concurrent-work state. Run only after #144 and other builds stop; do not run simultaneous baseline/candidate measurements. If external load is high or results noisy, record that limit instead of reporting an unsupported percentage.

Use the same real aspec binary/path for paired CLI measurements because production code is unchanged; verify its hash/build identity. Record whether optional real-capture cases actually exercised data or skipped their branch, without changing the fixture tree. Keep optional long-file coverage in the full gate. Distinguish compile/setup time from executable runtime. Record removed work counts (30-to-6 plain references, eliminated duplicate FFT passes/per-frame Vec allocations) independently of observed wall-time. No performance claim from whole-suite durations alone.

Temporary comparison helpers/results live outside the final test source unless they cover an independently meaningful lasting invariant; do not add permanent tests that merely mirror the rewritten helper. Numerical review examines preserved expression/order/tails and the surviving independent oracles.

## Implementation steps

- [x] Confirm merged #144, restore local changes exactly and create the ordinary #222 branch.
- [x] Commit this phase plan and open the early Draft (Refs #222): PR #237.
- [x] Record exact per-target discovery/ignored/cfg/fixture baseline and targeted measurements.
- [x] Extract 49 inline test modules across all crates without changing module/test names; preserve existing separate files.
- [x] Extract free/associated helper bodies/constants and resolve cfg-only seams with effective visibility preserved; retain only instrumentation slots and forwarding calls.
- [x] Verify full extraction manifest, token/wiring diff and discovery equality; run focused regressions.
- [x] Apply the four bounded fixture/setup optimizations with equivalence and retained-coverage evidence.
- [x] Record paired measurement samples, exercised optional fixtures and limitations; retain all original tests.
- [x] Update only current test organization/documentation and this phase plan; explicitly leave production module organization in #222 open for owner agreement.
- [x] Complete fmt, strict Clippy, full workspace tests and release after the gate.
- [x] Move this completed phase plan before final independent review; provide neutral exact snapshot/diff/manifest, raw validation and honest pending limits.
- [x] Address findings within the three-round policy; independent round two clean and coordinator assessment accepted.
- [ ] Publish with hook intact, Ready/current Linux+Windows+macOS ci/full; merge only under existing owner authorization and fresh exact-head/protection/thread guards.

## Acceptance and evidence matrix

| Criterion | Evidence | Current status / limits |
| --- | --- | --- |
| No inline test/helper implementation in production files | Syntax inventory + reviewed wiring/field-slot exceptions + diff | Extracted; remaining cfg-only include/slot/forwarding exceptions documented |
| Every previous test name/ignore/gate preserved | Per-executable before/after exact discovery plus platform source inventory | Exact executable-name sets match; final workspace tests 1162/0/0; platform gates assessed from source |
| Production behavior/API/tree unchanged | Non-test token/diff assessment, unchanged manifests/lock, full gate/release and reviewer | All 50 production-file non-test token streams match; cfg wiring/support only; independent round two clean; coordinator accepted |
| Four optimizations preserve all case axes/oracles | Helper equivalence evidence, critical numerical review and focused actual tests | Temporary fixture comparisons matched bitwise; case axes, names and tolerances retained |
| Cost statements supported | Paired raw measurements + work counts + load/fixture record | Three paired AB/BA/AB samples per case; limited local timings below |
| UI semantics unchanged | Existing production-view dispatch/focus/disabled/overlap/retirement suites retained | No new UI behavior or native acceptance claimed |
| #222 production tree remains owner-gated | Partial-phase plan and PR use Refs, issue stays open, explicit deferred production scope | Required boundary, not an unfinished implementation checkbox |

## Compilation guard and validation authority

Use workspace-selected commands to preserve dependency feature unification, including
focused filters. A temporary external compiler guard must allow actual compilation
only for this repository's own crates and block all registry/dependency compilation
before executing rustc. Compiler metadata probes may forward to the direct approved
compiler. An unexpected blocked dependency rebuild is a stop-and-explain condition;
validation does not waive owner approval. Do not change compiler/profile/features,
RUSTFLAGS, toolkit graph or dependency versions to make the guard pass.

Ordinary own-project recompilation for test changes is authorized within the agreed
baseline. The owner additionally approved the check/Clippy baseline in this same
compiler, cache and locked graph; the guard permits those locked registry inputs
only during that approved baseline. Other external rebuild requests still require
stop-and-explain approval. Guarded `cargo bench --workspace --bench conversion_cost
--no-run` passed with only own-crate compilation; the explicit measurement command
is `cargo bench --workspace --bench conversion_cost` and was not run. Package-only
selection can change feature unification and is excluded.

## Required checks and review

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (inherited strict warnings)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after the checks pass
- [x] Exact current-revision independent review and coordinator assessment
- [ ] Current Ready PR Linux, Windows, macOS and ci/full all explicitly success

No native UI acceptance is fabricated for a test-only change. If a relocation or optimization uncovers a production regression, apply the active-task routing rules and report the scope rather than silently redesigning production.

## Rejected approaches

Regex/brace counting; merging test modules or renaming tests; broad *_tests.rs renames; moving unit tests into integration targets; reducing DSP domains/FFT sizes/capture durations/tolerances; replacing real UI/subprocess dispatch with direct state calls; loosening waits or ignores to obtain green; production clock seams/framework refactoring; broad suppression or dependency changes; claiming success from test counts or suite-time deltas alone.

## Post-completion

Safe conditional cleanup of this accepted PR branch after protected squash merge; preserve owner stashes/backups/processes. #222 remains open for a separately discussed production-module proposal and any remaining owner-approved scope. No production module moves are authorized by completion of this test phase.

## Current implementation evidence

- Mechanical extraction compiled on the existing workspace debug/test graph with an
  external guard allowing only repository crate compilation; zero dependency builds
  were requested. All 1162 discovered test names and zero ignored cases matched.
- 48 ordinary module bodies matched their original slices after parsing/canonical
  formatting with rustfmt. The additional restricted-visibility shared harness and
  root test retain their original logical scopes. A token-tree audit confirms
  non-test token equality across all 50 changed production source files after
  removing positive cfg(test) items; this is an audit aid, not a Rust parser or
  replacement for compilation and independent review.
- Test-only helpers use cfg-only include files at the original module scope instead
  of new child modules. This preserves effective visibility, method paths and root
  test names without reexports or production module changes. Existing include-only
  wiring stays unchanged. The counter slot remains the same AtomicU64 via a type
  alias; its increment and the conversion reader-failure effect move behind cfg-only
  forwarding calls at the original sequence points.
- Temporary old/new comparisons passed on the existing conversion and suppression
  fixtures: every generated tone f32 and each band-power f64 matched bitwise. The
  existing scheduling matrix also passed. Temporary comparisons and old helper
  copies were removed from final source; numerical tolerances were not changed.
- Scheduling references drop from 30 to 6 while all 30 progressive cases remain.
  Paired suppression bands share FFTs only for identical sample intervals in three
  tests (four intervals). Conversion synthesis no longer allocates a Vec per frame.
- Only fixed/auto range-mode metadata cases use explicit 500x300 images. Their PNG
  publication checks remain explicit. The dedicated default-render test now omits
  the image option and asserts the documented 2048x512 PNG dimensions, retaining
  actual default-image coverage separately from the report-only cases.
- Final workspace tests pass: 1162 passed, zero failed/ignored; exact discovery is
  unchanged. Formatting and strict all-target Clippy pass. Clippy required a
  test-helper iterator filter and ordering three support includes before test
  modules; no production logic or lint suppression changed. Final release passed after all other checks (2m46s, own argand recompilation
  only); independent round two is clean and the coordinator accepted the critical invariants.
- Three paired local samples per case (AB/BA/AB) gave before/after medians:
  scheduling 7.6668/4.5979s, paired suppression 0.1385/0.1305s, conversion
  0.5533/0.5426s, and CLI range reports 0.7719/0.1671s. These are filtered
  debug-test runs, not release/application benchmarks. Conversion ranges overlap
  (0.5480..0.5595s vs 0.5329..0.5797s), so no reliable conversion runtime speedup
  is claimed; its removed per-frame allocations are directly visible in source.
- Guarded standalone workspace benchmark compilation passed with own-crate
  compilation only. Its three manual scenarios/output are unchanged; no measurement
  execution or performance threshold is introduced.
- Independent round one reported no substantive source findings. Round two found no substantive issues in
  the final source and Clippy delta; the coordinator accepted the assessment. Local evidence remains distinct from pending
  current-head Windows/macOS/Linux full CI.

## Final local evidence locations

The handoff retains raw fmt, Clippy, workspace-test and release logs, exact final
discovery, fixture-equivalence results and paired samples outside the worktree.
Final discovery matches all 1162 names with zero ignored; test totals are 1162/0/0.
The external guard blocked no dependency compilation during resumed tests/release.
The check/Clippy baseline was explicitly owner-approved; no compiler, feature,
profile, toolkit graph or lockfile changes were made. Production-token equality
was refreshed after the final cfg-only include ordering. Linux tests do not prove
platform-only execution or that optional external capture fixtures were present;
retain those branches and distinguish pending platform CI from local evidence.

Independent review used the approved gpt-6.1-sol high reviewer in both rounds.
No substantive findings were declined or left unresolved. The coordinator examined
the independent scheduling reference/fresh-source axes, conversion expression and
bit order, per-band FFT accumulation order and CLI default-image assertions.
Local implementation and review are complete; publication/current-head full CI
and protected merge remain post-implementation steps, not claimed as passed here.
