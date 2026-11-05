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
