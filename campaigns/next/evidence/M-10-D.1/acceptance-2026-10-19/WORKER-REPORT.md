# M-10-D.1 — derivative source and boot inspection

This continuation implements the remaining supported compiler join and preserves the useful retained source/catalogue/frontend work. This is an implementation report with author claims, not acceptance or a verified host result. Original D1 criteria and D2 boundary are unchanged. Source basis for the complete retained patch is current HEAD c9363af48959ecd12f7c7bef44c3fd4e061f9cd1 plus the preserved working tree; no Git mutation was made.

## Implemented producer and read path

The ordinary explicitly configured local scenario producer now passes its checked catalogue through `InstallResolver::capture_boot_scenario_with_derivatives` and `capture_boot_world_with_derivatives`. The incumbent selected registry transaction, before/after CandidateMetadata checks and `DerivativeCatalog::capture_selected`/`capture_retained` supply a complete selected local inventory. FrozenWorld checks the exact package pin/kind, descriptor selection, bytes and membership. World identity includes the checked records, so equal local bytes with a different retained base change the world identity. There is no new dependency edge, resolver, compiler or automatic discovery.

Only checked Vibe-spec descriptor inventories in a normal package enter the supported captured document path. Missing descriptor evidence, bridge, native/owner-runtime, conditional/hybrid preparation and remote/fetch gaps remain explicit. The captured source constructor uses the existing canonical document path, inspect_captured and shared Vibe selector; ordinary data never enter the Vibe parser.

The incumbent compiler worklist observes each successful actual returned source immediately before constructing SourceIr with that same address. CapturedSectionSource retains the strict result, original selected path and exact Markdown text supplied to SourceIr. A private per-invocation trace sink retains the final accepted closure plus the incumbent assemble carrier. Each compile_world still makes one call to compile_artifact_traced; scenario evaluation retains its existing separate baseline/candidate invocations. ProducedBoot is protected by private fields and is constructed only by this producer. Inspector callers cannot construct a provenance attachment to an old artifact.

The shared route retains the successful producer result as an immutable snapshot. Its graph is the strict compiler closure from that same invocation; source joins use the compiler's exact successful-read address, not a later catalogue/path/hash inference. The generated optional `Source.derivative_read` carrier carries checked derivative-result JSON, path, raw/projection resource IDs, a closed projection recipe, Observed strength and Unverified current base. Raw and selected projection resources have separate hashes and explicit original_bytes/captured_projection bases. The full retained record includes independent base/local/result/provenance/inventory identities and raw inventories.

Actual compiler contribution/occurrence pairs remain distinct and share the same source/result. The positive fixture now uses two genuine normal boot contributions, each declaring #use of the same derivative fact; it compiles through the ordinary local producer. The former fabricated STATIC marker fixture was removed. Incumbent #use topology deduplicates within one contribution, and inline #embed expansion does not create independent emitted occurrence records. Those semantics were not changed or replaced with invented occurrences. The final closure retains available edges; absent parent-edge/span/inline occurrence mappings stay gaps.

Inspect/tree/explain/cost/compare/chunk and source navigation consume retained data. They do not compile, call a producer/provider, reopen TraceRun writers, derive replacement content or execute payloads. Existing historical artifact capture still cannot acquire compiler provenance merely by attaching today's catalogue. Changing configuration does not mutate an older produced snapshot or its page session.

## Real production exposure

