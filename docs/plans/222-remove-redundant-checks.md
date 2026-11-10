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
- [ ] CLI/I/O/edit executor applies contract ownership and independent fixtures; records removed/added full names and rationale.
- [ ] App executor retires low-value test layers, preserving event/safety boundaries; records exact manifest.
- [ ] Core/DSP executor retires helper layers and duplicate execution, preserving numerical/resource/cancellation oracles; records exact manifest.
- [ ] Integrate after all executors stop editing; remove unused test-only support and update current UI inventory references. Completed historical plans stay unchanged.
- [ ] Assess production-token equivalence excluding necessary `cfg(test)` wiring; inspect platform-gated source inventory.
- [ ] Compare exact discovery against the preserved 1162-test baseline: every removed/added package/target/name receives a reason; zero ignored. Report source-line and executed-work reduction without unsupported runtime percentages.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` last, after the other checks pass.
- [ ] Record actual evidence, move plan to completed and obtain fresh independent review of the exact revision; resolve findings within three rounds.

Do not build until all edit streams settle. Use sole approved Rust1.98.1, the unchanged locked graph and shared intermediate build-dir/local final outputs. Direct executable selection and existing external compilation guard protect the approved debug/test, release and check/Clippy baseline. Ordinary own-code recompilation is approved; stop before dependencies beyond baseline. No further Clippy diagnosis, benchmark runs or fixture downloads.

Remote metadata update was denied by automatic approval review; do not retry/bypass without resolving authorization. Local work is unaffected. Code publication requires the unchanged full hook and gate, then current-head Linux/Windows/macOS and ci/full. No optional external capture or native UI acceptance is inferred from counts. Preserve existing stashes, unrelated worktrees and the separate Cargo Trash task; retained accepted PR237 refs must not be deleted after the prior automatic-review rejection.
