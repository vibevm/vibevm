# ADV-SDD-CHANGE-READ-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** MCP tool `change_query` (mode list|show|check) returns as structuredContent the same change documents the library builds for the server's project, refuses unknown members before reading, and is taught in the vibevm skill template.

**Write.** `crates/vibe-mcp/src/tools/change_query.rs`, `crates/vibe-mcp/src/tools.rs`, `crates/vibe-mcp/src/skill_template.md`, `crates/vibe-mcp/tests/change_query.rs`, `vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml`

**Check.** `cargo test -p vibe-mcp --test change_query 2>&1 | grep -F "test result: ok. 3 passed"`

**Test.** crates/vibe-mcp/tests/change_query.rs (new test target, auto-discovered): `change_query_show_returns_the_library_report`, `change_query_list_and_check_return_the_library_documents`, `change_query_refuses_unknown_members_before_reading`.

**Read first.**
- crates/vibe-mcp/src/tools/requirements_query.rs (the pattern: private deny_unknown_fields args, cell/spec attributes, descriptor doctest, run)
- crates/vibe-mcp/src/tools.rs (default_tools, the requirements_query module/pub use pair)
- crates/vibe-mcp/src/tools/output.rs (ToolOutput::ok, ToolOutput::structured)
- crates/vibe-mcp/src/lib.rs (ToolError variants InvalidArguments(String), NotFound(String), Io(std::io::Error), Internal(String))
- crates/vibe-mcp/src/install.rs (test every_served_tool_is_named_in_the_skill_template) and src/skill_template.md (the requirements_query bullet)
- crates/vibe-requirements/src/change/mod.rs (list_changes, change_report, check_changes)

