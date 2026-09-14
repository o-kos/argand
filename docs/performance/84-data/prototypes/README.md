# Diagnostic prototypes only

These patches preserve the experiment that produced the two `present` runs.
They are **not applied** to the application or its dependencies. The project
continues to use the registry dependencies fixed by `Cargo.lock`.

`immediate-app.patch` adds CPU probes, the waveform texture cache, an event-time
scene submission during minimap dragging, and FFT requests on release.
`gpui-present.patch` exposes the existing private submission method in GPUI 0.2.2.
The latter is a diagnostic hook, not a proposed stable public API.

To reproduce the build in a disposable environment:

1. Start from this branch's unchanged application source.
2. Apply `immediate-app.patch` to the application.
3. Copy the published GPUI 0.2.2 source to a temporary directory, and apply
   `gpui-present.patch` there with `patch -p1`.
4. Run the normal checks and release build, adding
   `--config 'paths=["/absolute/path/to/temporary/gpui"]'` **after** the Cargo
   subcommand. For example:

   ```sh
   cargo clippy --config 'paths=["/absolute/path/to/temporary/gpui"]' --all-targets --locked
   cargo test --config 'paths=["/absolute/path/to/temporary/gpui"]' --locked
   cargo build --config 'paths=["/absolute/path/to/temporary/gpui"]' --release --locked
   ```

   The override changes neither the manifest nor the lockfile. Cargo's external
   Clippy subcommand must receive the configuration argument itself.
5. Use the signal, geometry and motion described in the parent measurement report.

The combined run without immediate presentation uses the same application patch
with `window.present()` omitted, against the unmodified registry GPUI.
The `present-live` run keeps immediate presentation but restores ordinary
`ask_for_a_picture()` calls while moving and the original `finish_pan` handler.

## Known limitations

- The FFT-on-release prototype does not dispatch the final range when a missing
  mouse-up is detected by a later non-dragging motion. It is not suitable for use
  as an application build.
- Immediate rendering happens before returning from the platform input callback.
  It can block in swapchain acquisition or GPU synchronization, and calls are not
  coalesced. The measured pointer-handler scope excludes this deferred work.
- Texture painting differs from the old quads at fractional device coordinates.
  Both orientations and high/fractional display scales still require visual
  equivalence checks before retaining the cache in production.
- The unchanged two-frame-callback image retirement must be preserved. No
  immediate atlas destruction is added by these patches.

The owner declined connecting a custom GPUI dependency. These patches remain
reproduction material; they do not authorize a fork, dependency override in the
project, or upstream publication.
