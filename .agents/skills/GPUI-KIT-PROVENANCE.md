# GPUI Kit skill provenance

- Source: https://github.com/longbridge/gpui-kit
- Release tag: `v0.9.1`
- Resolved upstream commit: `39ab849a93b5f15507b969164d49891db8d5b470`
- Skill paths: `skills/gpui-kit` and `skills/gpui-kit-design-guides`
- Installed from the official GitHub API release archive, fetched through `gh`.
- Both skill directories, including their references, are unmodified upstream copies.

## Reading notes

Use these guides with Argand's repository instructions and the owner's agreed decisions.
Confirm APIs and migration claims against the exact locked toolkit sources.
The bundled testing reference still describes non-synced Animation as wall-clock based,
while the 0.9.0 release notes describe the executor-clock change. It also names GPUI
0.3.8 for a Windows headless-rendering limitation; reassess that claim against the
0.3.12 backend selected by GPUI Kit 0.9.1. Do not treat these older statements as
verified current behavior.

## Relative-link verification

All bundled skill-to-reference and GPUI-reference links resolve. The upstream
copies also retain links intended for the website or a complete upstream checkout;
they are not additional missing files from either skill package:

- `coding-guides.md` links to `./design-guides.md`: use the paired
  `gpui-kit-design-guides/references/design-guides.md`, as SKILL.md explains.
- `coding-guides.md` links to `./getting-started.md`: use
  https://github.com/longbridge/gpui-kit/blob/v0.9.1/website/docs/getting-started.md
- `usage.md` links twice to `../../../examples/ai_recipes/src/bootstrap.rs`: use
  https://github.com/longbridge/gpui-kit/blob/v0.9.1/examples/ai_recipes/src/bootstrap.rs
- `design-guides.md` links to `./coding-guides.md`: use the paired
  `gpui-kit/references/coding-guides.md`.
- `design-guides.md` links to `./style`, `./action` and `./keybinding`: these
  identify upstream website documentation, not skill-local reference files.

No upstream skill files were edited to rewrite those links. The installation
contains 27 files in gpui-kit and 2 in gpui-kit-design-guides, plus this provenance.

## Upstream whitespace

The unmodified references retain upstream Markdown hard-break spaces in
`gpui/entity.md` and extra final blank lines in `gpui/action.md`, `gpui/async.md`,
`gpui/event.md`, `gpui/focus-handle.md` and `gpui/global.md`. These produce seven
`git diff --check` diagnostics when first imported. The coordinator accepted this
source-preservation exception; Argand-authored plan and provenance are checked
without that exception. No lint configuration or upstream file was changed.
