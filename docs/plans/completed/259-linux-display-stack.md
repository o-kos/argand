# Issue #259: Linux display stack and bounded export

Resolves #259 (https://github.com/o-kos/argand/issues/259), together with
[the tagged-release plan](242-exact-commit-release.md) in PR #267.

## Accepted scope — 2026-11-23

The owner accepted PR #267 and the reported validation limits, authorizing Ready,
full CI and squash merge through the protected Pull Request. This supersedes the
historical transfer stops and earlier publication restrictions retained below.
Class A; Coordinator/Executor remain `gpt-6.1-sol` medium, independent Reviewer
`gpt-6.1-sol` high, following the current [role policy](../../../AGENTS.md).

- [x] Ship the Linux executable directly, without bundled system shared libraries
  or a launcher library-path override. Keep archive names, notices and Windows
  packaging, with compatible runtime libraries supplied by the host distribution.
- [x] Complete finite two-phase and bounded direct/partitioned export for whole,
  partial I/Q and real captures. Preserve physical coordinates, FIR quality,
  endpoint behavior, source identity, cancellation and publication protection.
- [x] Correct genuine test-context shutdown and retained reader/worker ownership.
  Remove only the approved unnecessary test setup, preserving numerical oracles,
  real workflow coverage and explicitly documented constructor coverage limits.
- [x] Complete fresh independent source/evidence reviews and the local gate.
  The final ordinary suite passed all 942 tests in 58.93 s under 1 GiB/no swap,
  256 tasks and default native parallelism. The release build ran last and passed
  in 97.91 s. Its binary identity is preserved in the external evidence manifest.
- [x] Commit and push the reviewed source with ordinary hooks. The pre-push hook
  passed formatting, all-target Clippy and all 942 tests again.
- [x] Record owner acceptance and move both accepted plans to completed. The
  protected merge still requires successful current `ci/full` on all platforms.

## Accepted evidence limits

The owner accepted the implementation as reported. Native Wayland, baseline-host,
portable-archive GUI startup and interactive GUI acceptance were not newly tested.
Earlier isolated Xvfb evidence applies to an older binary. No claim of current
GUI acceptance follows from the code gate or owner acceptance. Mixed isolated
setup timings and variable full-suite peaks establish no general speed/RSS gain.
The exact 1 MHz to 1000.1 Hz conversion point now checks geometry only; its former
full constructor coverage was withdrawn and this limit remains explicit.

Exact source, review, runtime and release identities are retained outside the
worktree under `/tmp/argand-259-242-evidence/`, including the final setup and
publication handoffs. No release was published by this task.

## Post-completion

Broader distribution formats remain in #86. Full current cross-platform CI and
resolved conversations are mandatory merge conditions. Production release and
any additional host/runtime validation remain separate authorized work. Branch
and worktree cleanup follows the accepted-head checks in CONTRIBUTING.

## Historical stages

The following is an unchanged chronological record apart from relative-link
relocation. Earlier unchecked boxes, pending statements and withdrawn scopes
record their stage at that time; the accepted scope and limits above govern the
completed plan. Unperformed historical runtime checks are not marked passed.

<details>
<summary>Historical implementation, diagnostics and transfer records</summary>

Tracks #259 (https://github.com/o-kos/argand/issues/259).

## Resumed work — 2026-11-23

The owner resumed the saved branch. Class A; Coordinator and Executor remain
`gpt-6.1-sol` medium, independent Reviewer `gpt-6.1-sol` high. The transfer stop
below is historical; focused validation and one bounded default full test are
now authorized. The owner subsequently authorized committing and pushing the
reviewed changes on this existing task branch, using the ordinary hooks.
Ready transition, merge and release remain unauthorized. Current remote checks
and final owner acceptance remain pending.

The installed compiler, repository pin and selected immutable SDK compiler/target
match. Artifact locations, dependency graph, profiles and compiler flags remain
unchanged. Other workspace processes must not be disturbed.

- [x] Set `Config.analysis.workers = 2` only in the shared replacement window,
  conversion property-panel window and editing test window before `Shell::new`.
  Preserve real parallel DSP, I/O, source identity, conversion/adoption, cancellation
  and every assertion. Product automatic workers and analysis budget tests stay
  unchanged, as do 1x workflow fixtures and dedicated 2x HiDPI tests.
- [x] Compile ordinary workspace test binaries separately under the existing
  compiler-appropriate 3 GiB bound, then run the existing seven disk-change cases
  under 512 MiB/no swap with ordinary harness parallelism.
- [x] Run one ordinary `cargo test --locked` under the unchanged 1 GiB/no-swap,
  256-task/180-second bound without CPU quota, cpuset or test-thread overrides.
  Capture exact source/binary identities and own-child process memory/thread data.
  On failure, diagnose before any retry; never raise limits automatically.
- [x] Obtain clean exact-snapshot review and complete the local gate, release
  build last. Current remote CI and final owner acceptance remain pending.

The first resumed default suite passed all 941 tests, including all 564 application
tests, in 84.54 seconds. The cgroup charged peak was 916.7 MiB with every memory
event counter zero and no swap; sampled application maximum was 71 threads.
Sampled application Anonymous reached 839852 KiB and PSS 938271 KiB. These are
different accounting views, not additive totals. The unchanged default harness
parallelism and production worker-budget tests were retained. Exact-snapshot
independent review of the three fixture changes was clean. The coordinator
accepted this as a passing test gate, not complete allocation attribution:
remaining live pixel buffers versus allocator-retained free chunks are unresolved.
A threshold smaps summary cannot distinguish those owners. The final ordinary
SDK release build passed in 1 minute 46 seconds under the separate compiler-only
3 GiB bound. The new production executable opened a disposable 48 kHz I/Q WAV
and rendered its 6 kHz tone on an isolated Xvfb display with isolated XDG state,
without LD_LIBRARY_PATH or LD_PRELOAD. The application reported analysis completion
and first picture painted; the captured frame shows the expected signal.

This establishes local SDK production startup with system display libraries. It
does not establish portable source-toolkit archive startup, a baseline Linux host,
native Wayland, GPU hardware acceleration or interactive UI acceptance. Xvfb
reported missing DRI3, but the application rendered successfully. No input was
injected. Only task-owned application/display processes were stopped; the owner's
existing application and display were preserved. Current remote CI, PR refresh,
commit/push and final owner acceptance are pending.

