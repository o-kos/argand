# Issue #259: Linux Display Stack

Tracks #259 (https://github.com/o-kos/argand/issues/259).

## Transfer handoff — stopped at the owner's request

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
- Consultant: `gpt-6-astra` medium only if a concrete uncertainty requires it.

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
