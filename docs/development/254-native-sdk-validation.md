# Native SDK layout validation (#254)

Validated on 2026-11-13 using the existing Rust 1.98.1 and GPUI Kit 0.9.1 SDK.
No dependency versions, features or profiles changed. The installer verified every
artifact against the existing immutable installation after validation.

## Result

Cargo's debug and release JSON messages each reported 177 compiler artifacts,
with no target-triple output paths and no toolkit compilation. The executable
paths are the ordinary workspace-local target/debug/{argand,aspec} and
target/release/{argand,aspec}. Both debug and release binaries passed --help;
debug argand was also invoked through ordinary cargo run --locked -p argand.

The app's ELF dynamic section names libgpui_kit_sdk.so and the immutable SDK
runtime path. The release CLI has no GPUI SDK dependency. A direct RenderImage
probe and the full app build verify the SDK's exact image::Frame type boundary.

## Checks

- Six SDK Python selection/safety unit tests passed.
- Six bounded build-support cases passed, using only tiny dependency-free fixture
  code: native layout, host flag isolation, removed image/artifact, compiler mismatch,
  missing selection and source-mode bypass.
- cargo fmt --all -- --check passed.
- cargo clippy --all-targets --locked passed.
- cargo test --locked: 898 tests passed with no failures; doc tests passed.
- cargo build --locked and the subsequent cargo build --release --locked passed.
- The release application rendered its actual empty start page on an isolated,
  identity-checked Xvfb display with separate XDG directories and D-Bus session.
  No launch/input used the owner's display. This is a startup smoke check, not a
  claim of a new UI acceptance matrix or native Wayland/Windows/macOS verification.

## Compiler scope and identity

The SDK library search and rpath directives appear only in three commands for
the app package, including its benchmark. Independently inspected verbose output
shows 33 host build-script/procedural-macro commands and 151 other dependency/CLI
commands without those SDK flags. No compiler invocation uses --target.

The native graph initially exposed a StableCrateId collision between separately
built rustversion artifacts in the immutable SDK and the consumer. A fixed additive
-Cmetadata=gpui-kit-sdk-client-v1 namespace separates consumer identities while
preserving Cargo's per-crate metadata. A consultant's installed-compiler scratch
proof reproduced the collision, resolved it with the extra metadata, and retained
two distinct initial metadata values under a shared namespace. The full native
Clippy/test/debug/release gates then passed. The namespace carries no SDK extern,
search or linker configuration into host compilation.

The validated source was the plan commit plus the submitted local changes. The
source checksums below bind the tested code independently of later documentation
and commit bookkeeping. Recheck affected evidence after any source/config change.

| Source | SHA256 |
| --- | --- |
| `crates/app/build.rs` | `d4b710f233b2fb78311f5a56a5acdaa8a4f09480ea8b7f6ef379ad82b717f1c4` |
| `crates/app/src/main.rs` | `4f26149431494448ac6ec94ad2b0b8f33035587a51b976e0cb5e68ca35f8d67b` |
| `tools/gpui-sdk/build_support.rs` | `43de6a65bb2c967d62227b2474058f4e0701586346b166bc889899f4a33c5887` |
| `tools/gpui-sdk/sdk.py` | `9962b8645c98c9bbec6446633c3274e5a7a8dde08a51076a0019b6e31dc0ada2` |
| `Cargo.toml` | `596cec45a4fe2a8e83582fe82e634d73d6c2cd3a6aad2491d2659d564ca13f82` |
| `Cargo.lock` | `eb59996667733c33971f21978539ddb9eff04487f9f30b07443bb7a0ff4418f4` |
| `rust-toolchain.toml` | `a7a23801522d469e93021507fc88e9f5651056fce0e6aa22b3d3157c5f3dc6fe` |

Full local source-toolkit compilation was not run: it would compile the toolkit
rather than consume the installed SDK. Its build-script bypass is covered by the
bounded fixture and source aliases remain explicitly cfg-gated. Portable source
compilation still needs the current remote platform checks. Old target-triple
artifacts are retained while another task may use them.

Independent review round 1 was clean. The plan's consultant assignment was corrected
as requested. Final evidence review is recorded in the task/PR after it completes.
