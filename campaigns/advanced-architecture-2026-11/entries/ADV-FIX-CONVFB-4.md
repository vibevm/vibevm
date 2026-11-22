# ADV-FIX-CONVFB-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe install` under spec_format = "xml" of a package with an unparseable Markdown spec exits 0. Its --json install document carries conversion_fallbacks naming the file with kept_format "markdown", and human mode prints a `! org.example/legacy@1.0.0: vibevm/vibespecs/PROP-001.md kept as Markdown (conversion failed: …)` line on stderr.

**Write.** `crates/vibe-orchestrator/src/install/outcome.rs`, `crates/vibe-orchestrator/src/install/ready.rs`, `crates/vibe-cli/src/commands/install/draft.rs`, `crates/vibe-cli/tests/cli_conversion_fallback.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_conversion_fallback`

**Test.** New file crates/vibe-cli/tests/cli_conversion_fallback.rs (`mod common;`, UserScratch). It copies write_package, file_url and the registry.toml setup from crates/vibe-cli/tests/cli_install_rollback.rs and the project setup with spec_format = "xml" inserted after [project]. Package org.example/legacy@1.0.0 has boot snippet "# Legacy\n\nlegacy rule.\n" plus the file common::spec_rel("PROP-001.md") = "# H {#h}\n\n@fact:DUP one\n\n@fact:DUP two\n". Package org.example/clean has only its boot snippet. Tests: (1) json_report_names_the_unconverted_spec_document runs `vibe --json install org.example/legacy --assume-yes --path <p>`; it succeeds. Parse stdout with serde_json::Deserializer::from_slice(..).into_iter::<Value>() and take the document whose "command" == "install". Its conversion_fallbacks[0] has package "org.example/legacy", version "1.0.0", file common::spec_rel("PROP-001.md"), kept_format "markdown", and a reason containing "twice". (2) human_output_warns_kept_as_markdown runs the same install without --json in a fresh project; it succeeds and stdout+stderr contains format!("{} kept as Markdown (conversion failed:", common::spec_rel("PROP-001.md")). (3) clean_package_reports_no_conversion_fallbacks: --json install of org.example/clean succeeds and the install document has no "conversion_fallbacks" key.

**Read first.**
- crates/vibe-orchestrator/src/install/outcome.rs:100-160
- crates/vibe-orchestrator/src/install/ready.rs:270-312
- crates/vibe-cli/src/commands/install/draft.rs:35-160,275-300
- crates/vibe-cli/tests/cli_install_rollback.rs
- crates/vibe-cli/src/output/language/block.rs:236-245
- crates/vibe-cli/src/output/lines.rs:83-100

**Steps.**
1. vibe-orchestrator outcome.rs: add `pub conversion_fallbacks: Vec<vibe_workspace::install::ConversionFallback>` to InstallRun with a doc comment, initialised to Vec::new() in InstallRun::new.
2. vibe-orchestrator ready.rs finish_ready: next to `run.progress = tail.applied.progress.clone();` add `run.conversion_fallbacks = tail.applied.outcome.conversion_fallbacks.clone();`.
3. vibe-cli install/draft.rs: add `pub(crate) conversion_fallbacks: Vec<vibe_workspace::install::ConversionFallback>` to InstallDraft. from_run copies run.conversion_fallbacks. failed() and the test literal near line 284 use Vec::new().
4. draft.rs into_report: replace the `conversion_fallbacks: Vec::new(),` that ADV-FIX-CONVFB-3 put in the InstallReport literal with the rows mapped into the generated InstallConversionFallback { package, version, file, kept_format, reason }, with kept_format SpecFormat::Xml => InstallConversionKeptFormat::Xml and Markdown | Mixed => InstallConversionKeptFormat::Markdown.
5. draft.rs render: at the top, collect `let warnings: Vec<String> = self.conversion_fallbacks.iter().map(ToString::to_string).collect();`. After render_weak_lines(ctx, &report), and before report::render_human_progress, print each line with `ctx.block_err(&crate::output::language::warning(line))`.
6. Write the test file and run the check. Then run `cargo xtask specmap` and commit specmap.json.

**Decisions already made.**
- The orchestrator carries rows on InstallRun.conversion_fallbacks, read from ApplyReport.outcome. InstallProgress is NOT extended, because it has about 14 literal sites across vibe-cli and vibe-orchestrator.
- Human line: the house warning glyph via output::language::warning plus the ConversionFallback Display text, on stderr through ctx.block_err. It is shown in human mode, not in quiet or JSON mode, and printed once per row.
- JSON member conversion_fallbacks on the final `install` document only
- The orchestrator part is two one-line additions and has no observable behaviour of its own, so it rides in this entry and is checked by the CLI end-to-end test
- The test file name avoids the substring "install" in the binary name, after the Windows UAC note in crates/vibe-install/Cargo.toml

**Not in this entry.** The --strict-conversion flag, and rendering in vibe update / vibe reinstall drafts or their schemas.

**Depends on.** ADV-FIX-CONVFB-1, ADV-FIX-CONVFB-3

Entries that write the same file run one after another, never in parallel.
