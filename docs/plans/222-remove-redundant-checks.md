# Issue #222: Remove low-value and redundant test layers

Refs [#222](https://github.com/o-kos/argand/issues/222), Draft PR #238. The owner requested a broader value audit after the first four consolidations. This phase removes identified low-value checks and repeated layers; it does not close the broader Issue or authorize production module reorganization.

## Roles, scope and evidence limits

- Class A: workspace-wide coverage changes include DSP correctness, UI interactions and file-safety boundaries.
- Coordinator: `gpt-6-astra` medium; executor: `gpt-6.1-sol` medium; fresh independent reviewer: `gpt-6.1-sol` high, separate context, at most three rounds.
- Preserve all production behavior/APIs, numerical inputs/tolerances except the explicitly tighter ENBW survivor, dependencies, features, profiles and toolchain/cache setup. No new widgets or production module changes.
- The suite-wide read-only audit considered APP, core/DSP, CLI, I/O and editing groups. CLI/EDIT and small I/O groups were read in detail; the large writer tail/FloatWave group had inventory/group assessment rather than a complete body audit. No deletion is inferred from those unread bodies. Stable code alone is not obsolete, and a unique literal assertion alone is not necessarily a useful contract.
- Preserve source-race/cancellation/corrupt-FLAC/PCM24-RF64 protections, real/IQ numerical/endpoint coverage, custom-frame/hover/selected guards, asset-list/session-version whitelists, Rect coefficients, inversion algebra and the zero-length capture constructor edge. No runtime speedup is claimed.

## Retain the four initial consolidations

Retain the already prepared full-byte opacity subset removal; shared four-window gain/ENBW textbook setup with every constant and tolerance; six-map endpoint/strict-brightness checks; and CLI JSON/quiet stdout/stderr matrix with one parse per case. Those changes replace seven old test names with three combined names (net four fewer). Their prior1158/0/0 validation does not validate the expanded source.

## Additional standalone removals

These are28 deletions of test functions, not a harness-count consolidation. The table distinguishes intentionally retired synthetic checks from contracts still exercised elsewhere; it does not claim every old fixture byte remains identical.

| Exact test name (package/target recorded in handoff) | Source | Reason, surviving protection and lost check |
| --- | --- | --- |
| `shell::settings_ui::editor::standard_input_tests::standard_inputs_type_choose_and_dismiss_inside_a_root_window` | `settings_editor_standard_input_test.rs` | Toolkit-only Form adds no Argand behavior; retain actual settings-editor hint/form typing, selection, transaction and Escape regressions. Lose standalone stock-control smoke, not product acceptance. |
| `config::tests::a_waveform_share_that_leaves_no_plot_is_refused` | `config_tests.rs` | Obsolete waveform_fraction expectation: renderer no longer consumes this field; retain live configuration/default/fallback tests. Validator behavior is not changed. |
| `shell::settings_ui::tests::control_foregrounds_keep_a_resting_colour_and_a_distinct_state_one` | `settings_ui_test.rs` | Mix-formula mirror; retain rendered real-control state/interaction tests. Lose exact synthetic interpolation values, not an accepted pixel contract. |
| `shell::plot_ui::tests::horizontal_corner_zones_put_time_bottom_left_and_frequency_top_right` | `plot_ui_test.rs` | Expected geometry repeats implementation; retain rendered resizing in both orientations plus hidden/83px threshold guards. Lose duplicate fixed-coordinate equality. |
| `shell::plot_ui::tests::vertical_corner_zones_put_time_top_right_and_frequency_bottom_left` | `plot_ui_test.rs` | Same geometry mirror in the other orientation; retain actual rendered orientation/resize and threshold behavior. |
| `app_menu::tests::a_recent_row_carries_the_capture_it_names` | `app_menu_test.rs` | Constructor payload7 smoke; retain real recent-menu activation, origin identity and routing tests. Lose one synthetic Row layout equality. |
| `document::tests::a_document_is_named_by_its_file_rather_than_by_its_whole_path` | `document_tests.rs` | Basename accessor smoke; retain document/opening summaries, real menu/title workflows. Lose one literal path basename assertion. |
| `text::tests::width_grows_with_text_and_size` | `text_tests.rs` | Generic font monotonicity smoke; retain exact fixture/real-font bridge and actual alignment/rendered glyph tests. Lose generic size20/10 and empty-width comparisons. |
| `text::tests::drawing_puts_ink_on_the_canvas` | `text_tests.rs` | Arbitrary >20ink smoke; alignment/blending and real render pixel/grid assertions already require actual ink. |
| `render::tests::the_spectrogram_is_drawn_whatever_the_panels_say` | `render_tests.rs` | Three Some checks repeat default-layout and actual waterfall/spectrum/shared-grid cases; no distinct visible contract. |
| `render::tests::rendering_fills_the_requested_canvas` | `render_tests.rs` | 900x320/>8colors smoke; retain actual default PNG dimensions, panel/orientation outputs and targeted pixel/grid checks. |
| `render::tests::panel_lists_reject_what_cannot_be_drawn` | `render_tests.rs` | CLI parser and binary invalid-panel cases preserve rejection classes/reasons; lose full copied English strings and whitespace-only spelling. |
| `cli::tests::output_defaults_to_the_input_with_a_png_suffix` | `cli_tests.rs` | Direct PathBuf accessor duplicate; retain actual default suffix publication and explicit-output integration tests. |
| `cli::tests::the_removed_reference_option_is_rejected` | `cli_tests.rs` | Same --ref rejection remains in binary integration test; no unique behavior lost. |
| `inputs::tests::a_mask_keeps_the_directory_the_argument_named` | `inputs_tests.rs` | Batch integrations pass directory-qualified masks and open/publish those files; lose direct vector-path equality. |
| `inputs::tests::a_mask_that_matches_nothing_is_an_error` | `inputs_tests.rs` | Binary unusable-mask coverage preserves public failure; lose internal ResolveError discriminant assertion. |
| `inputs::tests::a_mask_in_a_directory_component_is_refused` | `inputs_tests.rs` | Binary unusable-mask coverage preserves directory-mask rejection/reason; lose duplicate internal variant assertion. |
| `inputs::tests::a_recursive_mask_is_refused_wherever_it_appears` | `inputs_tests.rs` | Binary unusable-mask tests plus mask parser recursive syntax matrix preserve rejection; lose duplicate resolver-layer variants. |
| `convert::tests::stops_at_whichever_buffer_ends_first` | `convert_tests.rs` | Complete incomplete_scalar_tails matrix preserves short output, all scalar tails and sentinels, including I16. |
| `normalize::tests::explicit_factor_is_used_verbatim` | `normalize_tests.rs` | Budget and packed-I24 tests retain Factor7 unchanged with zero scan budget; lose literal8/F16 direct-match smoke. |
| `mmapped::tests::length_counts_iq_pairs_once` | `mmapped_tests.rs` | All-type matrix checks values/channels length for every real/IQ format; duration survives real-capture/document coverage. Lose literal100 sample fixture. |
| `write::tests::whole_capture_is_saved_without_a_span` | `write_tests.rs` | Temporary-name collision test also saves None-span and compares complete output/source bytes; retain source/publication matrix. Lose513-frame/2MHz happy-path fixture only. |
| `history::tests::a_new_edit_drops_what_could_have_been_redone` | `history_tests.rs` | Saved-version test retains undo/apply/!can_redo; version-walk test checks actual capture/state updates. Lose literal9 branch-length check. |
| `stft::tests::no_waveform_is_built_unless_one_is_asked_for` | `stft_tests.rs` | Time-peak tail test already explicitly requests no waveform and asserts None through actual analysis. |
| `window::tests::enbw_in_hertz_scales_with_the_transform_size` | `window_tests.rs` | Retain its stricter1e-3 threshold in the actual-analysis bandwidth test, with the same Hann1024/Fs24000 and independent1.5*Fs/N expectation. No tolerance weakening. |
| `selection::tests::a_time_selection_covers_every_frequency` | `selection_tests.rs` | Selection constructor/default shape smoke; retain actual selection-panel/editing workflows and band/range validity cases. Lose direct default/no-band field checks. |
| `signal::tests::duration_uses_samples_not_scalar_values` | `signal_tests.rs` | Document test covers actual real/IQ pair durations; retain true-rate/physical-frequency tests. Lose literal half-hour direct accessor check. |
| `sample::tests::error_message_lists_valid_tokens` | `sample_tests.rs` | CLI bad-values test uses the same iq_f64 input and checks iq_f16x8 guidance; lose duplicate error-text layer. |

Delete the stock-only209-line Form file and only its cfg(test) module wiring. Update current UI inventory references to the actual settings-editor form tests; completed historical plans remain unchanged. Remove only test helpers/imports made unused by these deletions.

## Preserve distinct matrix cases and strengthen a pixel oracle

For conversion fractional edges, `[rate, rate/2, 8000.1]` repeats the first rate when input rate is exactly8000.1. Omit only that duplicate: two references × two input domains × two output domains = eight identical plan constructions removed (72 to64), with all distinct tuples and existing assertions/tolerances retained.

Replace the upload test's file/settings/DSP fixture setup with an explicit asymmetric3x2 image and independent full expected BGRA bytes. The production function under test only converts the supplied image; the removed setup supplies its input rather than asserting an independent DSP oracle. Rename the test to describe multirow pixel-position/channel preservation. Assert total dimensions/length and every pixel; preserve deep-zoom, invalid-buffer/overflow, padding and vertical-rotation tests. This drops unneeded setup, not an asserted end-to-end production launch.

## Exact discovery and acceptance

Expected1130 Linux tests follows initial net−4 and28 standalone removals from the preserved1162 baseline; it is not a target to force. The image oracle rename adds one removed/added name without changing count. Compare exact package/target/name/ignored records:36 removed names and4 added names, all explicitly listed in the handoff; every other record and the zero-ignored set must match. If ENBW's exact preserved threshold cannot pass, retain the standalone test and explain the changed count rather than relax it.

| Criterion | Evidence / current result |
| --- | --- |
| Only approved low-value/redundant checks removed; meaningful risk coverage retained | Source mapping above; fresh independent review pending |
| Production behavior/API/modules and graph unchanged | Exact diff/cfg wiring assessment pending; no production refactor authorized |
| Every distinct conversion tuple retained, no tolerance weakened | Source tuple comparison and same-input ENBW1e-3 survivor pending |
| Exact discovery delta, zero ignored | Rebuild/discovery comparison pending for expanded source |
| Required local gate and current-snapshot review | Pending; earlier narrow-source results are historical only |

## Implementation and validation

- [ ] Commit this expanded scope before additional code edits; keep Draft PR #238.
- [ ] Apply only the listed removals, duplicate tuple pruning, fixture replacement and current documentation updates.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked` and exact discovery comparison against1162 baseline.
- [ ] `cargo build --release --locked` last after the other checks pass.
- [ ] Update evidence and move this plan to completed before fresh independent review.
- [ ] Resolve findings within three rounds; coordinator assesses actual surviving contracts, not just test count.

Use the existing sole Rust1.98.1 and agreed locked graph, shared intermediate build-dir/local final outputs, direct executable selection and external compilation guard. Ordinary own-crate recompilation within debug/test, release and check/Clippy baseline is approved. Stop on external rebuilds beyond that baseline; do not alter features/profiles/flags to pass a gate. No further Clippy diagnosis, tooling/policy work, dependency changes, ignored tests or benchmark runs are in scope.

The initial plan-only push used the approved scoped non-code hook exception. Code publication requires the complete gate and unchanged hook; current-head Ready Linux/Windows/macOS and ci/full must all succeed before protected merge. Optional external capture presence and native UI acceptance are not inferred from local counts. No new production UI behavior is introduced.

## Post-completion

Leave #222 open for further owner-directed scope and separate production-layout discussion. Preserve three existing stashes, unrelated worktrees and the external Cargo Trash proposal. Accepted PR237 refs remain retained after automatic approval review rejected remote deletion; do not retry or bypass that rejection without explicit authorization.