### Follow-up: two UI-only information-panel fixtures

Class A and the existing role assignments remain unchanged. Replace incidental
native analysis setup only in `information_buttons_share_toggle_internal_click_and_all_switch_routes`
and `all_information_panels_suppress_external_tooltips_and_restore_on_fresh_hover`
with the existing `describe_panel_document` helper. Keep genuine source metadata,
statistics, prepared-reader mailbox ownership, rendering, actions, guards and every
assertion. Real save/suppression/lifecycle integration cases remain unchanged.

- [x] Apply the two fixture substitutions, format and compile.
- [x] Run exactly these two existing cases once under 512 MiB/no swap with ordinary
  harness parallelism: both passed in 3.09 seconds, cgroup peak 28.4 MiB, zero
  memory events. Independent exact-snapshot review was clean.
- [x] Obtain clean independent review of the two-case scope before the final gate.

### Follow-up: scoped synchronous reference analysis

The two information-panel fixture substitutions passed independent review.
The existing aggregation replacement test computes a synchronous numerical
reference outside the analyst's local pool. Give only that reference computation
an existing scoped `Settings` pool with two workers. Preserve the real analyst,
stale/latest generation checks, numerical comparison and production defaults.
This removes an incidental global-pool initialization path; it does not attribute
the remaining full-suite anonymous memory to that global pool.

- [x] Apply the scoped reference pool, format and check Clippy.
- [x] Run exactly the existing aggregation replacement test once: passed in
  0.01 seconds under 512 MiB/no swap, with zero memory events. Narrow independent
  review was clean before the final full gate and release build.

### Final resumed gate

After both narrow follow-ups received clean independent review, the final ordinary
workspace suite passed all 941 tests, including all 564 application tests. The
application tests took 31.42 seconds and the whole workspace 57.40 seconds.
Default harness parallelism/native CPU availability remained unchanged. Under the
same 1 GiB/no-swap/256-task/180-second bound, cgroup peak was 883.8 MiB and every
memory event counter remained zero. Sampled application maxima were 981012 KiB
RSS, 906519 KiB PSS, 808564 KiB Anonymous and 85 threads; these are distinct
accounting views and sampling observations, not additive quantities. Timing and
thread-count variation do not establish a universal speed or thread reduction.

The remaining anonymous memory is not fully attributed to live buffers versus
allocator-retained free pages. The last application sample still held 807356 KiB
Anonymous with one analyst and twelve `processing::con`-named threads. Truncated
thread names do not prove complete ownership; scoping the reference computation
does not imply that no other caller initializes process-global Rayon. No further
optimization or instrumentation was added.

Formatting and all-target Clippy passed, then the final full test ran once.
Release build ran last and passed in 61.48 seconds under the separate 3 GiB
compiler bound. Its executable is byte-identical to the production executable
already verified in isolated Xvfb, so the startup scenario was not repeated.
The earlier runtime evidence and its SDK/X11/native-Wayland/portable-archive
limitations still apply. Code remains frozen after clean review; subsequent plan
changes only record evidence. Current remote CI, PR refresh, commit/push and final
owner acceptance are pending. No Ready, merge or release action was performed.

### Residual allocator attribution diagnostic

Class A and existing role assignments remain unchanged. The owner requested
attribution of the remaining anonymous memory. One approved app-only diagnostic
runs the existing 564 cases with default parallelism and unchanged 1 GiB/no-swap,
256-task/180-second limits. Temporary test-only instrumentation records glibc
`malloc_info` and synchronized process counters at first cleanup and process exit,
so accumulated free arena retention can be separated from approximate occupied
arena/mmap storage. The initial boundary is not a zero-allocation baseline;
parallel tests may already be running. No scheduling/assertion or allocator tuning
changes are allowed. This is a diagnostic, not a new gate or new test.

- [x] Preserve source bytes/identities, compile the temporary diagnostic separately
  under the compiler-only 3 GiB limit, then perform exactly one app-only run.
- [x] Restore the instrumented source byte-for-byte and compile ordinary workspace
  test binaries without running them. Preserve the accepted release executable.
- [x] Report arena system/free/mmap separately; approximate occupied includes
  metadata/tcache and does not identify non-glibc allocation owners.

The single app-only diagnostic passed all 564 existing tests. At process exit,
glibc reported 785.199 MiB current arena storage, 784.614 MiB already free,
0.585 MiB approximate occupied arena storage and no malloc mmap allocations.
Anonymous was 777.414 MiB, with thirteen threads and no native analyst/FFT names.
Thus the accumulated suite retains a large amount of already-freed glibc arena
storage; it is not a comparably sized live glibc pixel/context heap. Exact resident
free pages cannot be derived by subtracting XML free storage from Anonymous:
nonresident arena pages, stacks, metadata/tcache and non-glibc mappings differ.
This diagnostic reproduces the scale and establishes the retention mechanism,
not exact ownership of every page in the earlier run. No allocator tuning or
production fix was made. Temporary source was restored byte-for-byte and the
ordinary no-run compile restored the exact accepted app test binary; release
binary remained unchanged. This adds attribution evidence, not another gate.

### Test usefulness and computational-cost audit

The owner accepts the observed glibc retention as not presently a defect. Do not
add allocator tuning, trimming or arena limits. A read-only audit prioritized six
DSP/app test families, distinguishing real export/conversion boundaries from
expensive fixtures and incidental constructor work. It found a metadata-only
fractional conversion case constructing a 501795-tap filter with 2M/4M FFT plans,
64 repeated fractional-edge stream constructors, and 100000-sample chunk tests
that may retain their contract with a smaller derived fixture. Current export
convolution uses at most 65536-point partition FFTs; giant legacy conversion
constructors must not be conflated with spectrum visualization. Native endpoint,
80 dB image, near-guard and real source-safety contracts remain meaningful.

Priorities with exact source locations:

1. [Narrow fractional constructor and long-phase test](../../../crates/dsp/src/convert/tests.rs:266):
   separate the actual 1000:1 constructor acceptance boundary from phase checks.
   The former constructs 501795 taps, a 2097152-point forward and 4194304-point
   inverse plan without feeding samples; the fixed complex vectors alone are
   80 MiB. Do not delete this boundary as if it were only a phase assertion.
2. [Fractional native-edge matrix](../../../crates/dsp/src/convert/tests.rs:164):
   twelve outer cases expand to 64 real stream constructors. Keep all cheap
   metadata checks; discuss representative actual constructors for distinct
   boundary/rounding shapes, rather than synthesizing every Cartesian duplicate.
