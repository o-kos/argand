# Issue #222: Make tests follow product contracts

Refs [#222](https://github.com/o-kos/argand/issues/222), Draft PR #238. This test-only phase deliberately retires low-value automated guarantees and repeated layers. Product behavior, acceptance criteria and production module organization do not change. The broader Issue remains open.

## Roles and authority

Class A: workspace-wide coverage changes include numerical correctness, concurrency and file safety. Coordinator: `gpt-6-astra` medium; executors: `gpt-6.1-sol` medium; fresh independent reviewer: `gpt-6.1-sol` high, separate context, at most three rounds. The owner accepted the contract-first proposal after rejecting a narrow deletion list. Counts are observations, never quotas.

Preserve existing dirty work. Change test files and necessary `cfg(test)` wiring/helpers only, plus current documentation references. No production module/API/behavior changes, dependencies, toolkit, features, profiles, compiler flags or cache changes. Retiring an implementation assertion does not require replacing that assertion at another layer.

## Contract ownership and intentional retirement

| Area | Authoritative protection | Retired guarantees / duplicated execution |
| --- | --- | --- |
| CLI command | Actual command scenarios cover accepted inputs, physical units, output JSON/PNG, status and batch orchestration | Direct argument-field/accessor checks, copied help/error wording, dependency font behavior, private layout/formula snapshots and repeated parser rejection layers |
| CLI rendering | Small independent synthetic analysis fixtures cover orientation/composition and meaningful clipping/alignment | File/FFT setup where only layout or channel conversion is under test; decorative literal geometry/color pins |
| I/O reading | Independent known bytes cover native WAVE/raw, multirun, decoded WAVE and FLAC paths | Repeating scalar conversion, pair counting and metadata accessors through every wrapper and CLI layer |
| I/O publication | Preserve independent copy/encode/adjusted/computed/decoded paths and staged-publication failures | Trivial helper/constructor/default checks and redundant happy paths; do not remove known identity, cancellation, collision, corrupt-FLAC, PCM24/RF64 protections |
| Editing | Immutable source/IQ boundaries, independent sequence results and history state transitions | Private piece/index/accessor representation pins and repeated constructor/redo checks |
| Core/DSP | Numerical/alias/source parity, resource/cancellation and independent progressive scheduling oracles | Trivial constructors/getters/defaults/toggles, duplicate formatters, malformed private frame/shader inputs, older overlapping progressive-equivalence suite; reduce redundant batch/overview matrices to orthogonal boundary cases |
| App | Actual event workflows, navigation/physical boundaries, generations/lifetimes and data safety | Toolkit-only Form smoke, asset enumeration, decorative/private UI snapshots and ordinary model-wiring checks with a stronger consumer scenario |

Intentional losses include exact private synthetic colors/geometry, helper wording and representation details. Stable behavior alone is not obsolete; known regression and independently implemented paths remain meaningful. Unique assertions are not automatically product contracts. Preserve numerical thresholds rather than weakening them to meet a count. Remove only unused test instrumentation, never production instrumentation required by real behavior.

The earlier four consolidations and 28-removal proposal are historical inputs, not the controlling allowlist. Their prior 1158-test local gate does not validate this revision. Prior expected1130 is withdrawn. The audit deeply read CLI/edit and smaller I/O groups; large writer-tail/FloatWave assessment was incomplete, so unverified safety tests are retained. The provisional CLI25–40 / IO55–85 / EDIT6–9 scenario estimates were design estimates, not promised counts or permission to pack old assertions into fewer test functions.

## Implementation and evidence

- [x] Commit this replacement scope before further edits; keep PR Draft.
- [x] CLI/I/O/edit executor applies contract ownership and independent fixtures; records removed/added full names and rationale.
- [x] App executor retires low-value test layers, preserving event/safety boundaries; records exact manifest.
- [x] Core/DSP executor retires helper layers and duplicate execution, preserving numerical/resource/cancellation oracles; records exact manifest.
- [x] Integrate after all executors stop editing; remove unused test-only support and update current UI inventory references. Completed historical plans stay unchanged.
- [x] Assess production-token equivalence excluding necessary `cfg(test)` wiring; inspect platform-gated source inventory.
- [x] Compare exact discovery against the preserved 1162-test baseline: every removed/added package/target/name receives a reason; zero ignored. Report source-line and executed-work reduction without unsupported runtime percentages.
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` last, after the other checks pass.
- [x] Record actual local evidence and move plan to completed before fresh independent review.
- [x] Obtain fresh independent review of the exact revision; resolve findings within three rounds.

Do not build until all edit streams settle. Use sole approved Rust1.98.1, the unchanged locked graph and shared intermediate build-dir/local final outputs. Direct executable selection and existing external compilation guard protect the approved debug/test, release and check/Clippy baseline. Ordinary own-code recompilation is approved; stop before dependencies beyond baseline. No further Clippy diagnosis, benchmark runs or fixture downloads.

Remote metadata update was denied by automatic approval review; do not retry/bypass without resolving authorization. Local work is unaffected. Code publication requires the unchanged full hook and gate, then current-head Linux/Windows/macOS and ci/full. No optional external capture or native UI acceptance is inferred from counts. Preserve existing stashes, unrelated worktrees and the separate Cargo Trash task; retained accepted PR237 refs must not be deleted after the prior automatic-review rejection.

## Current source evidence (before the shared gate)

CLI/I/O/edit source inventory is331→236 tests: CLI131→74, I/O182→150,
editing18→12. This is101 removed names and6 additions/renames, net95 fewer;
compiled discovery remains pending. CLI primitive argument/prose/private-layout
checks were retired; custom grammar, physical report/schema, rendered alignment
and binary orchestration remain. I/O keeps distinct native/decoder/FLAC readers,
copy/adjusted/computed encoders and source/publication failure boundaries.

Selected execution pipelines shrink independently of test-function count:
CLI scalar-format rendering12→2 representative domains, panel rendering8→3
orthogonal compositions, writer copy/raw/adjustment families36→6. Ordinary
renderer fixtures supply independent images/envelopes instead of files and FFTs;
signal-dependent waveform/shared-grid scenarios retain real analysis. The editing
vector model now constructs inserted source coordinates independently of Capture.
The former adjusted-format oracle called production Linear decode/push; replacement
float and unsigned midpoint/conjugation checks assert independent literal bytes.
I32/computed, packed24, signed parity/clamping and FLAC paths remain separate.

The earlier CLI25–40/I/O55–85 estimates are retracted: they underestimated
independent renderer/batch contracts and the writer's distinct numerical and
identity/cancellation/publication paths. This phase does not establish a multiplefold
whole-suite count or runtime improvement. Observed source/selected-case reduction
must be reported separately from actual gate runtime and optional external fixtures.

## Round-1 local results (historical)

Compiled discovery and workspace tests agree on869 passed, zero failed/ignored:
App501, CLI unit50 plus binary24, core36, DSP96, editing12, I/O150.
The exact baseline delta is303 removed names and10 additions/renames, net293.
This is a count reduction of about25%, not a multiplefold whole-suite reduction.
The fresh baseline source inventory includes all109 test-bearing files and1162
attributes, including GPUI Kit tests and cfg declarations. Changed production
files differ only in six removed cfg(test) module declarations; no product code
or module organization changes. Current UI inventory references were reconciled;
completed plans and compatibility evidence remain historical.

Formatting and Clippy pass; Clippy took5.73s after removing two unused test imports
and adjusting the new fixture's fixed-size chunk iterator. Test compilation took
31.74s and compiled only seven own targets;702 other artifact records were fresh.
Raw test-family times: App46.82s, CLI unit1.16s, binary22.60s, core0.00s,
DSP19.44s, editing0.03s, I/O0.78s. These are current measurements, not a controlled
before/after benchmark. No optional external-fixture presence or native acceptance
is established. Release passed last in2m37s, recompiling only the app. Fresh independent review
and current-head remote CI/publication remain pending.

Further selected DSP work reduction: overlapping progressive suites84→18 analysis
calls; overview comparisons20→12; final-only16comparisoncases/32analyses→8cases/16analyses;
suppression removes six tone transforms and two repeated deterministic passes.
Independent numerical/source/resource/cancellation checks remain for review.

## Independent review, round 1

One P2 finding accepted: deleting the fixed-scale cache pixel regression left only
an oracle that changed shading style before each changed grid. Same-style calls
used unchanged data, so an erroneous early return for an unchanged Shading could
escape. Restore `fixed_scale_refreshes_changed_columns_and_gaps_without_touching_other_columns`
unchanged: data and propagated gap columns change under one Shading, the image is
compared with full shading, and the unchanged far column stays unchanged. This is
a pixel-cache invalidation regression, not evidence of a numerical gap-fill defect.
The finding does not authorize product changes. Round-1 gate/count evidence above
is historical for the earlier snapshot; the restored-test revision needs refreshed
formatting, Clippy, workspace tests/discovery and release last, then round 2 with
the same reviewer. All other reviewed categories had no substantive findings.

## Round-2 current local evidence

The restored regression passes. Current workspace/discovery:870 passed, zero
failed/ignored;302 removed baseline names and10 additions/renames, net292 fewer.
DSP now has97 tests; other package counts are unchanged. Formatting and Clippy pass
(Clippy2.10s). Only the DSP test target recompiled in3.42s;708 other test artifact
records were fresh. Raw current family times: App51.49s, CLI unit1.26s,
binary23.71s, core0.00s, DSP18.51s, editing0.03s, I/O0.71s. Release ran last and
passed in0.32s with all715 artifacts fresh. The external compiler guard stayed
enabled; no dependencies recompiled. These timings are not a controlled speedup
comparison. The second independent review is clean; current-head remote CI remains pending publication.

## Final review disposition

Round 2: clean, same independent reviewer (`gpt-6.1-sol` high). The accepted P2 is
resolved by restoring the original fixed-scale cache pixel regression exactly;
no other findings remain. The coordinator accepted the implementation and review.
Only this review-status prose changed after the reviewed snapshot; source and
validated compiler inputs are unchanged. This completes the local test-only phase,
not the broader Issue. Publication and current-head full CI remain pending; no
remote mutation or merge is claimed. The observed whole-suite reduction is about
25%, and no multiplefold whole-suite count/runtime improvement is claimed.
