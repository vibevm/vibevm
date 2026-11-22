# ADV-SDD-TREE-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `tools/call tree` on `vibe mcp` returns the project's package tree as structuredContent. The document validates against package-tree.schema.v1.json and has no `ok` or `command` keys. Any argument is refused as invalid arguments.

**Write.** `crates/vibe-mcp/src/tools/tree.rs`, `crates/vibe-mcp/src/tools.rs`, `crates/vibe-mcp/src/skill_template.md`, `crates/vibe-mcp/Cargo.toml`, `crates/vibe-mcp/tests/tree_tool.rs`, `Cargo.lock`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test tree_tool && bash tools/cargo-test-ran.sh -p vibe-mcp --lib every_served_tool_is_named_in_the_skill_template`

**Test.** New file crates/vibe-mcp/tests/tree_tool.rs, starting with `use vibe_test_support as _;`. Imports: `serde_json::{Value, json}`, `vibe_mcp::tools::{McpTool, ToolError, TreeMcpTool, default_tools}` (take ToolError from the path that read_doc.rs uses), and `vibe_mcp::{ServerContext, dispatch_one}`. Fixture helper `fn project() -> (tempfile::TempDir, ServerContext)`: create `vibe_core::layout::current_boot_dir()` under a tempdir and write `vibe.toml` = `"[project]\nname = \"lane\"\nversion = \"0.1.0\"\nspec_format = \"xml\"\n"`. ctx = `ServerContext::new(dir.path().to_path_buf())`. (1) `tree_is_registered_with_a_closed_empty_input_schema`: the names from `default_tools()` descriptors contain "tree"; `TreeMcpTool.descriptor().input_schema == json!({"type":"object","properties":{},"additionalProperties":false})`; `vibe_mcp::install::SKILL_TEMPLATE.contains("**`tree()`**")`. (2) `tree_call_returns_the_schema_v1_document_without_envelope`: `let reply = dispatch_one(ctx, &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tree","arguments":{}}}).to_string()).expect("a reply"); let v: Value = serde_json::from_str(&reply).unwrap();`. Assert `v["error"].is_null()` and `v["result"]["isError"] == json!(false)`. sc = `v["result"]["structuredContent"]` must be an object with no `ok` and no `command` key, with `sc["schema_version"] == json!(1)` and `sc["boot"]["static_lane_name"] == json!("STATIC.xml")`. sc must validate with zero errors under `jsonschema::validator_for(&schema).unwrap().iter_errors(&sc)`, where schema = `serde_json::from_str(include_str!("../../vibe-cli/resources/package-tree.schema.v1.json")).unwrap()`. With key `generated_at` removed from both, sc must equal `serde_json::to_value(vibe_workspace::tree::build_tree_bound(dir.path(), vibe_core::manifest::SpecFormat::Xml).unwrap()).unwrap()`. (3) `tree_refuses_any_argument`: `TreeMcpTool.run(&json!({"path":"elsewhere"}), &ctx)` matches `Err(ToolError::InvalidArguments(msg))`, and msg contains "`tree` takes no arguments" and "path". Also, `TreeMcpTool.run(&json!(5), &ctx)` is `Err(ToolError::InvalidArguments(_))`. All three tests fail to compile before the change.

**Read first.**
- crates/vibe-mcp/src/tools.rs:1-140
- crates/vibe-mcp/src/tools/read_doc.rs:1-60
- crates/vibe-mcp/src/tools/output.rs:120-145
- crates/vibe-mcp/tests/tools_oracle.rs:1-20,285-315
- crates/vibe-mcp/src/skill_template.md:95-110
- crates/vibe-cli/src/commands/reinstall.rs:295-306

**Steps.**
1. Create crates/vibe-mcp/src/tools/tree.rs with `pub struct TreeMcpTool;`, annotated `#[cell(seam = "McpTool", variant = "tree")]` and `#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")]` the way read_doc.rs annotates its tool. Add a doctest `use vibe_mcp::tools::McpTool; assert_eq!(vibe_mcp::tools::TreeMcpTool.descriptor().name, "tree");`. The descriptor has name "tree", input_schema `{"type":"object","properties":{},"additionalProperties":false}`, and the description from decisions.
2. Implement `run(&self, args, ctx)`, starting with argument validation. `Value::Null` and an empty object pass. A non-empty object returns `Err(ToolError::InvalidArguments(format!("`tree` takes no arguments; unexpected: {}", keys.join(", "))))`, with keys in the map's iteration order. Any other value returns `Err(ToolError::InvalidArguments("`tree` takes no arguments; pass {} or omit arguments".to_string()))`.
3. Continue run: `let manifest = vibe_core::manifest::Manifest::read(ctx.project_root.join(vibe_core::manifest::Manifest::FILENAME))` with map_err `ToolError::Internal(format!("reading vibe.toml: {e}"))`. `let user = vibe_core::user_config::UserConfig::load()` with map_err `ToolError::Internal(format!("loading the user config: {e}"))`. `let format = vibe_orchestrator::resolve_spec_format(&manifest, user.install.spec_format);`. `let tree = vibe_workspace::tree::build_tree_bound(&ctx.project_root, format)` with map_err `ToolError::Internal(format!("building the package tree: {e:#}"))`. Serialise with `serde_json::to_value(&tree)`, map_err `ToolError::Internal(format!("serialising the package tree: {e}"))`, and return `Ok(ToolOutput::ok(value))`.
4. crates/vibe-mcp/src/tools.rs: after the `pub use read_doc::ReadDocMcpTool;` line add `/// The package-tree view (PROP-015 `#TOOL-TREE`): the shared\n/// `vibe_workspace::tree` model that `vibe tree --json` also renders.\npub mod tree;\npub use tree::TreeMcpTool;`. In `default_tools()` add `Box::new(TreeMcpTool),` directly after `Box::new(ListToolsMcpTool),`.
5. crates/vibe-mcp/src/skill_template.md: insert the bullet from decisions directly after the `list_tools()` bullet (the one that starts `- **`list_tools()`**` at line 101 and ends with `an error.`), leaving one bullet per list item. crates/vibe-mcp/Cargo.toml [dev-dependencies]: add `jsonschema.workspace = true`, and add `tempfile.workspace = true` if it is not already there.
6. Create crates/vibe-mcp/tests/tree_tool.rs with the three tests described in `test`. Run `cargo fmt -p vibe-mcp`, then run the check.

**Decisions already made.**
- The tool is named `tree` and takes zero arguments in v1. Its input schema is closed (`additionalProperties: false`). The tool refuses arguments itself and does not rely on a dispatcher-wide schema check.
- The MCP result is the schema document without the CLI envelope keys (`ok`, `command`, `invoked_by`, `unattended`). It is returned via `ToolOutput::ok(value)` so that it lands as `structuredContent`. The schema stays at v1 with no new fields.
- Format selection mirrors `vibe tree --json` exactly: `vibe_orchestrator::resolve_spec_format(&manifest, UserConfig::load()?.install.spec_format)`, then `vibe_workspace::tree::build_tree_bound`. vibe-mcp already depends on vibe-orchestrator, vibe-core and vibe-workspace. The project root is always `ctx.project_root`, and there is no path argument.
- Descriptor description text: "The project's package tree — the same document `vibe tree --json` prints (package-tree schema v1) without the CLI `ok`/`command` keys: every locked package with its effective boot load type, the flags that explain it, its source and provenance, plus the boot lanes, in-place specs and diagnostics. Read-only; takes no arguments."
- Skill-template bullet text, inserted verbatim:\n- **`tree()`** — the project's package tree: the same document\n  `vibe tree --json` prints (package-tree schema v1) without the CLI\n  `ok`/`command` keys — every locked package, its effective boot load\n  type and why, its source and provenance, the boot lanes and\n  diagnostics. Takes no arguments; read-only.
- The test compares against the shared builder and not the `vibe` binary, because vibe-mcp cannot run the CLI. Parity with `vibe tree --json` holds because both surfaces call the same owner, and ADV-SDD-TREE-1 keeps tree_json green.
- `every_served_tool_is_named_in_the_skill_template` only checks for the substring "tree", which the template already contains (lines 151 and 153), so it is just a regression guard. The real gate for the bullet is test (1)'s `SKILL_TEMPLATE.contains("**`tree()`**")`.
- The new code lives in src/tools/tree.rs and not in tools.rs, which is at its size ceiling. tools.rs gains only the module line, the re-export and the registration.

**Not in this entry.** Not in this entry: a `path`/`package` filter or any other argument, a text/plain or layers rendering, a dispatcher-wide unknown-argument check for all tools, or edits to the lib.rs crate docs and PROP-015 (the planner applies the spec unit).

**Depends on.** ADV-SDD-TREE-1

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml, inside `<tools>`, in the `<facts ordered="false">` list that follows TOOLS-IO-CONVENTION. Insert directly after the line ending `</TOOL-MATERIALISE-SUBSKILL>`:
        <TOOL-TREE fact="true" status="spec/done" action="continue" actionstage="doc" audience="agent">**`tree`** — the project's package tree: exactly the document `vibe tree --json` prints (`package-tree.schema.v1.json`, schema version 1) without the CLI envelope keys (`ok`, `command`) — every locked package with its effective boot load type, the flags that explain it, its source and provenance, plus the boot lanes, in-place specs and diagnostics. Both surfaces are built by the one shared `vibe_workspace::tree` owner. Takes no arguments; any argument is refused as invalid arguments. Read-only.</TOOL-TREE>

Entries that write the same file run one after another, never in parallel.