- CLI boot: `commands::boot::run` -> `run_with_io` -> `open_route` -> ordinary `InstallResolver::open_boot_debugger`. Explicit --derivative-record configures the catalogue. Existing viewer scenario controls invoke the configured producer; the returned produced_snapshot_id selects the retained snapshot, and source navigation reads its joined raw resource. The supplemental bin family drives this standard constructor with a real registry fixture, not a prebuilt test route.
- MCP boot: `ServerContext::with_source_derivatives` -> ordinary lazy `boot_debugger_route` -> the same standard constructor and configured LocalProducer. Registered `default_tools`/boot_debugger dispatch prepares the scenario and then inspects/chunks the retained result. The decisive ordinary-host test does not inject a prebuilt route.
- Source API: `SourceQueryService::prepare_host`/execute_json_with_catalog validates the current prepared local owner/descriptor/full inventory before attaching the same checked record. Returned strength stays Observed; the catalogue does not authenticate today's base.
- CLI source: `commands::spec::run` delegates its standard implementation to run_with_io, with real stdout/stderr in production. The supplemental bin case parses the actual spec read command and drives this implementation with an explicit record file. Its decoded JSON equals the library service and registered MCP read_spec on the same fixture.
- MCP source: registered read_spec uses ServerContext's explicitly supplied catalogue and the same source service. No transport-supplied root or provider grant was added.

The explicit scenario preparation path is the supported compiled exposure. Opening an already existing static boot artifact remains passive retained artifact/history inspection; it cannot reconstruct missing producer evidence. The separate normal static installation compiler path was not broadened.

## Source/debugger proof matrix

| Family | Exact source evidence | Debugger/frontend evidence | Semantic/compiler strength |
|---|---|---|---|
| commonmark | Strict base/local/result record; full raw guide inventory, prose/fences retained | API retained inspection and CLI/MCP configured catalogue agree on complete record/bytes | Ordinary data; no Vibe facts or invented compiler join |
| xml-data | Exact raw XML inventory, including byte spelling | Same immutable checked record across read views | Inert data; optional inspection does not confer Vibe semantics |
| vibe-spec | Exact raw local XML plus independent Vibe projection; current query agrees on result identity and selected text | Real builtin source read, final closure, two distinct contribution/occurrence pairs, raw and selected projection chunks; ordinary CLI and lazy registered MCP routes | Observed same-invocation provenance; current-base authentication remains unverified |
| opaque-file | Arbitrary binary bytes retained without UTF-8 coercion | Full strict record/raw inventory remains inspectable after source removal | Raw only; semantic/native join explicitly unavailable |
| nested-tree | Exact relative-path inventory, additions/removals and inert script/binary bytes | Complete retained catalogue/inventory across source/boot frontends | Raw tree data; downstream activation/receipt work remains D2/later horizon |

The workspace and frontend families iterate all five rows. Unsupported semantic joins are explicit, not successful empty documents. Current-base proof, install-owned source proof, native/cross-platform execution, automatic discovery and D2 activation/recovery are not claimed.

## Identity, freshness and read-only evidence

The source test retains an old bound record, changes the base with equal local bytes, verifies a different result/current capture, refuses the old source cursor, and still exposes the old observed inventory. The actual compiler test checks the two genuine occurrence pairs and shared exact read/result, rejects a changed selected local capture, and refuses changed expected worlds. The MCP update case produces a new compiler capture after equal-byte rebind, refuses mixing its ID with the old page session, and continues the old session and old snapshot unchanged.

The ordinary MCP producer test compares selected projection bytes with the source API and raw chunks with retained authored bytes, checks the filesystem inventory over the read interval, removes registry/authored sources, and reads the identical retained inspection. A separate real-compiler capture is given a rejecting scenario producer after preparation: inspect/tree/cost and retained reads leave its counter at zero. Four inert rows also use the rejecting producer seam. These are finite fixture observations; they do not claim universal network instrumentation or hostile-ABA/cross-platform proof.

## Governing specifications and retained work

PROP-070, PROP-072 and PROP-074 gain the actual captured producer, exact source/read continuity, strict join/resource basis and passive navigation requirements. Independent capture, source-proof, graph, measurement, current-base, D2/later-horizon and no-execution laws remain. Only the earlier partial read-seam sentence disclaiming the now implemented bounded production/matrix path was replaced with the concrete supported Vibe path and explicit inert/unsupported limits; it was not used to weaken the original Work.

