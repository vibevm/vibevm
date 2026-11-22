# ADV-CONTROL-CAPS-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe capabilities --json` prints a `vibe-capabilities/1` document with version, platform, every registered format and epoch, an empty `commit` string, and a command tree equal to the vibe-doc help walk of the same binary. It writes nothing into the home directory.

**Write.** `crates/vibe-cli/src/cli/command.rs`, `crates/vibe-cli/src/main.rs`, `crates/vibe-cli/src/command_progress.rs`, `crates/vibe-cli/src/commands/mod.rs`, `crates/vibe-cli/src/commands/capabilities.rs`, `crates/vibe-cli/tests/cli_capabilities.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_capabilities`

**Test.** crates/vibe-cli/tests/cli_capabilities.rs: (1) `capabilities_json_reads_as_its_generated_type`: runs `env!("CARGO_BIN_EXE_vibe") capabilities --json` with cwd set to an empty tempdir and HOME and USERPROFILE set to an empty `<tempdir>/home`. It removes VIBE_INVOKED_BY, VIBE_UNATTENDED, VIBEVM_HOME, VIBE_SETTINGS and VIBE_PROGRESS_FORMAT from the environment. It asserts exit 0, that stdout parses into `vibe_wire::generated::capabilities_report::CapabilitiesReport`, `protocol == "vibe-capabilities/1"`, `ok == true`, `version == env!("CARGO_PKG_VERSION")`, `platform.os == std::env::consts::OS`, `platform.arch == std::env::consts::ARCH`, `formats.len() == FormatId::ALL.len()`, and that formats contain `{id: "cli-capabilities", epoch: 1}`. It also asserts that re-serialising the typed report equals the stdout parsed as `serde_json::Value`, and that `<tempdir>/home` has zero entries after the run. (2) `capabilities_commands_equal_the_doc_surface_walk`: runs the same command and computes `vibe_doc::surface::commands::tree(&SurfaceEnv { binary: CARGO_BIN_EXE_vibe, repo_root: CARGO_MANIFEST_DIR/../.., timeout_secs: 60 })`. It asserts that `report.commands` projected to `Vec<(path, Vec<(flag name, flag value)>)>` equals the vibe-doc result projected the same way. On failure the assert message names the first differing path. Today the subcommand is unknown (exit 2) and the test target does not exist.

**Read first.**
- crates/vibe-cli/src/main.rs:52-80
- crates/vibe-cli/src/main.rs:440-470
- crates/vibe-cli/src/cli/command.rs:340-361
- crates/vibe-cli/src/command_progress.rs:15-75
- crates/vibe-cli/src/command_progress.rs:470-485
- crates/vibe-cli/src/output.rs:285-345

**Steps.**
1. crates/vibe-cli/src/cli/command.rs: add a unit variant `Capabilities` directly before `Version`, with the doc comment given in the decisions.
2. crates/vibe-cli/src/command_progress.rs: insert "capabilities" into TOP_LEVEL_COMMANDS between "cache" and "check", and add `Command::Capabilities => SilentMetadata,` next to `Command::Version => SilentMetadata,`.
3. Create crates/vibe-cli/src/commands/capabilities.rs and add `pub mod capabilities;` to crates/vibe-cli/src/commands/mod.rs in alphabetical position. The module starts with `specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");` and exposes `pub fn run(ctx: &crate::output::Context) -> anyhow::Result<()>` plus `pub(crate) fn report() -> anyhow::Result<CapabilitiesReport>`. `report()` fills the fields as the decisions say, and `run` prints it.
4. crates/vibe-cli/src/main.rs: in `run()`, directly after the `if let Command::Kit(args) = cli.command { ... }` block and before `select_installed_destination`, add `if matches!(cli.command, Command::Capabilities) { ... }`. Inside it copy the Kit block's `configure_language` call and Context construction, call `commands::capabilities::run(&ctx)`, and map Ok to ExitCode::SUCCESS and Err to `ctx.error_with_suffix(&error, None); as_exit_code(&error)`. In the main dispatch match also add `Command::Capabilities => commands::capabilities::run(&ctx),` next to `Command::Version` so the match stays exhaustive.
5. Create crates/vibe-cli/tests/cli_capabilities.rs with the two tests. Use std::process::Command and tempfile directly; do not use tests/common.
6. Run the check. If test (2) still differs, stop and report the first differing command path with both flag lists. Do not change vibe-doc or the help parser to make it pass.

**Decisions already made.**
- Doc comment for the variant, as plain Rust text (plain backticks, no backslashes): /// Print what this vibe supports — version, commit, platform, every command with its flags, every report format with its epoch and the named features — as one `vibe-capabilities/1` document under `--json`. Needs no project, no network and no home.
- The report fields: `protocol: "vibe-capabilities/1".into()`, `ok: true`, `version: env!("CARGO_PKG_VERSION")`, `commit: option_env!("VIBE_BUILD_COMMIT").unwrap_or("")`, `platform: CapabilityPlatform { os: std::env::consts::OS, arch: std::env::consts::ARCH }`, `formats: FormatId::ALL.iter().map(|f| CapabilityFormat { id: f.id().into(), epoch: f.epoch() })` in ALL order, `features: Vec::new()` (filled by ADV-CONTROL-CAPS-4).
- Nothing sets VIBE_BUILD_COMMIT today: crates/vibe-cli/build.rs only sets VIBE_MSRV, and no xtask or `self install` path sets it. So `commit` is "" in every build this entry produces, and that is the expected result. The tests do not assert on `commit`.
- Commands: `let root = crate::cli::Cli::command();` (clap::CommandFactory). Call `vibe_doc::surface::commands::tree_from("vibe", &mut |path| { ... })`. The closure builds `argv = once("vibe".to_string()).chain(path.iter().cloned()).chain(once("--help".to_string()))` and calls `root.clone().try_get_matches_from(argv)`. When the result is `Err(e)` with `e.kind() == clap::error::ErrorKind::DisplayHelp`, it returns `Ok(e.render().to_string())`. Any other result returns `Err(format!("no help for {path:?}"))`. This is the same text a `vibe ... --help` child process prints: clap is built without `wrap_help`, so the width is fixed, and StyledStr's Display strips styling. Do not use `render_long_help`, because it always prints the long layout.
- Map each SurfaceCommand to `CapabilityCommand { path, flags: flags.into_iter().map(|f| CapabilityFlag { name: f.name, value: f.value }).collect() }`. The summary is dropped. A tree_from error becomes an anyhow error with its text.
- JSON output: `ctx.emit_json(&report)` when `ctx.is_json()`.
- Human output when not JSON: print exactly five lines: `vibe {version} ({os} {arch})`, `commit: {commit, or "unknown" when empty}`, `commands: {count}`, `formats: {count}`, `features: {comma+space joined, or "none" when empty}`.
- Old vibe answers exit 2 for the unknown subcommand. Nothing to build for that.

**Not in this entry.** Not in this entry: a producer for VIBE_BUILD_COMMIT (no build.rs change, no `git rev-parse`, no xtask or install change), the features table, an `error_codes` field, a `vibe self capabilities` alias, a JSON schema for the human output, teaching `vibe --version` to print JSON, or any change to vibe-doc.

**Depends on.** ADV-CONTROL-CAPS-1, ADV-CONTROL-CAPS-2

Entries that write the same file run one after another, never in parallel.
