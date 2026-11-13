# Local GPUI Kit SDK

Linux x86-64 development can use a separately installed GPUI Kit SDK. Application
builds never compile or repair the SDK automatically. The SDK and its consumers
must use the same installed Rust compiler and approved toolkit graph. Other
platforms and portable releases use the source mode below. Wireroom adoption is
separate; do not select another toolkit version to integrate it.

## Build and install explicitly

First follow the [local resource policy](../../CONTRIBUTING.md#local-rust-and-build-resources):
verify the approved installed compiler and workspace-local Cargo target directory, and obtain any
required dependency compilation approval. The commands never install Rust.

```sh
python3 tools/gpui-sdk/sdk.py build-install
python3 tools/gpui-sdk/sdk.py select ~/.local/lib/gpui-kit-sdk/<build-id>
cargo build --locked
cargo test --locked
cargo clippy --all-targets --locked
cargo run --locked -p argand
```

`build-install` is the only operation that builds the SDK. The standalone
`tools/gpui-sdk` workspace takes its exact graph from its committed lock file;
it is not an application workspace member or dependency. Its intermediate artifacts
and final outputs stay in its own `tools/gpui-sdk/target/`.
Its explicit root-only dynamic standard-library selection does not change the
compiler flags of the toolkit dependencies.

Installation collects Cargo-reported Rust libraries and proc macros, native
search-path libraries, the SDK dylib and matching standard-library runtime into
one versioned directory. Its identity includes compiler, lock and artifact hashes.
Installation publishes by renaming a complete staging directory; an existing
build ID is verified, never replaced. After a compiler/toolkit update, explicitly
build and select a new SDK; keep any version still used by another application.

`select` verifies that installation and creates a gitignored `.cargo/config.toml`.
It refuses to overwrite a configuration it did not generate. This configuration
sets `GPUI_KIT_SDK` and a fixed `-Cmetadata` consumer namespace; it does not set a
build target or global SDK extern/search/linker flags. The metadata value appends to
Cargo's per-crate disambiguators, keeping independently built consumer crates,
including host procedural macros, distinct from SDK crates with the same version.
Without it, native consumers can encounter colliding Rust crate identities.
The application's existing build script validates the installation and emits
package-scoped library search and runtime-path directives. The application imports
`gpui_kit_sdk` under its toolkit aliases and loads the SDK's exact `image` crate.
Build scripts, procedural macros and the independent CLI do not receive these SDK
search/link directives. No compiler wrapper, Git hook, modified PATH or automatic
SDK dependency participates.

Ordinary native builds put the application at `target/debug/argand` and the CLI at
`target/debug/aspec`; release binaries are in `target/release/`. Intermediate artifacts
also stay in the workspace-local `target/`; projects and worktrees do not share a
Cargo build directory. Run `select` again to migrate an older generated selection
that set an explicit target; it replaces that generated configuration without
deleting the old target-triple artifacts. The installed SDK remains shared and is independent of those
local build artifacts. SDK mode includes toolkit test support. Ordinary `cargo build --release --locked`
keeps the app linked to the SDK while building the CLI independently from source.
A missing/incompatible SDK produces an explicit error; run the installation or
selection operation yourself rather than expecting Cargo to rebuild it.

## Portable source mode

CI has no machine configuration or SDK. Source dependencies retain their own
Cargo names and are exposed to the application through explicit crate aliases.
For source builds, select `source-toolkit` (and `toolkit-test-support` for tests
and all-target Clippy). That mode bypasses the SDK build-script configuration and
uses only the source aliases, even if an installation is still selected.
To also remove the machine-local SDK selection:

```sh
python3 tools/gpui-sdk/sdk.py source-mode
cargo build --locked --features source-toolkit
cargo test --locked --features source-toolkit,toolkit-test-support
cargo clippy --all-targets --locked --features source-toolkit,toolkit-test-support
cargo build --release --locked --features source-toolkit
```

The test-support feature is explicit so normal source releases do not inherit
GPUI test facilities. Release/CI commands select source mode; they do not install
or package the local development SDK. Switching compiler flags/features can create
new artifact variants and remains subject to the resource policy.

The SDK is a development artifact, not a portable distribution or stable ABI.
Installed compilation metadata is as important as the runtime `.so`: equal crate
versions alone do not give Rust types the same identity. In particular, the app must use the SDK's exact `image` artifact because it
passes image types into GPUI. The CLI shares no GPUI types: its own aliased image
dependency is always built from source, including the bitcode needed by release
Thin LTO. The CLI does not require an SDK installation. Do not substitute
arbitrary similarly named libraries from another Cargo build. CLI modules qualify
`crate::image` explicitly so nested imports cannot load the injected SDK image
while LLVM collects release bitcode.
