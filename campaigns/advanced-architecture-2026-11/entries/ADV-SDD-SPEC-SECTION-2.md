# ADV-SDD-SPEC-SECTION-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The MCP server lists a `read_spec` tool. Calling it with {address, expand_embeds?} returns the same JSON object as `spec_section::read`. A refused address returns an error result whose structured content carries `kind`.

**Write.** `crates/vibe-mcp/src/tools/read_spec.rs`, `crates/vibe-mcp/src/tools.rs`, `crates/vibe-mcp/src/skill_template.md`, `crates/vibe-mcp/tests/read_spec.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test read_spec`

**Test.** crates/vibe-mcp/tests/read_spec.rs (new). Imports (exact): `use serde_json::json; use vibe_mcp::tools::{McpTool, ReadSpecMcpTool}; use vibe_mcp::{ServerContext, ToolError, default_tools};`. McpTool must be imported for `.run` to be callable; see crates/vibe-cli/tests/cli_mcp_path_parity.rs:1-12. Fixture: a tempdir with vibe.toml `[project]\nname = "host"\ngroup = "org.demo"\nversion = "0.1.0"\n` and `vibevm/vibespecs/common/OWN.md` = `# Own {#root}\n\n## Commits {#commits}\n\nCOMMIT_RULES\n`; ctx = ServerContext::with_store_root(dir.to_path_buf(), dir.join("store-root")). Tests: (1) read_spec_is_registered_in_default_tools: the names in default_tools() contain "read_spec". (2) read_spec_returns_the_library_report: ReadSpecMcpTool.run(&json!({"address":"spec://org.demo/host/common/OWN#commits"}), &ctx) → !is_error(); structured() == serde_json::to_value(vibe_workspace::spec_section::read(dir, "spec://org.demo/host/common/OWN#commits", Default::default()).unwrap()).unwrap(); structured()["disposition"] == "authored". (3) read_spec_refusal_names_kind: address "spec://no.such/pkg/X#a" → an Ok output with is_error()==true, structured()["kind"]=="not-installed" and structured()["ok"]==false. (4) read_spec_rejects_unknown_argument: {"address":"spec://org.demo/host/common/OWN","extra":1} → Err(ToolError::InvalidArguments(_)). (5) skill_template_names_read_spec: include_str!("../src/skill_template.md") contains "read_spec(address, expand_embeds?)".

**Read first.**
- crates/vibe-mcp/src/tools/read_doc.rs:1-110
- crates/vibe-mcp/src/tools.rs:19-95
- crates/vibe-mcp/src/tools/output.rs:110-250
- crates/vibe-mcp/src/skill_template.md:170-190
- crates/vibe-cli/tests/cli_mcp_path_parity.rs:1-12
- crates/vibe-workspace/src/spec_section.rs

**Steps.**
1. Create crates/vibe-mcp/src/tools/read_spec.rs containing `specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools");`, `#[cell(seam = "McpTool", variant = "read_spec")] #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#tools")] pub struct ReadSpecMcpTool;` and its McpTool impl per decisions. Use the same `use` lines for cell/spec attributes that read_doc.rs uses.
2. In crates/vibe-mcp/src/tools.rs add `pub mod read_spec;` and `pub use read_spec::ReadSpecMcpTool;` directly after the read_doc lines. Add `Box::new(ReadSpecMcpTool),` as the last element of default_tools().
3. In crates/vibe-mcp/src/skill_template.md insert the bullet given in decisions immediately after the `read_doc` bullet, i.e. after the line ending `to read it.`.
4. Write crates/vibe-mcp/tests/read_spec.rs with the imports and the five tests named in `test`, then run the check.

**Decisions already made.**
- Tool name: `read_spec`. read_doc is unchanged.
- Descriptor description (exact): "Read one spec section by its `spec://<group>/<name>[@<version>]/<document>[#anchor]` address from the version this project's vibe.lock selects (the project's own tree for its own coordinate). Returns package, version, file, format, disposition (authored/converted/copied), sha256 and text: the same object `vibe show section --json` prints. A refusal carries `kind`: invalid-address, not-installed, version-mismatch, doc-not-found, anchor-not-found, outside-package or unreadable-source. Read-only."
- input_schema (exact): {"type":"object","properties":{"address":{"type":"string","description":"`spec://<group>/<name>[@<version>]/<document>[#anchor]`; without #anchor the whole document is returned."},"expand_embeds":{"type":"boolean","description":"Expand `#embed` directives in the returned text. Default: false."}},"required":["address"],"additionalProperties":false}.
- Arguments are decoded with serde: `#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] struct ReadSpecArgs { address: String, #[serde(default)] expand_embeds: bool }`, via serde_json::from_value(args.clone()). A decode error → ToolError::InvalidArguments(error.to_string()).
- Success → ToolOutput::ok(serde_json::to_value(&read)). A serialisation error → ToolError::Internal("serialising read_spec report: {e}").
- Refusal → Ok(ToolOutput::executed_failure(json!({"ok": false, "kind": refusal.kind.as_str(), "error": refusal.message}), refusal.message.clone())).
- The project root is ctx.project_root.
- Skill template bullet (exact, 2-space continuation indent): "- **`read_spec(address, expand_embeds?)`** — one spec section by its\n  `spec://<group>/<name>[@<version>]/<document>[#anchor]` address, read from\n  the version `vibe.lock` selects. Returns `package`, `version`, `file`,\n  `format`, `disposition`, `sha256` and `text` — the object\n  `vibe show section --json` prints. Use it instead of guessing a\n  `vibedeps` path."

**Not in this entry.** A vibe-wire generated request schema for read_spec, strict-argument changes to other MCP tools, any change to read_doc, or regenerating specmap.json (entry 5).

**Depends on.** ADV-SDD-SPEC-SECTION-1

Entries that write the same file run one after another, never in parallel.