3. [Explicit carrier phase](../../../crates/dsp/src/convert/tests.rs:293): four stream
   constructions accompany metadata and one-sample long-index phase checks.
   Preserve the real translate result and extreme references; filter work is
   incidental, but do not replace it with duplicated oscillator logic.
4. [Whole RF guard](../../../crates/dsp/src/extract_tests.rs:220) and
   [inside-guard image](../../../crates/dsp/src/extract_tests.rs:272): each uses four
   million complex inputs and eight million outputs, not a four-million-point
   FFT. Current export uses 65536-point partitions and the production FIR.
   Any reduction must preserve both FIR margins, transients, multiple partitions
   and enough integration time to resolve the 20 Hz near-edge image relationship.
5. [Native/chunk exactness](../../../crates/dsp/src/extract_tests.rs:197) and
   [mixing order](../../../crates/dsp/src/extract_tests.rs:298): representative 5140-tap
   filters process 100000 samples repeatedly. Derive a smaller length from margins
   plus multiple 8192-sample partitions, keeping all chunk and analytic assertions.
6. [Narrow-band noise measurement](../../../crates/dsp/src/suppress_tests.rs:435):
   the measurement-only property constructs full suppression/crossfade filters
   (including a 262144-point setup FFT). Genuine noise-floor/tone rejection matters;
   avoiding unused filter synthesis needs an explicit boundary/API decision, not
   a fixture that fakes a measured result.

Exceptions: max FIR wanted/image response and real DC/Nyquist/export-tail tests
protect meaningful numerical boundaries. Overview compression must retain more
than 4096 frames; the large FFT values in pure scheduling-budget tests perform no
FFT. The [high-overlap cancellation case](../../../crates/dsp/src/progressive_tests.rs:199)
uses FFT262144/hop1, valid at the DSP boundary but unreachable through application
settings capped at 95% overlap. Consider separating adversarial budget arithmetic
from smaller real cancellation work without silently removing public API coverage.
No individual runtime ranking was inferred from overlapping completion timestamps.

The proposed first change is a derived smaller representative chunking fixture,
not deletion of numerical boundaries. Any reduction of Cartesian constructor
coverage needs an explicit mapping from retained cases to distinct rounding/domain
properties. The narrow 1000:1 constructor itself checks real acceptance and must
not be removed as if it only tested oscillator phase. No tests, benchmarks,
builds, code edits or new issues were performed for the audit. Implementation
requires discussion of these concrete findings; the accepted gate source stays
unchanged.

### Approved test-fixture optimization

Class A and existing role assignments remain unchanged. The owner approved four
narrow test families: retain every fractional metadata/validation assertion but
select representative real constructors; separate long-phase intent from the
exact narrow 1000:1 parameter boundary; reduce representative chunk fixtures;
and reduce whole-RF steady-tone fixtures using explicit support/coverage guards.
No production DSP/API, tolerance, source safety, allocator or dependency changes.

The fractional matrix retains all 64 cheap plan checks and twelve outside-band
rejections. Actual stream construction covers all three native rates and four
domain directions at the larger reference, with full and smallest candidate
output rates (24 constructors). The larger ULP is representative, not proof that
it dominates every rounding sign at other reference origins. After the owner questioned the scenario, the test now checks parameter geometry
without constructing filters. That exact full-constructor coverage is withdrawn.
Its exact parameters remain covered by cheap physical-band/width/sample-count
validation. Actual narrow filter construction is no longer covered by this case;
existing numerical conversion and integration cases remain unchanged.

Fixture lengths derive from four FIR margins plus multiple partition lengths and
a partial tail; the checked middle half stays strictly clear of zero-padded edges.
The inside-guard test also retains conservative coverage of 20 Hz near-edge
modulation. This is not a claimed wanted/image beat period: those components
are separated by Fs and alternate by parity at the 2Fs output rate.

- [x] Change only the selected tests, preserving numerical tolerances, analytic
  references, native-sample bit exactness and all chunk variants.
- [x] Format before one separate DSP no-run compile under 3 GiB. Run the changed
  cheap/constructor cases and two RF cases once in separate 512 MiB/no-swap,
  256-task bounded processes with default harness concurrency and no thread flags.
- [x] Run all-target Clippy and prepare the exact narrow delta for independent
  review. Do not repeat full tests or release build before evidence assessment.

Focused results: the initial five-case group passed in 0.98 seconds; the two RF
cases passed in 3.69 seconds. After the owner questioned the scenario, the narrow case was changed to check
parameter geometry without constructing filters; that changed
case passed once on final source, and final all-target Clippy passed. The other
four initial cases and both RF tests were not repeated because their code did not
change. All three runtime groups had zero memory events under 512 MiB/no swap.
Chunk fixtures now use 34873 samples; whole RF fixtures use 1381105. All existing
analytic tolerances and chunk variants remain. DSP test count changes from 105
to 106 by splitting two existing properties, not by adding replacement coverage
for the withdrawn full narrow constructor. Independent review was clean; the final gate results below supersede this
focused-only stage. No push was performed.

### Final gate after test-fixture optimization

The exact test-only delta passed independent review. Formatting and all-target
Clippy passed. After a separate ordinary workspace no-run compile under the
3 GiB compiler bound, one ordinary full test run passed all 942 tests: application
564 tests in 29.98 seconds, DSP 106 tests in 12.54 seconds and the whole workspace
in 46.77 seconds. The one additional DSP case separates two existing properties;
it does not replace coverage of the intentionally withdrawn full narrow constructor.
Default parallelism/native CPU availability remained unchanged. Runtime limits
were 1 GiB/no swap, 256 tasks and 180 seconds, with no quota/cpuset/thread overrides.
Cgroup peak was 906.5 MiB, every memory event counter was zero. Sampled app maxima
were 1001368 KiB RSS, 926884 KiB PSS, 828984 KiB Anonymous and 73 threads. These
accounting views and samples are not additive or universal performance claims.
Earlier timings are separate revisions, not a paired baseline for this change.

Release build ran last and passed in 60.21 seconds under the existing 3 GiB
compiler bound. Its executable is byte-identical to the already verified Xvfb
production executable; no duplicate runtime launch was performed. Existing
SDK/X11/native-Wayland/portable-archive limitations remain. Earlier allocator
attribution established predominantly free glibc arena storage, and the owner does
not currently consider retention a defect. No allocator tuning was made.

