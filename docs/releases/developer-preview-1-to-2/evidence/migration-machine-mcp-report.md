# C2 MCP transport — worker report

Candidate implementation; writer stopped. Only five authorized files changed:

- crates/vibe-mcp/src/lib.rs: neutral ToolError::StructuredPreflight { report: Value, message: String }, Class-F governing citation/fix, generic Err-branch carriage of structuredContent only for that variant. Ordinary errors retain text-only results. Existing explain error test now asserts that distinction.
- crates/vibe-mcp/src/tools.rs: one module/reexport and one default_tools entry for PlanManifestMigrationMcpTool; trait documentation distinguishes preflight from executed results.
- crates/vibe-mcp/src/tools/output.rs: documentation only, acknowledging typed preflight remains outside ToolOutput.
- crates/vibe-mcp/src/tools/migration_plan.rs: descriptor plan_manifest_migration with schema1/source_text required, three optional nonnullable string selections and additionalProperties=false. run ignores ServerContext and invokes the authoritative workspace plan_migration_value. Preflight uses Err StructuredPreflight; domain refusal uses executed_failure; questions/planned/no-op use executed nonerror. Serialization is exactly the inner generated report/error, without an enum wrapper or Manifest.
- crates/vibe-mcp/src/tools/migration_plan/tests.rs: five bounded cases through real dispatch_one/Server and MemoryTransport, explicit with_store_root absent project/store paths and actual directory/file-content snapshots before/after dispatch.

No dispatcher tool-name/domain logic, context bootstrap, configuration, registry/project discovery, filesystem/cache/lock/provider planning or duplicate methods/hash semantics were introduced. Generic probes prove an unrelated tool can retain typed preflight and ordinary InvalidArguments still has no structuredContent. The new registration changes the default tool denominator; root owns corresponding inventory/source-capture accounting.

## Actual selected native evidence

Only after root's stable workspace API9case signal:

`CARGO_TARGET_DIR=/home/olegchir/git/v/vibevm-next/target CARGO_BUILD_JOBS=4 cargo test --locked --offline -p vibe-mcp --lib tools::migration_plan::tests:: -- --nocapture`

Exit0:5passed/0failed/67filtered. Full stdout/stderr/compiler log: /tmp/vibevm-next-M13-C2-mcp-tests.log. Cases: registered_descriptor_is_strict_without_context_files; planner_outcomes_match_pure_adapter_and_correct_channels; epoch_and_utf8_crlf_source_are_carried_losslessly; malformed_null_unknown_fields_and_schema_are_structured_preflight; generic_preflight_and_ordinary_errors_remain_distinct.

Full DTO equality covers Planned/NoOp/NeedsInput and two domain refusal inputs. Null/wrong-type optional choices, nonobject/missing/wrong required data, unknown transport fields and schema2 retain typed preflight with citation/fix and no parser/private-path echo. UTF8/CRLF and explicit epoch1 no-op survive dispatch. Snapshot comparisons include existing sentinel bytes; absent project/store/.vibe/manifest/lock paths remain absent.

Exact existing selector `tests::explain_tool_runs_and_reports_an_unresolvable_target_in_band -- --exact` also exited0:1passed/0failed/71filtered. Actual log: /tmp/vibevm-next-M13-C2-mcp-legacy-tests.log. No failure/retry and no broad67case or whole-crate test run.

Targeted rustfmt five owners with skip_children=true exited0. Final lines lib532/tools584/output454/newtool87/newtests322, all≤600. No Cargo dependency, context.rs, source law/schema/generated/registry edits; no codegen, generic gates, Git writes, provider or public operations. Root owns commit, inventory accounting and semantic acceptance. This proves the bounded MCP transport, not final facade, broad historical corpus, effectful apply or public distribution authority.