**Steps.**
1. Create crates/vibe-mcp/src/tools/change_query.rs with first item `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-MCP-QUERY");` and imports as in requirements_query.rs (`use serde::Deserialize; use serde_json::{Value, json}; use specmark::{cell, spec}; use vibe_workspace::Workspace; use super::{McpTool, ToolOutput}; use crate::{ServerContext, ToolDescriptor, ToolError};`). Declare `#[cell(seam = "McpTool", variant = "change_query")] #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-MCP-QUERY")] pub struct ChangeQueryMcpTool;` with a doc comment containing this compiled doctest (vibe-mcp is in gated_pub_doctest): ```
/// ```
/// use vibe_mcp::tools::{ChangeQueryMcpTool, McpTool};
///
/// let descriptor = ChangeQueryMcpTool.descriptor();
/// assert_eq!(descriptor.name, "change_query");
/// assert_eq!(descriptor.input_schema["required"], serde_json::json!(["mode"]));
/// ```
``` Add a private `#[derive(Debug, Deserialize)] #[serde(deny_unknown_fields)] struct ChangeQueryArgs { mode: String, #[serde(default)] id: Option<String>, #[serde(default)] ids: Vec<String> }`.
2. Descriptor (follow requirements_query's `fn descriptor` shape): name `change_query`. Description exactly: ```
Read authored changes of this project (PROP-071), read-only: mode list names every change, show returns one change's digests, documents, work items and origin drift, check returns whether each change still matches the installed origin bytes. structuredContent is exactly the vibe-change-list/1, vibe-change-report/1 or vibe-change-check/1 document that `vibe change ... --json` prints.
``` input_schema: `json!({"type": "object", "properties": {"mode": {"type": "string", "enum": ["list", "show", "check"]}, "id": {"type": "string"}, "ids": {"type": "array", "items": {"type": "string"}}}, "required": ["mode"], "additionalProperties": false})`.
3. run: normalise `Value::Null` args to `json!({})`. Decode with `serde_json::from_value::<ChangeQueryArgs>(args.clone())` before any filesystem access; a decode failure maps to `ToolError::InvalidArguments(format!("`change_query` takes `mode` (list|show|check), `id` for show and `ids` for check, nothing else: {error}"))`. Refuse with `ToolError::InvalidArguments(..)` when: the mode is not `list`, `show` or `check`; mode show has no `id`; `id` is given with list or check; `ids` is non-empty with list or show. Then `Workspace::discover_selected(&ctx.project_root)`, mapping an error to `ToolError::Io(std::io::Error::other(error.to_string()))`. Call `vibe_requirements::change::list_changes`, `change_report` or `check_changes(&selected, &args.ids)`. A show error maps to `ToolError::NotFound(error.to_string())`; list and check errors map to `ToolError::Io(std::io::Error::other(error.to_string()))`. Return `Ok(ToolOutput::ok(serde_json::to_value(&doc).map_err(|e| ToolError::Internal(e.to_string()))?))`. check returns ToolOutput::ok even when `passed` is false.
4. In crates/vibe-mcp/src/tools.rs (563 lines today, +3) add `pub mod change_query;` and `pub use change_query::ChangeQueryMcpTool;` directly after the `pub mod requirements_query;` / `pub use requirements_query::RequirementsQueryMcpTool;` pair, and `Box::new(ChangeQueryMcpTool),` directly after `Box::new(RequirementsQueryMcpTool),` in default_tools.
5. In crates/vibe-mcp/src/skill_template.md insert this bullet directly after the requirements_query bullet (whose last line is `  algorithmic and read-only: no sync, project write, provider or LLM call.`) and before the `query_package(name)` bullet: ```
- **`change_query(mode, id?, ids?)`** — read authored changes of the
  selected workspace node (PROP-071): `list` names every change, `show`
  returns one change's digests, documents, work items and origin drift,
  `check` reports whether each change still matches the installed origin
  bytes. It returns the same document `vibe change ... --json` prints. The
  tool is read-only: no sync, install, project write or network.
```
6. Write crates/vibe-mcp/tests/change_query.rs. Fixture with std::fs in a tempdir: vibe.toml `[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n` and the change at `<vibe_core::layout::current_specs_root()>/changes/chg-0123456789abcdef0123456789abcdef/` exactly as entry 1's fixture (TEXT from `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../formats/corpora/change-manifest/e1/valid/representative.toml"))` plus the work row, intent.md, acceptance.md, work.toml with the WORK text, no closure.md, no lock). `let ctx = vibe_mcp::ServerContext::new(root.path());` and `let sel = vibe_workspace::Workspace::discover_selected(root.path()).unwrap();`. Test 1: `ChangeQueryMcpTool.run(&json!({"mode":"show","id":ID}), &ctx).unwrap().structured()` equals `&serde_json::to_value(change_report(&sel, ID).unwrap()).unwrap()`. Test 2: the same equality for `{"mode":"list"}` against `list_changes(&sel)` and for `{"mode":"check"}` against `check_changes(&sel, &[])`. Test 3: `{"mode":"list","path":"elsewhere"}` and `{"mode":"show"}` both satisfy `matches!(result, Err(ToolError::InvalidArguments(_)))`.
7. Insert this line in PROP-071 directly before the closing `</spec>` line (after entry 3's line): ```
  <p><CHANGE-MCP-QUERY fact="true" status="impl/done">The MCP tool change_query takes mode list, show or check, an id for show and optional ids for check, and nothing else; unknown members refuse before any read. It answers for the server's own project root and returns as structuredContent the same vibe-change-list/1, vibe-change-report/1 or vibe-change-check/1 document the CLI prints with --json, built by the same library calls.</CHANGE-MCP-QUERY></p>
```

**Decisions already made.**
- There is no `path` argument: the node is the server's project_root, as for requirements_query.
- Parity is by construction (one library call) and tested against the library value. No test spawns the CLI binary.
- The text channel is ToolOutput::ok's deterministic pretty JSON. No separate text renderer.
- The existing lib test `every_served_tool_is_named_in_the_skill_template` (install.rs) fails unless skill_template.md names `change_query`; the skill-template step keeps it passing. The bullet must not contain any retired identity listed in `skill_template_teaches_no_retired_identities`. Neither test is part of the check.
- This entry runs after entry 3 only because both edit PROP-071. It needs no code from entry 3.
- vibe-requirements, vibe-workspace, vibe-core, serde, serde_json and (dev) tempfile are already dependencies of vibe-mcp. Cargo.toml does not change.

**Not in this entry.** MCP progress or cancellation, a `path` or spec-root argument, editing other tools to strict decoding (ADV-CONTROL's strict MCP arguments), and a CLI-binary parity test.

**Depends on.** ADV-SDD-CHANGE-READ-2, ADV-SDD-CHANGE-READ-3

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml: add unit CHANGE-MCP-QUERY (status impl/done), exact text in the last step.

Entries that write the same file run one after another, never in parallel.
