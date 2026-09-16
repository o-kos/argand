# Issue #44: README badge claims Rust 1.88 while the workspace needs 1.97

Resolves #44.

Complexity class **C**: implementer `gpt-5.6-terra` at high reasoning effort,
reviewer `gpt-5.6-sol` at high reasoning effort, per "Agent roles and model
selection" in `AGENTS.md`.

## Overview

`README.md` carries a badge reading **Rust 1.88+** linked to `rust-toolchain.toml`.
The toolchain pins `1.97.1` and `workspace.package.rust-version` is `1.97`, both
raised when the GUI toolkit was adopted. Anyone who reads the badge and installs
1.88 gets a workspace that will not build.

Out of scope: the toolchain version itself, the other badges, and any other README
content.

## Context

- `README.md:9` holds the badge:
  `<img src="https://img.shields.io/badge/rust-1.88%2B-dea584" alt="Rust 1.88+">`,
  wrapped in `<a href="rust-toolchain.toml">`.
- `rust-toolchain.toml` sets `channel = "1.97.1"`.
- `Cargo.toml:8` sets `rust-version = "1.97"`.
- The badge states the minimum supported version, so it follows
  `workspace.package.rust-version`, not the pinned toolchain patch level.

## Decisions

- **The badge reads 1.97**, matching `rust-version`. That is the minimum the
  workspace declares it builds with; `1.97.1` is the exact toolchain this repository
  develops against, which is a stricter thing and not what a "1.97+" badge promises.
- **Both the image URL and the `alt` text change.** A screen reader and a text-mode
  browser read the `alt`, so leaving it saying 1.88 would keep the wrong claim for
  exactly the readers who cannot see the badge.

## Rejected alternatives

- **Badging 1.97.1.** It would go stale on every toolchain patch bump and overstates
  the minimum.
- **Generating the badge from `Cargo.toml`.** Shields.io can read a manifest from a
  repository, but that adds a network dependency on a private detail of this
  repository's layout to fix a two-number error.

## Implementation steps

- [ ] Update the badge URL and its `alt` text in `README.md` to 1.97.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] Confirm the rendered badge reads 1.97 and still links to `rust-toolchain.toml`,
      and that no other occurrence of 1.88 remains in the repository's documentation.

## Post-completion

- None.
