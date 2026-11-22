# ADV-SDD-SPEC-SECTION-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe show section <ADDRESS>` prints a one-line origin header followed by the section text, and `vibe --json show section <ADDRESS>` prints the same JSON object that the MCP `read_spec` tool returns.

**Write.** `crates/vibe-cli/src/cli/inspect.rs`, `crates/vibe-cli/src/commands/show/mod.rs`, `crates/vibe-cli/src/commands/show/section.rs`, `crates/vibe-cli/src/command_progress.rs`, `crates/vibe-cli/tests/cli_show_section.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_show_section`

**Test.** crates/vibe-cli/tests/cli_show_section.rs (new). Header (exact): `mod common;\n\nuse common::UserScratch;\nuse serde_json::{Value, json};\nuse vibe_mcp::ServerContext;\nuse vibe_mcp::tools::{McpTool, ReadSpecMcpTool};`. McpTool must be imported for `.run`; copy the pattern of crates/vibe-cli/tests/cli_mcp_path_parity.rs:1-12. Fixture: a tempdir with vibe.toml `[project]\nname = "host"\ngroup = "org.demo"\nversion = "0.1.0"\n` and `vibevm/vibespecs/common/OWN.md` = `# Own {#root}\n\n## Commits {#commits}\n\nCOMMIT_RULES\n\n## Outer {#outer}\n\nOUTER_ONLY\n`. Every command is `UserScratch::new().vibe()` with `.env_remove("VIBE_UNATTENDED").env_remove("VIBE_INVOKED_BY")`. Tests: (1) plain_output_prints_origin_header_then_section_text: `vibe show section spec://org.demo/host/common/OWN#commits --path <dir>` succeeds; the first stdout line starts with `--- spec://org.demo/host/common/OWN#commits (org.demo/host@0.1.0, vibevm/vibespecs/common/OWN.md, authored, sha256:`; stdout contains COMMIT_RULES and not OUTER_ONLY. (2) json_output_carries_origin_fields: stdout of `vibe --json show section … --path <dir>` parsed as JSON: ok==true, command=="show section", package=="org.demo/host", version=="0.1.0", file=="vibevm/vibespecs/common/OWN.md", format=="markdown", disposition=="authored", sha256 is 64 lowercase hex chars, text contains COMMIT_RULES. (3) cli_json_equals_mcp_read_spec: the parsed CLI JSON == ReadSpecMcpTool.run(&json!({"address": "spec://org.demo/host/common/OWN#commits"}), &ServerContext::with_store_root(dir.to_path_buf(), dir.join("store-root"))).unwrap().structured().

**Read first.**
- crates/vibe-cli/src/cli/inspect.rs:1-120
- crates/vibe-cli/src/commands/show/mod.rs:1-71
- crates/vibe-cli/src/commands/show/source_path.rs:55-80
- crates/vibe-cli/src/command_progress.rs:360-372
- crates/vibe-cli/tests/cli_mcp_path_parity.rs:1-80
- crates/vibe-workspace/src/spec_section.rs

**Steps.**
1. In crates/vibe-cli/src/cli/inspect.rs add the variant `/// Print one spec section by its `spec://` address, from the version the lock selects.\n    Section(ShowSectionArgs),` as the last ShowSubcommand variant. Add the ShowSectionArgs struct from decisions after ShowSourcePathArgs.
2. In crates/vibe-cli/src/commands/show/mod.rs add `mod section;` in alphabetical order (after `mod purls;`) and the arm `ShowSubcommand::Section(sub) => section::run_section(ctx, sub),`.
3. In crates/vibe-cli/src/command_progress.rs add `| ShowSubcommand::Section(_)` to the existing `FiniteFallback("Command show")` arm.
4. Create crates/vibe-cli/src/commands/show/section.rs per decisions.
5. Write crates/vibe-cli/tests/cli_show_section.rs with the header and the three tests named in `test`, then run the check.

**Decisions already made.**
- ShowSectionArgs (exact): `#[derive(Debug, clap::Args)] pub struct ShowSectionArgs { /// `spec://<group>/<name>[@<version>]/<document>[#anchor]`. pub address: String, /// Expand `#embed` directives in the section text. #[arg(long)] pub expand_embeds: bool, /// Project root. Defaults to current directory. #[arg(long, default_value = ".")] pub path: PathBuf }`. --json is the existing global flag.
- section.rs starts with `specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");` (as in features.rs and purls.rs) and defines `pub(super) fn run_section(ctx: &output::Context, args: ShowSectionArgs) -> anyhow::Result<()>`.
- run_section: `let root = super::resolve_project_root(&args.path)?; let read = vibe_workspace::spec_section::read(&root, &args.address, ReadOptions { expand_embeds: args.expand_embeds })?;`. The SectionRefusal is returned as the anyhow error unchanged, with no .context().
- JSON mode: `if ctx.is_json() { return ctx.emit_json(&read); }`.
- Plain mode header (exact): `--- {address} ({package}@{version}, {file}, {disposition}, sha256:{sha256})`. When version is None the parenthesis starts `({package}, `. Print the disposition with Disposition::as_str. Print the header with `println!`, then `print!("{}", read.text)`, then a trailing `println!()` only when the text does not end with '\n'.
- A refusal keeps the generic exit code 1 (no exit_code.rs change).

**Not in this entry.** The `kind` field on the JSON refusal (entry 4), new exit codes, a `--format` flag, touching `vibe show effective` / `source-path`, or regenerating specmap.json (entry 5).

**Depends on.** ADV-SDD-SPEC-SECTION-1, ADV-SDD-SPEC-SECTION-2

Entries that write the same file run one after another, never in parallel.