The parameter-only narrow test is an implementation decision following discussion
of the scenario, not a separate owner policy or waiver. Source is frozen after
clean review. This final update changes only plan evidence. Current remote CI,
PR refresh, commit/push and final owner assessment remain pending. No publication,
Ready transition, merge or release occurred.

### Approved remaining DSP test cost optimization

Class A. Coordinator and executor remain `gpt-6.1-sol` medium, with a fresh
read-only `gpt-6.1-sol` high reviewer arranged by the coordinator. The owner
approved the remaining suppression noise-floor, progressive hop-one cancellation
and two extraction fixtures in this PR. Preserve prior dirty work. No commit or
push is authorized. Consultations now require the user-installed Codex CLI with
`gpt-6.2-astra` high in a read-only sandbox. The preceding targeted consultation
was `gpt-6-astra` high, not `gpt-6.2-astra`.

- [x] Share private suppression band geometry between FIR synthesis and actual
  noise-floor measurement, retaining the original noise/tone input and assertions.
- [x] Retain actual hop-one progressive cancellation with FFT 2048 and batch 4,
  checking the exact bounded read count. Keep large allocation arithmetic coverage.
- [x] Retain true long-FIR independent frequency-response oracles. Exercise real
  mirror rejection and complex near-whole rejection through smaller, genuinely
  steady-state streams sized from actual margins and partition boundaries.
- [x] Measure focused before/after cases, then run the full required local gate
  once on final source. Preserve exact dirty-source identity and narrow delta for
  independent review. Do not disrupt the running application or Xvfb.

Focused evidence is in `/tmp/argand-259-242-evidence/remaining-fft/`.
The shared private geometry removes six unused 262144-point transforms from the
noise-floor case while preserving its 262144-sample complex/real noise and tone
inputs and genuine measurements. The tone at 6001 Hz is inside 5991..6011 Hz, so
the test name now says inside. All public constructor/fill integration tests remain.
The hop-one cancellation case now reads exactly 2051 samples at FFT2048/batch4.
Large allocation arithmetic coverage is unchanged.

The original real 1..2001 Hz filter retains its independent wanted/image response
bound at -999/-1003 Hz, and the original complex -23990..23990 Hz filter retains
its independent 23997 Hz rejection bound. Actual representative streaming still
checks mirror/rejection behavior with unchanged tolerances. Its 31457/14233-sample
fixtures derive from actual margins, two partitions and a partial final partition.

Separate one-test processes measured before/after wall times of 1.08/0.28 seconds
(noise), 0.30/<0.01 (cancellation), 0.79/0.12 (real) and 0.30/0.07 (complex).
Process max RSS was 29220/14572, 36648/8384, 34552/9344 and 22780/8564 KiB,
respectively. These single observations include harness cost, not incremental FIR
storage or universal speedup. The corrected baseline used the original binary;
its source-patch metadata was captured after edits. The first baseline's zero-test
cancellation filter was discarded. After timings precede only the mechanical
Clippy direct-return correction of the same iterator expression. The final full
suite covers the corrected exact source.

Final formatting and all-target Clippy passed without suppression. A separate
workspace no-run compile passed. Exactly one ordinary full test run passed all
942 tests in 64.20 seconds, app564/46.07s and DSP106/12.68s. Same hard 1 GiB,
no swap, 256 tasks, 180 seconds, native CPU availability and default concurrency.
Cgroup peak was 915.1 MiB with all memory events zero. App sampled maxima were
994060 KiB RSS, 919983 KiB PSS, 821484 KiB Anonymous and 67 threads. These
accounting views are not additive. The preceding snapshot was 46.77 seconds,
app29.98s, DSP12.54s and906.5MiB: no whole-suite gain or regression attribution
is claimed, and no repeat was run to seek better timing.

Release ran last under 3 GiB/no swap/256 tasks. First attempt timed out at the
procedural 180-second deadline with no diagnostics, leaving the old application
binary. One unchanged-limits retry passed in 177.36 seconds. Both attempts are
preserved. Final application SHA256 is
`27a66ff728dc008953995880c251635a40d552cce8b1dadf641d5de61b3f076d`.
It differs from the prior runtime binary, so that exact-binary runtime evidence
is not reused. No new UI runtime claim or application/Xvfb disruption occurred.
The exact validated revision is recorded in the external evidence manifest,
with tracked patch SHA256
`29cce1ce31fd3b41849b4a5a46198b953e32855d97d5ab33c488bcd400df80c0`.
All nine Rust file hashes match the corrected independent-review snapshot.
The prior GUI startup binary hash does not validate this current executable
because production suppression code changed. No new UI acceptance is claimed.
The current consultant policy was synchronized to the owner-approved 6.2/CLI rule
in AGENTS and the current role assignment; historical consultation attribution
remains 6-astra high. These post-gate changes are documentation only. Fresh review
assessment is coordinated separately. Final packet changes after validation are
documentation only, including owner-approved consultant instructions. No commit, push, Ready transition, merge or release publication.

### Final approved test setup cleanup

Class A. Coordinator/executor remain `gpt-6.1-sol` medium and the fresh read-only
reviewer is `gpt-6.1-sol` high. The owner approved exactly the three remaining
setup wastes identified by the actual user-installed CLI consultation using
`gpt-6.2-astra` high. This is separate from the earlier `gpt-6-astra` high advice.
After these changes, stop optimization. No new Issue/worktree or commit/push.

- [x] Reuse the selected polyphase backend when it matches the forced test kind.
  Retain actual direct/partitioned comparison, decimations, chunks and sparse FIR.
- [x] Avoid unused Kaiser synthesis in three test plans whose taps are replaced.
  Preserve native centre, every export field, actual taps and numerical oracles.
- [x] Build the existing 2.4M-sample two-tone application fixture in one pass,
  preserving each independent f32 conversion/multiply and f32 sum, exact phase,
  span and all 60006 output comparisons. Check full input bit equality temporarily.
- [x] Record isolated before/after costs, obtain fresh exact-snapshot review, and
  run final formatting, all-target Clippy, ordinary workspace tests and release
  last. Keep all setup/dependencies/SDK/local target unchanged and preserve the
  owner's app/Xvfb session. No further optimization search.

