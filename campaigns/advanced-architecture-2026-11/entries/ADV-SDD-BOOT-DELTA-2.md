# ADV-SDD-BOOT-DELTA-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe install` accepts `--boot-delta` together with `--plan-only`, refuses it without `--plan-only` (clap MissingRequiredArgument, exit 2), and refuses it with `-g` (clap ArgumentConflict).

**Write.** `crates/vibe-cli/src/cli/pkg.rs`, `crates/vibe-cli/src/cli/tests.rs`, `crates/vibe-cli/src/cli/lifecycle.rs`, `crates/vibe-cli/src/commands/reinstall/inputs.rs`, `crates/vibe-cli/src/commands/cache/add.rs`, `crates/vibe-cli/src/commands/remap/packages.rs`, `crates/vibe-cli/src/commands/install/flag_tests.rs`, `crates/vibe-cli/src/commands/install/direct/tests.rs`, `crates/vibe-cli/src/commands/install/global_mcp.rs`, `crates/vibe-cli/src/commands/update/inputs.rs`

**Check.** `cargo test -p vibe-cli --bin vibe cli::tests::boot_delta_requires_plan_only_and_refuses_global 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/src/cli/tests.rs :: boot_delta_requires_plan_only_and_refuses_global (new)

**Read first.**
- crates/vibe-cli/src/cli/pkg.rs struct InstallArgs (the field `plan_only` and its doc comment)
- crates/vibe-cli/src/cli/tests.rs (the existing Cli::try_parse_from tests and the `let Command::Install(args) = cli.command else` pattern)
- crates/vibe-cli/src/cli/command.rs enum Command variant `Install(Box<InstallArgs>)`

**Steps.**
1. In crates/vibe-cli/src/cli/pkg.rs struct InstallArgs, directly after the field `pub plan_only: bool,`, add the field given in decisions. (pkg.rs is 539 lines; it stays under 600.)
2. Every full InstallArgs struct literal must set the new field. In each of these 8 files, directly after the single line `plan_only: false,`, add `boot_delta: false,` with the same indentation: crates/vibe-cli/src/cli/lifecycle.rs (fn install_args), crates/vibe-cli/src/commands/reinstall/inputs.rs (fn resolver_args), crates/vibe-cli/src/commands/cache/add.rs (fn stub_install_args), crates/vibe-cli/src/commands/remap/packages.rs (fn inert_args), crates/vibe-cli/src/commands/install/flag_tests.rs (fn base_args), crates/vibe-cli/src/commands/install/direct/tests.rs (fn args), crates/vibe-cli/src/commands/install/global_mcp.rs (the `let install_args = InstallArgs {` literal), crates/vibe-cli/src/commands/update/inputs.rs (fn install_args_from). `grep -rn "plan_only: false," crates --include=*.rs` lists exactly these 8 sites. Literals that end in `..inert_args()` need no change.
3. Append the test given in decisions to crates/vibe-cli/src/cli/tests.rs. Do not add a #[verifies] attribute to it.
4. Run `cargo fmt --all`. rustfmt will reflow the long `Cli::try_parse_from([...])` lines of the test; that is expected.
5. Run `cargo clippy -p vibe-cli --all-targets -- -D warnings` and fix only warnings in this entry's files.
6. Run the check.

**Decisions already made.**
- Field (exact):
```
    /// With `--plan-only`: also install the candidate closure into a
    /// throwaway copy of the project and print how the size of each watched
    /// boot lane would change. Package content may be downloaded into the
    /// ordinary package cache; the project itself is not changed.
    #[arg(long, requires = "plan_only", conflicts_with = "global")]
    pub boot_delta: bool,
```
- Test (before rustfmt):
```
#[test]
fn boot_delta_requires_plan_only_and_refuses_global() {
    let cli = Cli::try_parse_from(["vibe", "install", "org.example/b", "--plan-only", "--boot-delta"])
        .unwrap_or_else(|error| panic!("parse: {error}"));
    let Command::Install(args) = cli.command else {
        panic!("expected install");
    };
    assert!(args.boot_delta);
    let missing = Cli::try_parse_from(["vibe", "install", "org.example/b", "--boot-delta"])
        .expect_err("--boot-delta needs --plan-only");
    assert_eq!(missing.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    let global = Cli::try_parse_from(["vibe", "install", "-g", "org.example/b", "--plan-only", "--boot-delta"])
        .expect_err("--boot-delta is refused with -g");
    assert_eq!(global.kind(), clap::error::ErrorKind::ArgumentConflict);
}
```
- Use `.expect_err(..)`, never `.err().expect(..)`: the latter fails clippy::err_expect under -D warnings. Cli derives Debug, so expect_err compiles.
- No #[verifies] on this test: PROP-050#verification is the lock closure-diff unit, and the PROP-073#BOOT-DELTA-PREVIEW anchor does not exist until ADV-SDD-BOOT-DELTA-3, whose integration test carries the edge.
- --user-project already requires -g, so the conflict with global covers it. No other flag gets a rule.
- The field is read by nothing in this entry. That is not a clippy warning, because the struct field is public.

**Not in this entry.** Not in this entry: any behaviour behind the flag (the preview stays unchanged), a --boot-delta flag on update or uninstall, or reordering any other InstallArgs field.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