The earlier catalogue, query cursor binding, MCP ordinary configuration correction, CLI options, generated carrier additions and five-family passive inspection were preserved/adapted. No predecessor success flag, artifact digest or collected candidate was treated as acceptance. No files were reset, staged or committed. No subagents, models, installs, downloads, provider/native execution or unrelated source edits were used.

## Actual focused checks

All final bound families ran under the supplied exclusive target/private TMP with the exact /home/olegchir/.cargo/bin/cargo, --offline --locked -j2, CARGO_NET_OFFLINE=true and RUSTUP_AUTO_INSTALL=0. No whole-package or workspace suite ran. The final runs contain **16 passed, 0 failed** selected tests.

### vibe-spec --lib resolver::m_10_d_1_cases

```text
running 2 tests
test resolver::m_10_d_1_cases::competing_representations_and_wrong_base_selection_refuse ... ok
test resolver::m_10_d_1_cases::repeated_reads_share_payload_while_local_and_selected_upstream_stay_distinct ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1011 filtered out; finished in 0.00s
```

### vibe-workspace --lib boot_debugger::m_10_d_1_cases

```text
running 4 tests
test boot_debugger::m_10_d_1_cases::altered_record_and_competing_local_bindings_refuse ... ok
test boot_debugger::m_10_d_1_cases::all_five_families_retain_exact_passive_source_navigation ... ok
test boot_debugger::m_10_d_1_cases::actual_builtin_read_and_closure_keep_exact_derivative_and_repeated_occurrences ... ok
test boot_debugger::m_10_d_1_cases::selected_query_binds_retained_result_and_refuses_stale_bytes_and_cursor ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 765 filtered out; finished in 0.09s
```

### vibe-mcp --lib tools::boot_debugger::m_10_d_1_cases

```text
running 6 tests
test tools::boot_debugger::m_10_d_1_cases::real_compiler_capture_is_passive_with_a_rejecting_provider_after_preparation ... ok
test tools::boot_debugger::m_10_d_1_cases::registered_source_and_debugger_routes_share_exact_retained_record_without_proof_promotion ... ok
test tools::boot_debugger::m_10_d_1_cases::ordinary_lazy_producer_compiles_derivative_and_retains_exact_source_navigation ... ok
test tools::boot_debugger::m_10_d_1_cases::derivative_update_keeps_old_capture_and_refuses_mixed_snapshot_paging ... ok
test tools::boot_debugger::m_10_d_1_cases::ordinary_lazy_host_carries_explicit_catalogue_through_registered_tools ... ok
test tools::boot_debugger::m_10_d_1_cases::registered_dispatcher_retains_all_families_with_exact_records_and_explicit_join_gaps ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 223 filtered out; finished in 0.21s
```

### vibe-cli --bin vibe commands::boot::m_10_d_1_cases

```text
running 4 tests
test commands::boot::m_10_d_1_cases::host_record_configuration_refuses_escape_and_corruption ... ok
test commands::boot::m_10_d_1_cases::standard_spec_command_and_registered_mcp_agree_with_source_api ... ok
test commands::boot::m_10_d_1_cases::standard_command_prepares_real_derivative_then_reads_captured_occurrences ... ok
test commands::boot::m_10_d_1_cases::standard_command_constructor_exposes_exact_retained_history_for_all_families ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 974 filtered out; finished in 0.26s
```

Original slots were preserved: 0/1 select/run the two resolver cases; 2/3 select/run the four workspace cases; 4/5 select/run the six MCP cases; slot 6 is literal globalfmt and slot 7 literal git diff --check. The four CLI bin cases are supplemental exposure evidence, not a fabricated original slot. Test declarations are distinguished from these observed nonzero runs.