The exact baseline and final source/release identities are retained outside the
worktree in `/tmp/argand-259-242-evidence/setup-waste-final/`. The polyphase
change removes 24 unused forward FFT65536 operations by source count. Three test
plans avoid a total 1973328 unused Kaiser coefficients while preserving their
actual centre, decimation/export/interpolation fields and replacement taps.
Genuine whole-export constructor integration cases remain unchanged. The input
fixture removes a 19200000-byte temporary (18.3 MiB), without reducing its size.
A temporary comparison verified all 4800000 f32 values bit-identical, including
independent casts/multiplies and the final f32 addition. No permanent test added.

All six original and modified isolated cases passed. Before/after wall seconds
and process max RSS KiB were: odd/even0.38/0.41 and8868/8548;
sparsepolyphase2.90/2.02 and26944/19508; wholeboth0.63/0.37 and13488/8900;
wholesparse1.05/1.76 and18752/17012; mixing0.25/0.11 and13216/9292;
app2.20/2.84 and166228/164528 (162.3/160.7MiB). These single observations are
mixed, include harness/runtime cost and do not establish a generic gain. Removed
operations/storage are source-level facts, not claimed measured RSS savings.
No repeats were made to seek prettier numbers.

Final fmt, all-target Clippy and separate workspace no-run compile passed.
Exactly one ordinary full test run passed942tests in58.93s, app564/41.05s,
DSP106/12.49s. Same1GiB/noSwap/Tasks256/180s/nativeCPU/defaultparallelism.
Cgrouppeak898.9MiB, allmemoryevents0; app sampledRSS981092KiB/PSS907174/
Anonymous809144/59threads. These accounting views are not additive. Previous
64.20s/915.1MiB and earlier46.77s/906.5MiB runs show variability, not causal
whole-suite improvement from this change.

Release ran last and passed97.91s under3GiB/noSwap/Tasks256/CPUWeight10 with
an adequate600s procedural deadline. Its application SHA256
`27a66ff728dc008953995880c251635a40d552cce8b1dadf641d5de61b3f076d`
is byte-identical to the preceding product build. No new GUI acceptance is claimed.
Fresh source/evidence review by `gpt-6.1-sol` high was clean in its first round,
with exact source hashes, all942pass and full input-bit equality checked.
The final handoff binds these results to unchanged reviewed source hashes.
After validation, only documentation/evidence changed. Optimization stops here.
All earlier whole-PR GUI/currentCI/owner-acceptance limits remain; no commit,
push, Ready transition, merge or release publication occurred.

## Historical transfer handoff — stopped at the owner's request

The owner explicitly requested a commit and push without more tests so work can
continue on another computer. Preserve this unfinished joint branch. Do not move
this plan to completed or treat the snapshot as an accepted release candidate.

Implemented: bounded whole-band, partial-IQ and real export computation with the
existing FIR/phase/80 dB contracts; conversion retains its original spectral path.
Numerical export tests use the same production computation with an in-memory sink.
Test-context cleanup, complete AppCell release and workerless state/UI fixtures
are retained. Four conversion-panel fixtures use genuine tiny-file metadata,
format/stamp/units and finished statistics without an unused analyst/FFT pool.
Ordinary conversion/property and shared real-file windows explicitly use scale 1;
logical bounds/assertions and dedicated HiDPI tests remain unchanged. Linux archives
now copy the executable without system shared libraries or a library-path launcher;
Rust/asset notices and Windows packaging are retained.

Focused DSP, UI, packaging and policy checks passed at their recorded revisions.
The latest ordinary default suite reached **554 of 564 application tests**, then
was OOM-killed at the unchanged **1 GiB** test limit after about 76 seconds. Its
sampled maximum was **135 threads**, with twelve native analysts and eight FFT
workers each. This is not an accepted full gate. Compilation had separately
passed under a compiler-only 3 GiB bound; compiler memory is not test memory.

The next proposed correction is `Config.analysis.workers = 2` only in the three
real-workflow test constructors (replacement, property panel and editing). The
owner approved that test-only choice, but the edit was interrupted before it ran:
**two-worker fixtures are NOT implemented and NOT validated in this snapshot**.
Product automatic worker defaults, analysis scheduling tests and default libtest
parallelism remain unchanged. Resume by inspecting these constructors and the
remaining rendering/live-versus-allocator memory question, then perform focused
validation before another bounded default run. Do not infer exact heap ownership
from the aggregate memory/thread samples.

Included new source modules: `crates/dsp/src/extract_fir.rs`, `extract_whole.rs`,
`extract_polyphase.rs`, `extract_polyphase_tests.rs`, and
`crates/app/src/shell/document_support_test.rs`. Existing source/test/documentation
changes and withdrawn release-script/probe deletions belong to the same snapshot.
Task-owned test/build services were absent at stop; unrelated running applications
were left alone. The original pre-push hook remains unchanged. Only this transfer
push skips it, explicitly authorized by the owner.

Pending: accepted full local gate, final release build, current system-library GUI
startup, final aggregate review, updated PR description and current full remote CI.
No merge, Ready transition or release is authorized. Draft PR #267's public
description remains outdated (withdrawn administrator/scripts architecture and old
DSP scope); rewrite it when work resumes, not during this stop-and-transfer action.
Machine-local SDK selection and target artifacts are not part of the transfer.
The other computer must use its owner-approved Rust/SDK setup under repository
instructions, without automatic toolchain/SDK installation. Local raw diagnostics
are supplementary; copying target/ or temporary evidence is not required to resume
from the tracked code, scope, outcomes and next steps above.

## Current owner-directed scope

The owner resumed #259 with a minimal change: Linux archives must not carry
system shared libraries or override the process library path. Copy the existing
executable directly, retaining archive names, Rust/asset notices and Windows
packaging. Remove the native closure/copyright collection and launcher rather
than introducing another dependency list, probe or packaging framework. The
operating system supplies its compatible runtime libraries. Broader distribution
format work remains in #86.

Class A roles remain Coordinator/Executor `gpt-6.1-sol` medium and independent
Reviewer `gpt-6.1-sol` high. Preserve all approved DSP and test-resource work.

- [x] Remove Linux native-library copying and its LD_LIBRARY_PATH launcher.
- [x] Adapt the existing archive regression to direct executable bytes, absent
  bundled libraries and successful extracted execution. Keep other packaging tests.
- [x] Synchronize installation/release documentation without new library lists.
- [ ] Validate packaging cheaply, then verify the final production binary with
  system libraries in an isolated display/config session after the local gate.
