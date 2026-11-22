# ADV-CONTROL-MCP-STRICT-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The `query`, `select` and `explain` MCP tools refuse an unknown or wrong-typed argument with `isError:true` naming the member (e.g. `invalid arguments: limit: invalid type: string "10", expected u64`) instead of silently defaulting it, and refuse `limit: 0`. A `limit` above 200 is still clamped by the library, not refused.

**Write.** `crates/vibe-mcp/src/tools/args.rs`, `crates/vibe-mcp/src/tools.rs`, `crates/vibe-mcp/src/tools/query.rs`, `crates/vibe-mcp/src/tools/select.rs`, `crates/vibe-mcp/src/tools/explain.rs`, `crates/vibe-mcp/tests/strict_arguments.rs`, `vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test strict_arguments --test tools_oracle`

**Test.** New file crates/vibe-mcp/tests/strict_arguments.rs. After the module doc comment its first lines are exactly: `use vibe_test_support as _;` then `use serde_json::{Value, json};` then `use vibe_mcp::tools::{ExplainMcpTool, McpTool, QueryMcpTool, SelectMcpTool};` then `use vibe_mcp::{ServerContext, ToolError};`. Every test starts with `let dir = tempfile::tempdir().unwrap();` and, for direct calls, `let ctx = ServerContext::new(dir.path());`. Tests: (1) `query_refuses_a_string_limit`: `QueryMcpTool.run(&json!({"limit":"10"}), &ctx)` matches `Err(ToolError::InvalidArguments(m))` with `m.starts_with("limit: invalid type")`. (2) `query_refuses_an_unknown_member`: `{"unexpected":true}` gives `Err(ToolError::InvalidArguments(m))` with `m.starts_with("unexpected: unknown field")`. (3) `query_refuses_limit_zero`: `{"limit":0}` gives `Err(ToolError::InvalidArguments(m))` with `m == "limit: must be at least 1"`. (4) `explain_refuses_a_string_json`: `ExplainMcpTool.run(&json!({"target":"x","json":"true"}), &ctx)` gives `Err(ToolError::InvalidArguments(m))` with `m.starts_with("json: invalid type")`. (5) `select_refuses_an_unknown_member`: `SelectMcpTool.run(&json!({"where":"kind:fn","depth":1}), &ctx)` gives `Err(ToolError::InvalidArguments(m))` with `m.starts_with("depth: unknown field")`. (6) `query_wrong_limit_is_an_error_result_through_dispatch`: `let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"query","arguments":{"limit":"wrong"}}}).to_string(); let reply = vibe_mcp::dispatch_one(ServerContext::new(dir.path()), &req).unwrap(); let v: Value = serde_json::from_str(&reply).unwrap();` then assert `v["result"]["isError"] == true` and `v["result"]["content"][0]["text"].as_str().unwrap().contains("invalid arguments: limit: invalid type")` (dispatch_one takes the context by value and returns `Result<String, ServerError>`, so build a fresh context and unwrap). Use `match`/`assert!(matches!(..))` with a message showing the actual result. The existing tools_oracle tests must still pass unchanged.

**Read first.**
- crates/vibe-mcp/src/tools/query.rs:38-104
- crates/vibe-mcp/src/tools/select.rs:38-94
- crates/vibe-mcp/src/tools/explain.rs:30-68
- crates/vibe-mcp/src/tools.rs:525-563
- crates/vibe-mcp/src/lib.rs:104-135
- vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml:62-75