- `cargo fmt --all -- --check`: **Failed, exit 1**. Final output names seven files outside the 43 changed/new paths. Scoped formatting does not replace this failure.
- `git diff --check`: **Passed, exit 0**, including a repeat on final source.
- Scoped rustfmt check: **Passed, exit 0**, 38 changed/new Rust files with skip_children=true; no unrelated files rewritten.
- Governing XML parse: **Passed**, all three modified documents.
- Actual cached incumbent `xtask check-codegen`: **Failed, exit 1**. Final schema regeneration completed; its Git comparison reports exactly the two deliberately modified generated owners (boot_debugger/e1/result and source_query/e1/result). This is not reported as a clean codegen check or replaced with a raw-generator pass. The cached executable was invoked with CARGO_MANIFEST_DIR pointing to xtask. No toolchain/generator install or version change occurred.

Globalfmt baseline paths:

- /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/frame_cross_surface.rs
- /fast/git/v/vibevm-next/crates/vibe-cli/tests/cli_mcp_fixed_deploy/local_route.rs
- /fast/git/v/vibevm-next/crates/vibe-mcp/src/machine_session/incremental.rs
- /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/exact_local_install_decision/tests.rs
- /fast/git/v/vibevm-next/crates/vibe-orchestrator/src/machine_submission/lifecycle_adapter/live_owner/control.rs
- /fast/git/v/vibevm-next/crates/vibe-publish/src/package_git_payload/tests.rs
- /fast/git/v/vibevm-next/crates/vibe-wire/src/generated/format_id/properties.rs

Codegen comparison paths:

```text
diff --git a/crates/vibe-wire/src/generated/boot_debugger/e1/result/mod.rs b/crates/vibe-wire/src/generated/boot_debugger/e1/result/mod.rs
diff --git a/crates/vibe-wire/src/generated/source_query/e1/result/mod.rs b/crates/vibe-wire/src/generated/source_query/e1/result/mod.rs
```

## Retained failed/intermediate attempts

Failures below are retained as actual outcomes; later passes do not relabel them. Full command logs remain at the named local /tmp paths. Deciding diagnostics and failed result counts are preserved here.

### /tmp/m10d1-check-codegen.log

The first direct cached-xtask invocation failed because CARGO_MANIFEST_DIR was absent. Corrected invocation supplies the owning xtask directory.

```text
Error: CARGO_MANIFEST_DIR not set; is xtask running under cargo?

Caused by:
    environment variable not found
```

### /tmp/m10d1-mcp-1.log

Compilation failed with three missing CapturedProjection enum variants: this run overlapped the carrier-generation transition. Final generation was allowed to settle before later checks.

```text
error[E0599]: no variant, associated function, or constant named `CapturedProjection` found for enum `ResourceBasis` in the current scope
   --> crates/vibe-orchestrator/src/machine_operation/boot_debugger/dispatch.rs:271:80
    |
271 | ... r::ResourceBasis::CapturedProjection { r::BodyChunkBasis::CapturedProjection } else { r::BodyChunkBasis::OriginalBytes },
    |                       ^^^^^^^^^^^^^^^^^^ variant, associated function, or constant not found in `ResourceBasis`

error[E0599]: no variant, associated function, or constant named `CapturedProjection` found for enum `ResourceBasis` in the current scope
  --> crates/vibe-orchestrator/src/machine_operation/boot_debugger/produced.rs:39:172
   |
39 | ..., r::ResourceBasis::CapturedProjection);
error[E0599]: no variant, associated function, or constant named `CapturedProjection` found for enum `ResourceBasis` in the current scope
  --> crates/vibe-orchestrator/src/machine_operation/boot_debugger/produced.rs:39:172
   |
39 | ..., r::ResourceBasis::CapturedProjection);
   |                        ^^^^^^^^^^^^^^^^^^ variant, associated function, or constant not found in `ResourceBasis`

error[E0599]: no variant, associated function, or constant named `CapturedProjection` found for enum `BodyChunkBasis` in the current scope
   --> crates/vibe-orchestrator/src/machine_operation/boot_debugger/dispatch.rs:271:120
    |
271 | ...r::BodyChunkBasis::CapturedProjection } else { r::BodyChunkBasis::OriginalBytes },
error[E0599]: no variant, associated function, or constant named `CapturedProjection` found for enum `BodyChunkBasis` in the current scope
   --> crates/vibe-orchestrator/src/machine_operation/boot_debugger/dispatch.rs:271:120
    |
271 | ...r::BodyChunkBasis::CapturedProjection } else { r::BodyChunkBasis::OriginalBytes },
    |                       ^^^^^^^^^^^^^^^^^^ variant, associated function, or constant not found in `BodyChunkBasis`

warning: unused import: `RunObserver`
  --> crates/vibe-orchestrator/src/phase.rs:38:26
   |
38 |     RegistryEnvironment, RunObserver,

For more information about this error, try `rustc --explain E0599`.
warning: `vibe-orchestrator` (lib) generated 15 warnings
error: could not compile `vibe-orchestrator` (lib) due to 3 previous errors; 15 warnings emitted
```