- [ ] Assess one bounded default full-test run and complete the local gate, release
  build last, without overlapping builds or duplicate test runs.
- [ ] Obtain current review, update plans/PR and commit/push for required full CI.

### Remaining parallel UI setup cost

The bounded default full suite hit its 1 GiB hard limit before completion; no
release build or retry follows that failed gate. Four conversion-panel UI tests
passed separately but retained substantial anonymous memory while running together.
Replace only their incidental real-reader setup with the existing prepared Readers
fixture and identical described metadata/statistics. Preserve every UI assertion,
real conversion/save/cancel/adoption test, production worker policy and default
libtest parallelism. Compare exactly those four cases once under identical hard
limits before considering another full gate. These measurements do not attribute
all earlier full-suite memory to analysis workers.

The subsequent default run still reached the 1 GiB test limit. Real conversion
integration fixtures retain their real readers, conversion, publication/adoption
and cancellation. Their display setup now explicitly chooses scale 1 while keeping
logical window bounds and interaction assertions. The toolkit's incidental default
scale 2 multiplies physical pixel planes by four. Dedicated HiDPI tests remain unchanged. A subsequent late disk-change-family
peak justified the same 1x choice in the shared real-file workflow window;
redundant local conversion overrides are removed. Other independent test-window
constructors remain unchanged. This is a test-display choice,
not a production renderer, DSP, worker or harness-concurrency change. Verify one
real converted-document case, then the ordinary default suite under the same cap.

### Historical scope and evidence

The earlier broad exclusion approach and extra runtime prerequisites were
withdrawn. The earlier decision deferring all native packaging to #86 is superseded
by the minimal removal above. Historical evidence below retains its original
revision limits and does not establish final system-library startup.

The current Class A scope retains the test-context cleanup and complete AppCell
release assertion. The owner withdrew the proposed formatting-only hook and
standalone local-gate wrapper. Pre-push retains its original formatting, Clippy
and workspace-test sequence; the documented full local gate remains unchanged.
No validation stamps, cache bypasses or additional commands are introduced.

Read-only call-graph inspection found no release dependency-list generation in
the hook or Cargo build script. The app build script validates and links the
explicitly selected immutable SDK; it does not package libraries. Release native
closure and license inventories remain in the separate packaging script used by
remote CI/release workflows, which are preserved for the deferred archive work.
Necessary SDK compiler/target/link validation remains intact. Shell syntax and
diff whitespace checks cover this reversal; no real Rust gate was rerun.

### Earlier approved removal of unnecessary test workers

The owner approved a narrow test-only correction before further memory attribution.
That earlier scope kept production analysis/DSP, numerical precision, default workers, the restored
hook and all real I/O/save/cancellation/reader tests unchanged.

1. Add a test-support Analyst fixture with the real bounded mailbox and retained
   request receiver, without starting a native reader or FFT pool. Preserve real
   request_view, generation/view revision and accepts transitions.
2. Use it in the six open-file acceptance tests and progress-delivery bookkeeping
   test that currently start unused real workers. Preserve every assertion.
3. Inspect fake-document menu/editing/selection helpers for the existing prepared
   Readers injection boundary. Keep real UI state/layout/focus/history, and keep
   the inert request receiver owned by the fixture's delivery task.
4. Verify exact existing Rust/immutable SDK selection and compile affected app
   tests incrementally once. No full suite, source-toolkit rebuild or containers.
5. Propose each isolated diagnostic binary/filter and temporary RSS/PSS/thread-name
   sampling with runtime/resource caps to the coordinator before execution. Bind
   evidence to committed revision plus dirty diff identity. No permanent monitor.

The subsequent whole-band export expansion below changes production DSP only as
explicitly approved by the owner. The earlier numerical-test-only restriction is
superseded for that path.

This remains Class A. Coordinator and Executor use `gpt-6.1-sol` medium;
independent Reviewer uses `gpt-6.1-sol` high. Numerical export boundary tests are
preserved: their large production FIR/FFT cost is separate from unnecessary UI
worker startup, and the full-suite memory peak remains unattributed.

### Focused workerless validation and memory evidence

The state-only fixture uses the real Analyst mailbox and retained receiver. Six
open-file acceptance tests and one progress bookkeeping test passed. Fake setup
for menu, editing, selection, frame observer and analysis/editor hints now uses
the existing prepared Readers boundary with a scope-owned inert delivery task.
Real open-action, dirty-prompt, file-close and async worker paths stay real.

Four serial fresh-process UI groups passed: menu 27, editing 11, selection 16,
and frame/hints 18, for 72 UI tests. Each ran under hard 512 MiB/no-swap/one-CPU,
32-task and 60-second limits. All cgroup memory-limit/OOM events were zero.
Sampled RSS peaks were about 165–169 MiB, PSS 74–76 MiB and anonymous memory
12–14 MiB. Large shared/file mappings account for much of RSS in these groups.
The one-CPU cap also reduces automatic native pool size in retained real action
paths, so these thread counts do not predict an unrestricted parallel full suite.

One separately authorized real DC/Nyquist export test passed under hard 512 MiB,
no-swap, one-CPU, 16-task and 25-second limits. It used two threads and reached
about 441 MiB sampled RSS, 379 MiB PSS and 337 MiB anonymous memory. Its cgroup
peak was about 347 MiB. These are different accounting metrics, not additive.
The test exercises the production 4M/8M FFT export filter, not ordinary spectrum
visualization. Its exact numerical assertions remain unchanged.

Formatting and app all-target Clippy passed. The initial formatter failure was
only a long fixture-helper signature, corrected without semantic change. No full
suite or release/source-toolkit build was repeated. Timestamped stdout chunks,
RSS/PSS/file/anonymous/thread samples, GNU time and cgroup events are preserved
outside the checkout with binary and dirty-source identities. Observed pipe
output timestamps do not establish exact test-body boundaries. The earlier full
suite memory/thread maxima remain unattributed and unaccepted by the owner.

The previous full gate passed 931 tests but peaked at 2,638,984 KiB RSS
and 138 threads in the app binary, which the owner has not accepted. A subsequent
bounded two-test diagnostic proves complete AppCell release; the retained weak
assertion does not attribute the full-suite resource peak.

Roles remain Coordinator `gpt-6.1-sol` medium, Executor `gpt-6.1-sol` medium,
and independent Reviewer `gpt-6.1-sol` high. The owner-directed scope change
follows the previous clean third review and requires scoped review of
the new diff, rather than treating the old review as covering these edits.

