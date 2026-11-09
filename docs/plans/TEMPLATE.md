# Issue #N: Short descriptive title

Resolves #N.

## Overview

Describe the problem, desired outcome, and boundaries of the change.

## Roles and risk

- Class: A / B / C, with a brief risk rationale (see `AGENTS.md`).
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

These defaults are owner-approved. Record any owner-approved deviation. For work
without code, record the coordinator's workflow and validation decisions when a
formal plan is warranted; this template does not itself require one.

## Context

Record relevant architecture, constraints, prior work, and affected files.

## Decisions

- Record material implementation decisions and their rationale.

## Rejected alternatives

- Record meaningful alternatives and why they were rejected.

## Acceptance criteria and evidence

State independently observable outcomes. Map each criterion to its verification,
result and limits; distinguish automated checks, native application evidence and
unperformed checks. A test count alone is not evidence that a criterion is met.

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Observable outcome | Independent check and relevant inputs | Pending / observed result | Evidence boundary |

For UI tasks, use the [component selection](../../CONTRIBUTING.md#ui-component-selection)
and [running-application acceptance](../../CONTRIBUTING.md#ui-acceptance) requirements.

### Numerical examples for DSP changes only

Omit this subsection for tasks that do not affect DSP semantics. Before implementing
DSP changes, give concrete input/output examples with expectations derived
independently of the implementation. Include sample domain and Fs, frequencies or
spectrum placement, amplitude and its units, sample count/duration, reference
metadata and the expected saved/reopened result. State tolerances and relevant
boundary cases; do not substitute the algorithm's own output for an expected result.

For example, whole-capture normalization of a real float32 cosine with a target
peak of -0.1 dBFS can specify:

| Quantity | Input | Expected output |
| --- | --- | --- |
| Domain and sample rate | Real, Fs = 8000 Hz | Real, Fs = 8000 Hz |
| Spectrum | 1000 Hz cosine | 1000 Hz cosine; no frequency shift |
| Peak amplitude | 0.5 on nominal full scale 1 | 10^(-0.1/20), approximately 0.988553; tolerance 1e-6 |
| Length | 8000 samples, 1 second | 8000 samples, 1 second |
| Reference metadata | 123000 Hz | 123000 Hz |
| Save/reopen | Save As to a new float32 WAVE path; original source stays unchanged | Saved/reopened output retains rate, reference, count and expected peak within tolerance |

Choose examples for the operation actually proposed; this example does not add
normalization or conversion behavior to unrelated work.

## Implementation steps

- [ ] First independently verifiable task.
- [ ] Second independently verifiable task.
- [ ] Update relevant documentation.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

The checks below are required for code changes. For work without code, replace them
with the coordinator's risk-based validation decision and explain omissions.

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Change-specific manual or automated validation.

## Post-completion

List actions that can only happen after merge. Do not use unfinished implementation checkboxes here.