### /tmp/m10d1-mcp-2.log

Compilation failed because package-source has no direct workspace dependency. Fixed by the existing orchestrator composition reexport and a catalogue-owned capture helper; no new dependency was added.

```text
error[E0433]: cannot find module or crate `vibe_workspace` in this scope
  --> crates/vibe-package-source/src/source/preview.rs:38:23
   |
38 |         derivatives: &vibe_workspace::source_query::derivatives::DerivativeCatalog,
   |                       ^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `vibe_workspace`
   |
   = help: if you wanted to use a crate named `vibe_workspace`, use `cargo add vibe_workspace` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `vibe_workspace` in this scope
   --> crates/vibe-package-source/src/source/preview.rs:168:25
error[E0433]: cannot find module or crate `vibe_workspace` in this scope
   --> crates/vibe-package-source/src/source/preview.rs:168:25
    |
168 |                         vibe_workspace::derivatives::capture_retained(&project, local).map_err(|e| e.to_string())?;
    |                         ^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `vibe_workspace`
    |
    = help: if you wanted to use a crate named `vibe_workspace`, use `cargo add vibe_workspace` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `vibe_workspace` in this scope
  --> crates/vibe-package-source/src/source/scenario.rs:32:23
error[E0433]: cannot find module or crate `vibe_workspace` in this scope
  --> crates/vibe-package-source/src/source/scenario.rs:32:23
   |
32 |         derivatives: &vibe_workspace::source_query::derivatives::DerivativeCatalog,
   |                       ^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `vibe_workspace`
   |
   = help: if you wanted to use a crate named `vibe_workspace`, use `cargo add vibe_workspace` to add it to your `Cargo.toml`

error[E0433]: cannot find module or crate `vibe_workspace` in this scope
   --> crates/vibe-package-source/src/source/scenario.rs:123:23
error[E0433]: cannot find module or crate `vibe_workspace` in this scope
   --> crates/vibe-package-source/src/source/scenario.rs:123:23
    |
123 |         derivatives: &vibe_workspace::source_query::derivatives::DerivativeCatalog,
    |                       ^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `vibe_workspace`
    |
    = help: if you wanted to use a crate named `vibe_workspace`, use `cargo add vibe_workspace` to add it to your `Cargo.toml`

For more information about this error, try `rustc --explain E0433`.
error: could not compile `vibe-package-source` (lib) due to 4 previous errors
For more information about this error, try `rustc --explain E0433`.
error: could not compile `vibe-package-source` (lib) due to 4 previous errors
warning: build failed, waiting for other jobs to finish...
warning: `vibe-orchestrator` (lib) generated 15 warnings (run `cargo fix --lib -p vibe-orchestrator` to apply 14 suggestions)
```

### /tmp/m10d1-mcp-3.log

Executed 5 cases: 2 passed, 3 failed. The first fixture used inline #embed without separately retained occurrence records; its graph did not contain the required positive joins. A parity assertion also incorrectly compared two separate capture operations. The positive now uses two real compiler boot contributions with #use, and parity comparisons use the same immutable capture rather than pretending fresh capture IDs are equal.