## Owner-directed whole-band export optimization

Class A: production DSP correctness and public extraction construction. Roles:
Coordinator and Executor `gpt-6.1-sol` medium; independent Reviewer
`gpt-6.1-sol` high. Existing release/test-cleanup work remains preserved.

- [x] Preserve the existing whole-band IQ Kaiser FIR, exact native even samples,
  odd interpolation phase, physical mixing, margins and output coordinates.
- [x] Add a conservative generic constructor and a validated input-support option
  for callers that prove capture-boundary zero padding. Select limited-output
  direct convolution only when the bounded actual support/odd-output overlap is
  small. Do not infer zero input from selected span length.
- [x] Use uniform 32768-sample partitions and 65536-point FFTs for longer whole-band
  convolution. Both filter spectra and input spectra rings have K complete FFT
  slots, K=ceil(taps/32768). For the current FIR K=21, their combined payload is
  about21 MiB, plus2.51 MiB taps and about2.76 MiB native-delay ring,
  transform/output buffers and small FFT tables/scratch. Only partitions
  intersecting the requested causal output interval accumulate spectra and
  perform an inverse transform; input transforms still populate the history.
  This is an allocation calculation, not a measured RSS promise.
- [x] Keep nonwhole extraction/interpolation unchanged. Bound partition work and
  caller batches so existing cancellation checks remain useful.
- [x] Separate the existing production extraction/resampling/domain-mixing stream
  from its FloatWave sink, without another file writer. Move numerical edge/tail
  tests to memory input/output through that same computation.
- [x] Retain justified file integration: stamped source reopening/final identity,
  cancellation and target protection, real capture margins and output metadata/
  domain wiring. FloatWave/EncodedWave format/container tests remain in I/O.
- [x] Verify direct/partitioned/reference convolution, boundary/chunking/native
  sample identity, the same numerical export regressions and bounded resource use.
  Four small reference/index tests passed, including an independent sparse three-
  partition FIR and coordinates around the32bit boundary. The exact whole-band pair passed with original
  tolerances; eleven extraction safety/integration/numerical tests passed, including
  newly explicit source-change-after-read and cancellation-before-publication cases.
- [ ] Obtain fresh exact-snapshot independent review. Full gate/current CI and owner
  acceptance remain pending; no full suite, push or release is authorized yet.

First scoped review found two accepted P2 gaps: native-ring read coordinates cast
before modulo on32bit, and missing independent multi-partition reference coverage.
Read/write now share u64-modulo indexing; arithmetic boundary and sparse three-
partition reference tests cover these corrections. Four bounded DSP tests pass
without rerunning heavy app cases; final relevant formatting/Clippy pass. The app
resource evidence predates only the64bit-equivalent index helper and added tests.

The exact two-test pair under the same 512 MiB/no-swap/one-CPU/16-task/25-second
limits changed from 12.70 to 2.42 seconds of libtest time. Sampled anonymous peak
changed from 439476 KiB (429.2 MiB) to 61896 KiB (60.4 MiB); RSS from 548076 to
167924 KiB, PSS from 484633 to 103165 KiB. Separate cgroup charged peaks were
479350784 and 207306752 bytes, with zero memory-limit/OOM events in both runs.
This measures the combined algorithm and in-memory numerical-test change, not an
isolated FFT speedup. It does not explain or accept the prior full-suite peak.
The eleven-case serial group still exercises the unchanged real/nonwhole 4M/8M
path and reached 341952 KiB anonymous memory. That path is outside this whole-band
IQ optimization. FloatWave still has one format writer; EncodedWave wraps it.
Numerical memory tests run the production extraction, resampling and domain mixing.
Disk integration remains for metadata/domain wiring, capture-neighbour margins,
resolved normalization, stamped source checks and cancellation/target protection.

## Owner-directed remaining export optimization

The owner explicitly extended this Class A work to every remaining export path,
including partial IQ and real DC/Nyquist. Earlier whole-band-only restrictions
are superseded. Role assignments remain Coordinator/Executor `gpt-6.1-sol` medium
and independent Reviewer `gpt-6.1-sol` high; targeted DSP consultation assessed the
finite polyphase design. Conversion keeps its existing spectral interpolation.

- [x] Preserve integer-phase FIR coefficients, gain, cutoff, physical oscillator,
  decimation, margins and output coordinates. Construct an export-only half phase
  from the SAME continuous Kaiser prototype: h1[j]=f(j-m+0.5), j=0..2m-1,
  window radius m. Separately normalize its DC gain to the integer phase's gain.
- [x] For output n, let q=n*D, k=floor(q/2), p=q%2. Evaluate phase p at causal
  input coordinate 2m+k, with mixing referenced to the requested span. No
  half-phase allocation/calculation is needed when D is even.
- [x] Share bounded input FFT history among phase filter banks and the whole-band
  implementation. Use direct limited-output work only under a conservative
  multiply-add bound. At worst two filters plus one history retain about31.5 MiB
  principal spectra and about5 MiB coefficients, plus small buffers/plans. This is
  an allocation estimate, not measured RSS. Avoid monolithic4M/8M export FFTs.
- [x] Retain the single production compute/sink and safety shell. Finite polyphase
  reconstruction replaces the N-dependent periodic interpolant, so exact bitwise
  equivalence to that intermediate is not claimed. Required amplitude, phase,
  80 dB response, image rejection, alignment, short tails and original test
  tolerances must hold. No acceptance relaxation is authorized.
- [x] Validate f64 combined wanted/image responses (R0 +/- R1)/2, independent
  direct/sparse references for both phases, off-centre mixing and partition/final
  boundaries. Keep real DC/Nyquist, near-edge and short-tail numerical regressions.
  Conversion regressions must establish that its existing route stays selected.
- [x] Run isolated bounded checks and bind evidence to exact source/binaries:
  ten focused DSP checks pass, including original near-Nyquist/native-boundary,
  odd/even D, sparse K3, short1/2/7sample tails and conversion dispatch. Twelve
  final app safety/numerical cases pass. Separate real DC/Nyquist passes with the
  original tolerances. All hard512MiB/no-swap/one-CPU/16-task/25slimits held.
- [ ] Obtain fresh exact-snapshot independent review. Full gate/current CI and
  owner acceptance remain pending. No publication, commit/push or further run.