**Steps.**
1. Apply the spec_edit text verbatim to vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml: insert the `<TOOL-ARGS-STRICT fact="true" status="impl/plan">…</TOOL-ARGS-STRICT>` element on the line right after the line ending `</TOOL-CELL-BOUNDARY>` (line 72), inside the same `<facts ordered="false">` block. Change nothing else in that file.
2. Create crates/vibe-mcp/src/tools/args.rs starting with `specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");` and a function `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#TOOL-ARGS-STRICT")] pub(crate) fn decode_args<T: serde::de::DeserializeOwned>(args: &serde_json::Value) -> Result<T, crate::ToolError>`: `Value::Null` becomes an empty `serde_json::Map`; `Value::Object(m)` uses `m.clone()`; any other value returns `ToolError::InvalidArguments(format!("arguments must be a JSON object, got {other}"))`. Decode with `serde_json::from_value::<T>(Value::Object(object.clone()))`; on error find the culprit as `object.keys().find(|key| { let without: serde_json::Map<String, Value> = object.iter().filter(|(k, _)| k != key).map(|(k, v)| (k.clone(), v.clone())).collect(); match serde_json::from_value::<T>(Value::Object(without)) { Ok(_) => true, Err(other) => other.to_string() == format!("missing field `{key}`") } })`; return `ToolError::InvalidArguments(format!("{key}: {error}"))` when a culprit is found, else `ToolError::InvalidArguments(error.to_string())`.
3. In the same file add `pub(crate) fn resolve_limit(limit: Option<u64>) -> Result<usize, crate::ToolError>`: `None` -> `Ok(vibe_trace::search::DEFAULT_LIMIT)`; `Some(0)` -> `Err(ToolError::InvalidArguments("limit: must be at least 1".into()))`; `Some(n)` -> `Ok(usize::try_from(n).unwrap_or(usize::MAX))`. Give each function a one-line doc comment. In crates/vibe-mcp/src/tools.rs add exactly one line `pub(crate) mod args;` directly above the line `pub mod agentic_explain;` (line 531); change nothing else in tools.rs.
4. In query.rs add `#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] struct QueryArgs { #[serde(default)] uri: Option<String>, #[serde(default)] symbol: Option<String>, #[serde(default)] kind: Option<String>, #[serde(default)] limit: Option<u64> }`; replace the manual argument reads in `run` (lines 74-84) with `let args: QueryArgs = super::args::decode_args(args)?; let limit = super::args::resolve_limit(args.limit)?;` and build `Filters` from `args.uri`, `args.symbol`, `args.kind`, `limit`.
5. In select.rs add `#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] struct SelectArgs { #[serde(rename = "where")] where_: String, #[serde(default)] limit: Option<u64> }`; replace the manual reads (lines 67-75) with `let args: SelectArgs = super::args::decode_args(args)?; let limit = super::args::resolve_limit(args.limit)?;` and use `&args.where_` where `query` was used; keep the `vibe_trace::select::parse` call and everything after it unchanged. In explain.rs add `#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] struct ExplainArgs { target: String, #[serde(default)] json: bool }`; replace lines 56-60 with `let args: ExplainArgs = super::args::decode_args(args)?;` and call `vibe_trace::explain(&ctx.project_root, &args.target, args.json)`.
6. Write crates/vibe-mcp/tests/strict_arguments.rs with the six tests named in `test`, run `cargo fmt -p vibe-mcp`, then run the check.

**Decisions already made.**
- Helper lives in a new file crates/vibe-mcp/src/tools/args.rs (tools.rs is 563 lines, near its 600-line budget); module declared `pub(crate) mod args;` in tools.rs.
- This entry owns the PROP-015 spec edit: it inserts TOOL-ARGS-STRICT with status impl/plan before writing code that names the anchor; entry 2 flips it to impl/done.
- Error channel: `Err(ToolError::InvalidArguments(..))` returned before any map build/search runs; its rendered text is `invalid arguments: <member>: <serde message> (violates ...)`.
- Message format: `<member>: <serde error text>` when one member is identified, plain serde text otherwise (e.g. `missing field `target``). No new dependency (no serde_path_to_error).
- Absent or `null` arguments decode as `{}`.
- `limit` semantics: default 50 (`vibe_trace::search::DEFAULT_LIMIT`), 0 refused with exactly `limit: must be at least 1`, values above 200 passed through and clamped by the library (unchanged, though the schema advertises maximum 200).
- `json` defaults to false via `#[serde(default)]`; a string `"true"` is refused.
- Descriptors (input_schema JSON) and PROTOCOL_VERSION are not changed.
- No `#[spec(verifies)]` attributes on the new tests.

**Not in this entry.** Not in this entry: refusing `limit` above 200 (it stays clamped), converting query_package / read_subskill / materialise_subskill / list_tools / agentic_explain, the default_tools() sweep test, or any generic JSON-Schema validator or dispatcher-level validation in lib.rs.

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml: inside the `<facts ordered="false">` block of `<tools title="2.2 The tool surface: one seam, three tools">`, immediately after the line ending `</TOOL-CELL-BOUNDARY>` (line 72), insert:
        <TOOL-ARGS-STRICT fact="true" status="impl/plan">Every tool decodes its
  `arguments` strictly against its advertised input schema before it reads
  the project: an unknown member or a wrong-typed member is refused with
  `isError: true` naming the member, never ignored or defaulted. Absent or
  `null` `arguments` decode as an empty object.</TOOL-ARGS-STRICT>

Entries that write the same file run one after another, never in parallel.