```text
failures:

---- tools::boot_debugger::m_10_d_1_cases::derivative_update_keeps_old_capture_and_refuses_mixed_snapshot_paging stdout ----

thread 'tools::boot_debugger::m_10_d_1_cases::derivative_update_keeps_old_capture_and_refuses_mixed_snapshot_paging' (2651340) panicked at crates/vibe-mcp/src/tools/boot_debugger/m_10_d_1_cases.rs:314:50:
called `Option::unwrap()` on a `None` value
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- tools::boot_debugger::m_10_d_1_cases::ordinary_lazy_producer_compiles_derivative_and_retains_exact_source_navigation stdout ----

thread 'tools::boot_debugger::m_10_d_1_cases::ordinary_lazy_producer_compiles_derivative_and_retains_exact_source_navigation' (2651342) panicked at crates/vibe-mcp/src/tools/boot_debugger/m_10_d_1_cases.rs:399:9:
assertion `left == right` failed
  left: Object {"body": Object {"stage": String("result"), "completeness": String("complete"), "effect_state": String("no_effects"), "format_id": String("boot-debugger-result"), "payload_json": String("{\"body\":{\"kind\":\"capture\",\"capture_id\":\"document-capture/1:17:1:2\",\"resources\":[{\"basis\":\"original_bytes\",\"buffer_version\":null,\"bytes\":{\"exact\":\"169\",\"unavailable\":null},\"digest\":\"sha256:9860eb0b6926c25264b74946cc0f37b198903d2926b69c0e7d105320949a3668\",\"path\":\"vibe.toml\",\"resource_id\":\"document-capture/1:17:1:2:resource-0\"},{\"basis\":\"original_bytes\",\"buffer_version\":null,\"bytes\":{\"exact\":\"107\",\"unavailable\":null},\"digest\":\"sha256:cc6502352151457c3aadecf12b9fd2a301a1714985426a862954b4e8945d7c50\",\"path\":\"STATIC.md\",\"resource_id\":\"document-capture/1:17:1:2:resource-1\"}]},\"context_id\":\"boot-context:17\",\"result_id\":\"sha256:1c75c01ab04edef3d01e2b666a795df6091d5f11aca6e13d6541eb79a6b94d0f\",\"schema\":1,\"snapshot_id\":\"document-capture/1:17:1:2\"}"), "provenance_id": String("document-capture/1:17:1:2")}, "domain_owner": String("rust:vibe-workspace:boot-debugger"), "evidence_id": Null, "operation_id": String("rust:vibe-workspace:boot-debugger"), "schema": Number(1), "source_id": String("captured-boot-debugger/1")}
 right: Object {"body": Object {"stage": String("result"), "completeness": String("complete"), "effect_state": String("no_effects"), "format_id": String("boot-debugger-result"), "payload_json": String("{\"body\":{\"kind\":\"capture\",\"capture_id\":\"document-capture/1:17:1:3\",\"resources\":[{\"basis\":\"original_bytes\",\"buffer_version\":null,\"bytes\":{\"exact\":\"169\",\"unavailable\":null},\"digest\":\"sha256:9860eb0b6926c25264b74946cc0f37b198903d2926b69c0e7d105320949a3668\",\"path\":\"vibe.toml\",\"resource_id\":\"document-capture/1:17:1:3:resource-0\"},{\"basis\":\"original_bytes\",\"buffer_version\":null,\"bytes\":{\"exact\":\"107\",\"unavailable\":null},\"digest\":\"sha256:cc6502352151457c3aadecf12b9fd2a301a1714985426a862954b4e8945d7c50\",\"path\":\"STATIC.md\",\"resource_id\":\"document-capture/1:17:1:3:resource-1\"}]},\"context_id\":\"boot-context:17\",\"result_id\":\"sha256:6f9f37bcd8e0743222ea949f0646b3ea5e2eed3b0c47599a63c8258aa1d4e656\",\"schema\":1,\"snapshot_id\":\"document-capture/1:17:1:3\"}"), "provenance_id": String("document-capture/1:17:1:3")}, "domain_owner": String("rust:vibe-workspace:boot-debugger"), "evidence_id": Null, "operation_id": String("rust:vibe-workspace:boot-debugger"), "schema": Number(1), "source_id": String("captured-boot-debugger/1")}

---- tools::boot_debugger::m_10_d_1_cases::registered_dispatcher_retains_all_families_with_exact_records_and_explicit_join_gaps stdout ----

thread 'tools::boot_debugger::m_10_d_1_cases::registered_dispatcher_retains_all_families_with_exact_records_and_explicit_join_gaps' (2651343) panicked at crates/vibe-mcp/src/tools/boot_debugger/m_10_d_1_cases.rs:137:74:
called `Option::unwrap()` on a `None` value


failures:
    tools::boot_debugger::m_10_d_1_cases::derivative_update_keeps_old_capture_and_refuses_mixed_snapshot_paging
    tools::boot_debugger::m_10_d_1_cases::ordinary_lazy_producer_compiles_derivative_and_retains_exact_source_navigation
    tools::boot_debugger::m_10_d_1_cases::registered_dispatcher_retains_all_families_with_exact_records_and_explicit_join_gaps

test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 223 filtered out; finished in 0.20s

error: test failed, to rerun pass `-p vibe-mcp --lib`
```