Short FIRs use adaptive power-of-two partitions256..32768, avoiding a65536point
transform for ordinary few-hundred-tap filters. Every threshold, clip, ring and
history uses the selected partition; largest FIRs retain the measured P32768.
The exact real-edge case changed from12.44s/RSS451328/PSS387832/anonymous344908KiB
to6.59s/RSS156980/PSS107796/anonymous51188KiB. Separate cgroup charged peaks were
363929600 and207486976bytes. This comparison combines computation and memory-test
routing, not an isolated algorithm speedup or a full-suite resource result.
The final adaptive10-test DSP group took13.57s vs5.16s before adaptation; even
unchanged fixed-P/response cases took longer. Cause is unverified, and no generic
speedup or host-load explanation is claimed. Final app12case group passed5.35s.

Final relevant fmt/Clippy cover the final source. The initial lint check found
excessive nesting in shared-bank accumulation; the same arithmetic loop was
factored into a method without suppression or further numerical changes.
The real-edge run predates adaptive sizing: its largest phase still selects
P32768/K21 with the same arithmetic. App/DSP runtime snapshots predate only the
lint-method extraction and subsequent documentation. No heavy repeat is claimed.

## Overview

Keep the Linux host display libraries coherent with its graphics drivers instead of placing older display libraries ahead of them in the application launcher.

The owner requested Issues #259 and #242 together in one separate linked worktree, based on the main checkout's local branch. No merge or release is authorized by this implementation task.

## Roles and risk

- Class: A, release authority and cross-host runtime safety.
- Coordinator: `gpt-6.1-sol` medium, owner communication and final assessment.
- Executor: `gpt-6.1-sol` medium, investigation, implementation, validation and PR operations.
- Reviewer: `gpt-6.1-sol` high, fresh read-only context.
- Consultant: `gpt-6.2-astra` high through the user-installed Codex CLI with
  `codex exec --model gpt-6.2-astra -c 'model_reasoning_effort="high"' -s read-only`,
  only if a concrete uncertainty requires it.

## Context and decisions

- Preserve the approved Rust/toolkit graph and immutable installed SDK. All Cargo artifacts stay in this worktree's target directory.
- Preserve main branch protection and Release tags ruleset. No secrets, new bypass actors, force tag updates or production release during development.
- Owner-authorized tag pushes trigger the existing release workflow; GitHub controls actor authorization. The separate release CLI and retry orchestration were withdrawn by the owner.

## Superseded packaging investigation evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| Host display libraries excluded throughout closure | Ten packaging tests, including synthetic transitive SONAME closure and real packaged ELF | Passed | Host GPU runtime is separate |
| Native notices and other bundled libraries retained | Real archive closure/notices regression | Passed | CI baseline packaging still pending |
| Vulkan ICD instance/device/desktop extensions work under launcher environment | probe-linux-backend.py on extracted archive | Pending | Probe does not create a rendering surface |
| Baseline Ubuntu 24.04 production GUI opens capture | Existing Ubuntu24 image identified with Wayland1.22/Mesa24 | Pending | Desktop runtime not exercised yet |
| Newer Wayland/Mesa production GUI opens capture | Ubuntu26.04 host with ICDs and isolated Xvfb available | Pending | Startup/real frame evidence not exercised yet |
| Existing application behavior | fmt, Clippy, workspace tests and release build | Pre-cleanup gate passed, refreshed gate pending | Local SDK gate, portable build separate |

## Superseded packaging implementation steps

- [x] Inspect native dependencies and define the host display-stack boundary with explicit baseline requirements.
- [x] Exclude host display-stack libraries from every transitive bundle closure and retain native notices for bundled libraries.
- [x] Add regression checks and a real Vulkan loader/device probe using the packaged library environment.
- [ ] Document and record baseline/newer-host backend evidence and limitations.

- [ ] Complete independent review and relevant validation.
- [ ] Move this plan to completed before final review when all acceptance evidence exists.

## Owner-approved validation cleanup

The owner expanded this same PR to fix the analyzer/thread accumulation observed
in the existing GUI test harness, without a new Issue. Installed GPUI Kit's test
macro delegates teardown to App::quit on a no-op test platform. Its genuine
TestAppContext::quit cleanup is not called, leaving raw VisualTestContext owners
and document readers retained. This is a verified static retention path consistent
with the observed analyzer/pool multiplicities, not per-instance heap attribution.

The repository correction is test-only: one shared RAII cleanup scope, created
before each GPUI test's other locals, calls the existing genuine test-context quit
API after those locals drop. It also runs during unwinding, preserves the original
panic, and propagates cleanup failures on otherwise successful tests. No registry,
immutable SDK, dependency graph, production worker policy or test assertions change.

- [x] Add shared test-context cleanup and first-local guards to all app GPUI tests.
- [x] Prove real Shell/capture ownership and reader release across repeated normal and panic scopes.
- [x] Observe a bounded serial subset returning analyzer/pool threads to baseline before any full gate.
- [ ] Repeat full gate and independent review only after focused lifecycle evidence passes.

The pre-fix single-case regression failed with "test window retained its Shell".
After cleanup, both focused tests passed: three normal document scopes and one
intentional unwind, with the exact original panic checked. Each scope releases its
real Shell weak handle and every existing reader lease within a bounded deadline.
A serial temporary process observation sampled 385 points: at most one analyzer
and eight FFT workers, with three observed returns to zero between scopes and a
successful final process exit. This is task evidence, not a permanent monitor or
a fixed global-thread-count assertion in the parallel test suite.

The first refreshed parallel gate reached 559 passing app tests and three scheduler
failures from native reader channel-close wakeups during context shutdown. It did
not run the release build. Its measured peak was 2,994,984 KiB RSS and 142 threads;
this still reflects substantial concurrent test work and is not claimed as a
complete memory-cost fix. The correction permits native I/O wakes only inside the
cleanup scope immediately before real shutdown, then drains cancelled listeners.
It does not clear recorded test-body determinism errors. All three affected cases
and both lifecycle regressions passed together in the focused follow-up. Pending
receiver-listener removal and native notification share event-listener's mutex,
so draining cancellation also synchronizes with an in-flight native wake.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, last
- [ ] Change-specific policy and integration checks.

## Post-completion

Only the owner authorizes merge and production release. Broader distribution
formats remain in #86. The known published bundle defect remains until the
system-library packaging change is validated and released.

</details>
