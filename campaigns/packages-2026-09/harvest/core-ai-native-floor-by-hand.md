# core-ai-native — the floor, run by hand

_Captured 2026-07-28 against `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/`._

**`core-ai-native` ships library crates only** — `-conform` and `-specmap` build to rlibs and `-specmark` to a proc-macro dll, so the package has no umbrella CLI and no `floor` subcommand exists for it. The floor's three portable steps are therefore run by hand here; the discipline-specific steps (conform, specmap, test-gate) have no runnable form in this package at all.

```console
$ cargo fmt --all --check
EXIT=0
```

```console
$ cargo test --workspace

running 26 tests
test baseline::tests::baseline_diff_news_and_stales ... ok
test rules::go::tests::cell_scoped_kinds_stay_silent_outside_cells_dir ... ok
test finding::tests::scope_filters_findings_not_facts ... ok
test rules::go::tests::deviation_reason_is_honoured_and_reasonless_suppression_is_not ... ok
test rules::go::tests::files_outside_cells_dir_import_cells_freely ... ok
test rules::go::tests::sibling_import_fails_seams_and_own_cell_pass ... ok
test rules::go::tests::value_bans_skip_test_files_but_t_skip_fires_only_there ... ok
test rules::tests::bare_single_crate_paths_are_in_scope ... ok
test rules::tests::error_enum_cites_req_flags_once_per_enum ... ok
test rules::tests::r001_flags_ctor_outside_registry ... ok
test rules::tests::r002_flags_sibling_cell_import ... ok
test rules::tests::cell_has_oracle_satisfied_by_test_reference ... ok
test rules::tests::req_grammar_renderer_and_acceptor_agree ... ok
test rules::tests::every_rule_message_speaks_the_req_grammar ... ok
test rules::tests::seam_has_doctest_gates_pub_root_items_only ... ok
test rules::tests::unsafe_gate_fingerprint_survives_line_shifts ... ok
test rules::tests::unsafe_gate_honors_testimony_but_not_test_context ... ok
test rules::tests::unsafe_gate_respects_audit_crates ... ok
test rules::typescript::tests::reasoned_expect_error_is_honoured_and_unreasoned_is_not ... ok
test rules::typescript::tests::value_bans_skip_test_files_but_ts_ignore_never_does ... ok
test rules::typescript::tests::seam_imports_and_core_imports_pass_internals_fail ... ok
test sarif::tests::sarif_is_byte_stable ... ok
test config::tests::dot_root_names_the_project_directory ... ok
test config::tests::load_or_default_detects_topology ... ok
test config::tests::tree_invariant_catches_each_violation_class ... ok
test store::tests::dot_root_names_the_project_directory ... ok

test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 11 tests
test toolset::tests::tool_output_renders_the_mcp_result_shape ... ok
test server::tests::missing_call_name_is_invalid_params ... ok
test toolset::tests::descriptors_are_sorted_and_last_registration_wins ... ok
test server::tests::tool_level_failure_is_a_result_not_a_protocol_error ... ok
test server::tests::handshake_list_call_ping_round_trip ... ok
test server::tests::unknown_tool_and_method_answer_not_found ... ok
test wire::tests::classifies_requests_and_notifications_by_id ... ok
test server::tests::malformed_and_blank_lines_never_kill_the_loop ... ok
test wire::tests::responses_render_one_line ... ok
test wire::tests::malformed_frames_are_errors_not_panics ... ok
test capture::tests::capture_guard_end_to_end ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s


running 70 tests
test config::tests::defaults_are_project_neutral ... ok
test config::tests::unknown_field_is_rejected ... ok
test config::tests::external_specs_parse ... ok
test explain::tests::suffix_resolution_finds_the_unique_item ... ok
test explain::tests::unit_view_lists_incoming_edges ... ok
test explain::tests::unknown_targets_error_clearly ... ok
test explain::tests::json_subgraph_has_all_four_tables ... ok
test explain::tests::symbol_view_renders_edges_units_and_siblings ... ok
test mdspec::tests::a_heading_anchor_may_be_written_the_way_a_fact_id_may ... ok
test mdspec::tests::a_heading_anchor_and_a_fact_id_differing_only_in_case_are_two_names ... ok
test mdspec::tests::a_lead_paragraph_fact_and_its_list_item_facts_coexist ... ok
test config::tests::absent_file_is_none ... ok
test mdspec::tests::anchored_heading_becomes_a_unit_with_span_hash ... ok
test mdspec::tests::duplicate_anchor_in_one_file_warns_and_keeps_both ... ok
test mdspec::tests::duplicate_fact_ids_warn_like_heading_anchors ... ok
test mdspec::tests::every_list_marker_flavour_and_indent_is_recognised ... ok
test mdspec::tests::fact_anchor_inside_a_fence_is_not_a_unit ... ok
test mdspec::tests::fact_id_colliding_with_a_heading_anchor_warns ... ok
test mdspec::tests::fact_span_covers_indented_continuations ... ok
test mdspec::tests::fenced_sample_headings_are_not_units_and_do_not_cut_spans ... ok
test mdspec::tests::hash_is_line_ending_invariant ... ok
test mdspec::tests::hashhash_with_no_space_is_a_fact_not_a_heading ... ok
test mdspec::tests::heading_line_is_never_a_fact_anchor ... ok
test mdspec::tests::invalid_anchor_warns_and_skips ... ok
test mdspec::tests::invalid_fact_id_after_hashes_is_silently_prose ... ok
test mdspec::tests::kind_line_parses_kind_revision_status ... ok
test mdspec::tests::list_item_fact_anchor_upper_is_addressable ... ok
test mdspec::tests::malformed_kind_line_warns_but_keeps_the_unit ... ok
test mdspec::tests::nested_list_item_fact_anchor_is_its_own_unit ... ok
test mdspec::tests::ordinary_inline_code_is_not_a_kind_line ... ok
test mdspec::tests::paragraph_fact_anchor_becomes_an_untyped_unit ... ok
test mdspec::tests::unanchored_heading_ends_a_span_but_is_not_a_unit ... ok
test ratchet::tests::private_and_pub_crate_items_are_ignored ... ok
test ratchet::tests::disposition_is_carried ... ok
test ratchet::tests::cfg_test_mod_is_skipped ... ok
test ratchet::tests::scope_edge_on_module_covers_items ... ok
test ratchet::tests::tagged_item_is_not_orphan ... ok
test ratchet::tests::untagged_pub_item_is_orphan ... ok
test rscan::tests::bad_grammar_becomes_a_warning_not_an_error ... ok
test rscan::tests::foreign_scope_macros_are_ignored ... ok
test rscan::tests::module_paths_follow_the_scheme ... ok
test rscan::tests::scope_marker_records_a_module_edge ... ok
test rscan::tests::unparseable_source_degrades_to_a_warning ... ok
test rscan::tests::untagged_items_are_not_inventoried ... ok
test testgate::tests::green_run_with_quarantined_skips_is_green ... ok
test testgate::tests::baseline_entry_missing_from_run_is_a_warning_not_a_failure ... ok
test testgate::tests::newly_failing_trips_the_gate ... ok
test testgate::tests::parses_real_nextest_line_shapes ... ok
test testgate::tests::unexpectedly_passing_trips_the_gate ... ok
test tests::content_hash_differs_on_content ... ok
test testgate::tests::unknown_baseline_status_is_rejected ... ok
test tests::content_hash_is_crlf_invariant ... ok
test rscan::tests::tagged_items_yield_edges ... ok
test config::tests::glob_expands_subdirs_and_literals_pass_through ... ok
test tripwire::tests::exact_file_tripwire_fires ... ok
test tripwire::tests::fixed_debts_do_not_fire ... ok
test tripwire::tests::unrelated_changes_fire_nothing ... ok
test tripwire::tests::touch_glob_fires_on_matching_change ... ok
test config::tests::present_file_requires_namespace ... ok
test tripwire::tests::reads_a_debt_file_from_disk_and_evaluates ... ok
test mdspec::tests::root_spec_docs_are_scanned_and_other_root_md_is_not ... ok
test index::tests::drift_classification_reports_bumps_and_unbumped_hashes ... ok
test mdspec::tests::external_specs_resolve_under_their_own_namespace_and_are_skipped_when_absent ... ok
test index::tests::suspects_dangling_and_pin_ahead_are_detected ... ok
test index::tests::node_inventory_is_ordered_and_house_style ... ok
test index::tests::index_is_deterministic ... ok
test index::tests::external_specs_resolve_edges_without_entering_the_index ... ok
test ledger::tests::epoch_is_stable_for_unchanged_inputs ... ok
test ledger::tests::second_identical_prose_call_is_a_cache_hit ... ok
test ledger::tests::editing_cargo_lock_invalidates_the_render ... ok

test result: ok. 70 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
test tags_are_inert ... ok
test verifies_without_pin_compiles_and_runs ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 14 tests
test tests::a_digit_head_is_the_one_thing_the_widening_took_away ... ok
test tests::spec_args_reason_rejected_on_other_verbs ... ok
test tests::spec_args_happy_path ... ok
test tests::uri_parses_revision_pin ... ok
test tests::spec_args_deviates_requires_reason ... ok
test tests::spec_args_pin_conflict_and_agreement ... ok
test tests::spec_args_unknown_verb_and_key ... ok
test tests::the_two_validators_agree_on_every_input ... ok
test tests::uri_accepts_an_upper_fact_anchor ... ok
test tests::uri_args_for_verifies_and_scope ... ok
test tests::uri_rejections ... ok
test tests::cell_args_happy_path_and_rejections ... ok
test tests::uri_parses_with_all_parts ... ok
test tests::spec_args_rejects_zero_revision_and_empty_reason ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 38 tests
test crates\core-ai-native-conform\src\baseline.rs - baseline::load (line 32) - compile ... ok
test crates\core-ai-native-conform\src\baseline.rs - baseline::Baseline (line 13) ... ok
test crates\core-ai-native-conform\src\baseline.rs - baseline::diff (line 54) ... ok
test crates\core-ai-native-conform\src\config.rs - config::Config (line 22) ... ok
test crates\core-ai-native-conform\src\config.rs - config::Config::vacuously_gated (line 335) ... ok
test crates\core-ai-native-conform\src\config.rs - config::TsConfig (line 147) ... ok
test crates\core-ai-native-conform\src\config.rs - config::ExemptEntry (line 202) ... ok
test crates\core-ai-native-conform\src\config.rs - config::GoConfig (line 96) ... ok
test crates\core-ai-native-conform\src\facts.rs - facts::Frontend (line 163) ... ok
test crates\core-ai-native-conform\src\facts.rs - facts::Fact (line 10) ... ok
test crates\core-ai-native-conform\src\finding.rs - finding::Finding (line 9) ... ok
test crates\core-ai-native-conform\src\config.rs - config::ConfigOrigin (line 222) ... ok
test crates\core-ai-native-conform\src\rules\go.rs - rules::go::GoUnsafeInDomain (line 33) ... ok
test crates\core-ai-native-conform\src\facts.rs - facts::SourceFacts (line 139) ... ok
test crates\core-ai-native-conform\src\finding.rs - finding::count_by_rule (line 90) ... ok
test crates\core-ai-native-conform\src\finding.rs - finding::Rule (line 42) ... ok
test crates\core-ai-native-conform\src\rules\budget.rs - rules::budget::AmbientEnv (line 290) ... ok
test crates\core-ai-native-conform\src\rules\budget.rs - rules::budget::FileLength (line 125) ... ok
test crates\core-ai-native-conform\src\rules\budget.rs - rules::budget::NoUnwrapInDomain (line 194) ... ok
test crates\core-ai-native-conform\src\rules\budget.rs - rules::budget::UnsafeGate (line 24) ... ok
test crates\core-ai-native-conform\src\store.rs - store::Store (line 28) - compile ... ok
test crates\core-ai-native-conform\src\rules\diagnostics.rs - rules::diagnostics::ErrorEnumCitesReq (line 290) ... ok
test crates\core-ai-native-conform\src\rules\diagnostics.rs - rules::diagnostics::ErrorMessageCitesReq (line 211) ... ok
test crates\core-ai-native-conform\src\rules\diagnostics.rs - rules::diagnostics::PubDoctest (line 122) ... ok
test crates\core-ai-native-conform\src\rules\diagnostics.rs - rules::diagnostics::SeamHasDoctest (line 23) ... ok
test crates\core-ai-native-conform\src\rules\go.rs - rules::go::GoCellIsolation (line 176) ... ok
test crates\core-ai-native-conform\src\finding.rs - finding::check (line 63) ... ok
test crates\core-ai-native-conform\src\rules\mod.rs - rules::matches_req_grammar (line 56) ... ok
test crates\core-ai-native-conform\src\rules\mod.rs - rules::req_message (line 37) ... ok
test crates\core-ai-native-conform\src\rules\structure.rs - rules::structure::CellHasOracle (line 156) ... ok
test crates\core-ai-native-conform\src\rules\structure.rs - rules::structure::CellIsolation (line 80) ... ok
test crates\core-ai-native-conform\src\rules\structure.rs - rules::structure::FlagSites (line 15) ... ok
test crates\core-ai-native-conform\src\rules\typescript.rs - rules::typescript::TsCellIsolation (line 111) ... ok
test crates\core-ai-native-conform\src\rules\typescript.rs - rules::typescript::TsUnsafeInDomain (line 26) ... ok
test crates\core-ai-native-conform\src\sarif.rs - sarif::render (line 10) ... ok
test crates\core-ai-native-conform\src\store.rs - store::ExtractionLog (line 13) ... ok
test crates\core-ai-native-conform\src\store.rs - store::content_hash (line 185) ... ok
test crates\core-ai-native-conform\src\store.rs - store::sort_source_facts (line 475) ... ok

test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

all doctests ran in 0.67s; merged doctests compilation took 0.49s

running 10 tests
test crates\core-ai-native-mcp\src\server.rs - server::StdioTransport (line 29) - compile ... ok
test crates\core-ai-native-mcp\src\server.rs - server::Server (line 69) ... ok
test crates\core-ai-native-mcp\src\wire.rs - wire::JsonRpcError (line 123) ... ok
test crates\core-ai-native-mcp\src\capture.rs - capture::capture (line 46) ... ok
test crates\core-ai-native-mcp\src\toolset.rs - toolset::ToolDescriptor (line 15) ... ok
test crates\core-ai-native-mcp\src\toolset.rs - toolset::ToolOutput (line 36) ... ok
test crates\core-ai-native-mcp\src\wire.rs - wire::JsonRpcMessage (line 19) ... ok
test crates\core-ai-native-mcp\src\error.rs - error::McpCoreError (line 14) ... ok
test crates\core-ai-native-mcp\src\toolset.rs - toolset::ToolSet (line 84) ... ok
test crates\core-ai-native-mcp\src\wire.rs - wire::JsonRpcResponse (line 78) ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

all doctests ran in 0.47s; merged doctests compilation took 0.36s

running 6 tests
test crates\core-ai-native-specmap\src\config.rs - config::Config (line 27) ... ok
test crates\core-ai-native-specmap\src\config.rs - config::Disposition (line 110) ... ok
test crates\core-ai-native-specmap\src\config.rs - config::ExternalSpec (line 91) ... ok
test crates\core-ai-native-specmap\src\index.rs - index::vacuity_warning (line 400) ... ok
test crates\core-ai-native-specmap\src\lib.rs - content_hash (line 48) ... ok
test crates\core-ai-native-specmap\src\lib.rs - fwd (line 70) ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

all doctests ran in 0.51s; merged doctests compilation took 0.41s

running 5 tests
test crates\core-ai-native-specmark\src\lib.rs - (line 10) ... ignored
test crates\core-ai-native-specmark\src\lib.rs - cell (line 126) ... ok
test crates\core-ai-native-specmark\src\lib.rs - scope (line 107) ... ok
test crates\core-ai-native-specmark\src\lib.rs - spec (line 69) ... ok
test crates\core-ai-native-specmark\src\lib.rs - verifies (line 88) ... ok

test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s

all doctests ran in 0.32s; merged doctests compilation took 0.24s

running 9 tests
test crates\core-ai-native-specmark-grammar\src\lib.rs - Verb (line 33) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - CellArgs (line 456) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - is_valid_anchor (line 118) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - SpecUri (line 73) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - EdgeSpec (line 231) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - UriArgs (line 382) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - SpecArgs (line 278) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - is_valid_fact_id (line 139) ... ok
test crates\core-ai-native-specmark-grammar\src\lib.rs - parse_spec_uri (line 166) ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

all doctests ran in 0.43s; merged doctests compilation took 0.33s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running unittests src\lib.rs (target\debug\deps\core_ai_native_conform-54dc11bdc7e27bed.exe)
     Running unittests src\lib.rs (target\debug\deps\core_ai_native_mcp-8c84865a7d958bbb.exe)
     Running unittests src\lib.rs (target\debug\deps\core_ai_native_specmap-5800b9bcb25d8ad7.exe)
     Running unittests src\lib.rs (target\debug\deps\core_ai_native_specmark-b598517096f228f0.exe)
     Running tests\usage.rs (target\debug\deps\usage-89fbf2f99385dc4a.exe)
     Running unittests src\lib.rs (target\debug\deps\core_ai_native_specmark_grammar-0181ae61dab11165.exe)
   Doc-tests core_ai_native_conform
   Doc-tests core_ai_native_mcp
   Doc-tests core_ai_native_specmap
   Doc-tests core_ai_native_specmark
   Doc-tests core_ai_native_specmark_grammar
EXIT=0
```

```console
$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
EXIT=0
```

**Scope:** every fact under `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/` that this run bears on. The anchor list is not maintained here — a verdict cites this file in its `ev[]`, and the reverse index is derived from the verdict maps at the phase close (PHASE-C-BATCH-PLAN.md §5).
