# ADV-SDD-RENAME-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** After a non-dry-run rename that wrote files, the command runs `vibe install --assume-yes` (plus `--offline` when the root `--offline` flag was given) in the workspace root. vibe.lock then names org.new/bar and vibevm/vibedeps holds org.new.bar instead of org.old.foo, and the JSON reports `install: {ran: true, exit_code: 0, args: [...]}` with the exact child arguments. `--no-install` skips this step and prints the command to run.

**Write.** `crates/vibe-cli/src/main.rs`, `crates/vibe-cli/src/commands/refactor/mod.rs`, `crates/vibe-cli/src/commands/refactor/rename_package.rs`, `crates/vibe-cli/src/commands/refactor/rename_package/regenerate.rs`, `crates/vibe-cli/tests/cli_refactor_rename_package.rs`

**Check.** `cargo test -p vibe-cli --test cli_refactor_rename_package 2>&1 | grep -F "test result: ok. 5 passed"`

**Test.** Add `install_regenerates_lock_after_rename` to crates/vibe-cli/tests/cli_refactor_rename_package.rs. The file has 4 tests before this entry, so "5 passed" fails until it lands. Change the import to `use vibe_test_support::{UserScratch, vibe};`. Body: `let user = UserScratch::new();` then write_fixture(root, false). Run `user.vibe().current_dir(root).args(["--offline", "install", "--assume-yes"]).output()` and assert success and that root/vibevm/vibedeps/org.old.foo is a directory. Then run `user.vibe().current_dir(root).args(["--offline", "--json", "refactor", "rename-package", "org.old/foo", "org.new/bar"]).output()`. Assert success and that JSON install == json!({"ran": true, "exit_code": 0, "args": ["install", "--assume-yes", "--offline"]}) (this asserts the root --offline flag reached the child). Assert that root/vibe.lock contains `group = "org.new"` and `name = "bar"` and does not contain `name = "foo"`. Assert that root/vibevm/vibedeps/org.new.bar is a directory and root/vibevm/vibedeps/org.old.foo does not exist. Also add `"--no-install"` to every non-dry-run rename invocation in `manifest_apply_rewrites_and_rerun_is_noop` and `spec_uris_are_rewritten_and_generated_files_reported`, so those tests never spawn install.

**Read first.**
- crates/vibe-cli/src/commands/refactor/rename_package.rs (RenamePackageArgs, run, report struct)
- crates/vibe-cli/src/commands/refactor/mod.rs (`pub fn run(ctx: &output::Context, args: RefactorArgs) -> Result<()>` and its RenamePackage arm)
- crates/vibe-cli/src/main.rs (the `Command::Refactor(args) => commands::refactor::run(&ctx, args),` dispatch arm and how other arms pass `cli.offline`)
- crates/vibe-cli/src/cli.rs (`pub offline: bool`, declared `#[arg(long, global = true)]`)
- crates/vibe-cli/tests/cli_refactor_rename_package.rs (existing tests to extend with --no-install)
- crates/vibe-test-support/src/lib.rs (UserScratch::new, UserScratch::vibe)

**Steps.**
1. Create rename_package/regenerate.rs with `#[derive(Debug, serde::Serialize)] pub(super) struct InstallOutcome { pub(super) ran: bool, pub(super) exit_code: Option<i32>, pub(super) args: Vec<String> }`, `impl InstallOutcome { pub(super) fn skipped() -> Self { Self { ran: false, exit_code: None, args: Vec::new() } } }`, and `pub(super) fn run_install(root: &std::path::Path, offline: bool, capture: bool) -> anyhow::Result<InstallOutcome>`. It builds the argument list `["install", "--assume-yes"]` and appends `"--offline"` when offline is true. It runs `std::process::Command::new(std::env::current_exe().context("locating the running vibe executable")?)` with those args and `current_dir(root)`, inheriting the environment (VIBE_OFFLINE, settings and registry variables pass through). When capture is true it uses `.output()` and discards the bytes, so JSON stdout stays a single document. Otherwise it uses `.status()` with inherited stdio. It returns `ran: true, exit_code: status.code(), args: <the list as Vec<String>>`.
2. Thread the root flag through. In crates/vibe-cli/src/commands/refactor/mod.rs change the signature to exactly `pub fn run(ctx: &output::Context, args: RefactorArgs, offline: bool) -> Result<()>` (keep its existing #[spec] attribute) and the arm to `RefactorCommand::RenamePackage(args) => rename_package::run(ctx, args, offline),`; the other arms ignore `offline`. In crates/vibe-cli/src/main.rs change the dispatch arm to `Command::Refactor(args) => commands::refactor::run(&ctx, args, cli.offline),`. This is the only caller of refactor::run (machine_entry is a separate function and stays unchanged). Change rename_package::run to `pub(super) fn run(ctx: &crate::output::Context, args: RenamePackageArgs, offline: bool) -> anyhow::Result<()>`.
3. In rename_package.rs add `mod regenerate;` and the flag `#[arg(long)] pub(crate) no_install: bool` (doc: Skip running `vibe install` after rewriting). Add the field `install: regenerate::InstallOutcome` to the JSON report. After a successful apply that wrote at least one file and `!args.no_install`, call `regenerate::run_install(&plan.root, offline, ctx.is_json())?`. In every other case (dry-run, no-op, --no-install) use `InstallOutcome::skipped()`. Set report `ok` to false when ran is true and exit_code != Some(0).
4. Human output, after the summary line: `install exit={code} args={args joined by a space}` when ran (`exit=signal` when the code is None). When files were written and --no-install was given, print `install skipped; run `vibe install` to regenerate vibe.lock and vibedeps`. Emit the report (JSON or human) first. Then, when ok is false, bail `rename-package: files were rewritten, but `vibe install` exited with {code}; fix the cause and run `vibe install``.
5. Update the two existing apply tests with `--no-install` and add the new test. Run `cargo fmt --all` and `cargo clippy -p vibe-cli --all-targets -- -D warnings`. `offline` is used by the RenamePackage arm, so no unused-parameter warning arises in refactor::run. Check with `wc -l` that rename_package.rs, refactor/mod.rs (344 lines today) and main.rs (469 lines today) stay under 600.

**Decisions already made.**
- Regeneration runs as a child process of the same binary (`current_exe() install --assume-yes`, cwd = workspace root) rather than calling the install dispatch in-process. The install dispatch needs main.rs-owned state (network route, stop input, embedded root).
- The root `--offline` flag IS forwarded: main.rs passes `cli.offline` into refactor::run, and the child gets `--offline` appended. VIBE_OFFLINE and the user-config posture reach the child through the inherited environment. This keeps the cli.rs promise that --offline never silently degrades to a network run.
- JSON `install` is always present as {ran, exit_code, args}; `args` is the exact child argument list (empty when skipped), so the forwarded posture is observable.
- Install runs only after a non-dry-run apply that wrote at least one file.
- A failed install leaves the rewritten files in place, reports ok=false and exits 1 with the named message. Nothing is rolled back.
- The lockfile and vibedeps change only through this install run, never by direct edits.
- Entries 1, 2 and 3 share rename_package.rs and the test file, so this entry runs after ADV-SDD-RENAME-2.

**Not in this entry.** Not in this entry: calling install in-process from main.rs, forwarding global flags other than --offline (e.g. --refresh, --verbose) to the child, rolling back the rewrite when install fails, running `vibe check` afterwards, and changing exit_code.rs.

**Depends on.** ADV-SDD-RENAME-2

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