### /tmp/m10d1-workspace-2.log

Executed 4 cases: 3 passed, 1 failed. A projection-prefix assertion was invalid after builtin fact qualification. Replaced it with an assertion that the exact retained local fact body is emitted twice, together with actual compiler source/occurrence checks.

```text
failures:

---- boot_debugger::m_10_d_1_cases::actual_builtin_read_and_closure_keep_exact_derivative_and_repeated_occurrences stdout ----

thread 'boot_debugger::m_10_d_1_cases::actual_builtin_read_and_closure_keep_exact_derivative_and_repeated_occurrences' (2669909) panicked at crates/vibe-workspace/src/boot_debugger/m_10_d_1_cases.rs:163:5:
assertion failed: std::str::from_utf8(&artifact).unwrap().contains(&read.projection.trim()[..20])
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    boot_debugger::m_10_d_1_cases::actual_builtin_read_and_closure_keep_exact_derivative_and_repeated_occurrences

test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 765 filtered out; finished in 0.08s

error: test failed, to rerun pass `-p vibe-workspace --lib`
```

Intermediate successful runs also occurred (resolver 2, workspace 3/4, MCP 5/6, CLI 3/4), but they are not substituted for the final 2+4+6+4 results. Earlier globalfmt and full check-codegen invocations likewise exited 1 and remain failures; final outcomes above are the deciding check state.

## Author criterion claims and remaining limits

Slot 0, CLI/API/MCP inspection agrees on identity, bytes and completeness: **satisfied as an author implementation claim** for the explicit supported configured routes and finite fixtures described above. Actual standard CLI source JSON equals source API and registered MCP; actual compiled boot source identity/result/raw/projection and distinct occurrence pairs are retained by the shared service.

Slot 1, all required families appear in the debugger/source proof matrix: **satisfied as an author implementation claim** by the five-row matrix and focused families. The four ordinary/inert rows keep exact raw inventories and explicit unavailable semantic/native joins; the Vibe row proves the real builtin positive. No all-family Vibe compiler claim is made.

Open review/check outcomes: literal globalfmt and check-codegen remain Failed as qualified above; independent Root source/spec review and acceptance are outstanding. Native execution, source-proven/current-base authentication, automatic discovery, unrecorded parent/inline occurrence/span mappings and D2 activation/recovery remain their separate original limits/later horizons, not claims supplied by this report. No remaining implementation follow-up is proposed for the previously missing supported compiler join.

All Cargo processes for this continuation are terminal. Source work, fixture preparation, offline builds and content-only artifact publication are the actual effects. No derived active content was executed.
