# ADV-FIX-CONVFB-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe install --strict-conversion` of a package with an unparseable opposite-format spec exits nonzero with an error naming the file and --strict-conversion, and leaves vibe.toml, vibe.lock, vibedeps and the boot lane byte-identical.

**Write.** `crates/vibe-orchestrator/src/install/args.rs`, `crates/vibe-orchestrator/src/install/mod.rs`, `crates/vibe-cli/src/cli/pkg.rs`, `crates/vibe-cli/src/cli/lifecycle.rs`, `crates/vibe-cli/src/commands/reinstall/inputs.rs`, `crates/vibe-cli/src/commands/cache/add.rs`, `crates/vibe-cli/src/commands/remap/packages.rs`, `crates/vibe-cli/src/commands/install/flag_tests.rs`, `crates/vibe-cli/src/commands/install/global_mcp.rs`, `crates/vibe-cli/src/commands/install/direct/tests.rs`, `crates/vibe-cli/src/commands/update/inputs.rs`, `crates/vibe-cli/tests/cli_conversion_fallback.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_conversion_fallback strict_conversion`

**Test.** crates/vibe-cli/tests/cli_conversion_fallback.rs, new #[test] fn strict_conversion_refuses_and_leaves_the_project_untouched. Same org.example/legacy fixture and spec_format = "xml" project as the file's other tests. It takes a snapshot of every project file except .vibe/ (copy snapshot and describe_difference from cli_install_rollback.rs), then runs `vibe install org.example/legacy --strict-conversion --assume-yes --path <p>`. It asserts: the exit status is not success; stdout+stderr contains "PROP-001.md" and "--strict-conversion"; the snapshot after equals the snapshot before.

**Read first.**
- crates/vibe-orchestrator/src/install/args.rs:25-80
- crates/vibe-orchestrator/src/install/mod.rs:515-560
- crates/vibe-cli/src/cli/pkg.rs:40-90
- crates/vibe-cli/src/commands/update/inputs.rs:15-70
- crates/vibe-cli/tests/cli_install_rollback.rs:40-80

**Steps.**
1. vibe-orchestrator args.rs: add `pub strict_conversion: bool` to InstallInputs, with the doc comment "`--strict-conversion`: refuse the install when a spec document could not be converted to the effective spec format". InstallInputs derives Default, and pkg.rs is its only literal.
2. vibe-orchestrator install/mod.rs: change the `Plan::Ready(planned) =>` arm to `Plan::Ready(mut planned) =>` and, before ready::apply, add `if args.strict_conversion { planned.refuse_conversion_fallbacks(); }`.
3. vibe-cli cli/pkg.rs: add to InstallArgs `/// Refuse the install when a spec document could not be converted to the project's spec_format, instead of keeping it in its source format.` `#[arg(long)] pub strict_conversion: bool,` and copy it in inputs(): `strict_conversion: self.strict_conversion,`.
4. Add `strict_conversion: false,` to each InstallArgs literal that has no `..` spread: cli/lifecycle.rs install_args, reinstall/inputs.rs resolver_args, cache/add.rs stub_install_args, remap/packages.rs inert_args, install/flag_tests.rs base_args, install/global_mcp.rs (literal near line 105), install/direct/tests.rs args, update/inputs.rs install_args_from. Use `cargo check -p vibe-cli --tests` to confirm that none is missed.
5. Write the test and run the check
6. Run `cargo xtask specmap` and commit specmap.json

**Decisions already made.**
- Flag spelling: `--strict-conversion` on `vibe install`. It is a CLI flag only: no vibe.toml key and no user-config key in this release.
- The path is InstallArgs.strict_conversion, then InstallInputs.strict_conversion, then PlannedInstall::refuse_conversion_fallbacks() in the orchestrator's Ready arm. InstallPolicy is NOT extended (about 15 literal sites across vibe-mcp, vibe-orchestrator and vibe-cli).
- update/inputs.rs passes false. Exposing the flag on `vibe update` is a later list.
- The refusal text comes from ADV-FIX-CONVFB-2 (WorkspaceError::SpecMaterialization naming the file and --strict-conversion), so there is no new CLI error class
- A rerun over an already-current slot does not repeat or enforce old rows (no materialisation, so no rows)

**Not in this entry.** Adding --strict-conversion to UpdateArgs or ReinstallArgs, or a manifest/user-config strictness key.

**Depends on.** ADV-FIX-CONVFB-2, ADV-FIX-CONVFB-4

Entries that write the same file run one after another, never in parallel.
