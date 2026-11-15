# Supplemental upstream notices

Taffy 0.15.0 declares MIT but its published crate omits the LICENSE file. The
unmodified license here comes from the exact Git revision recorded in that crate's
`.cargo_vcs_info.json`. `manifest.json` records the immutable source URL, revision,
locked crate checksum and license SHA-256. Packaging verifies those identities and
copies the notice and provenance only for this exact locked package.

No fallback license or copyright holder is inferred for other crates.
