# ADV-SDD-CHANGE-READ-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe change list`, `vibe change show <ID>` and `vibe change check [ID...]` print the change documents (exactly the library JSON under `--json`), and `check` exits 1 when a change has a stale or missing origin capture.

**Write.** `crates/vibe-cli/src/commands/change.rs`, `crates/vibe-cli/src/commands/mod.rs`, `crates/vibe-cli/src/cli/command.rs`, `crates/vibe-cli/src/main.rs`, `crates/vibe-cli/src/command_progress.rs`, `crates/vibe-cli/tests/cli_change.rs`, `vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml`

**Check.** `cargo test -p vibe-cli --test cli_change 2>&1 | grep -F "test result: ok. 3 passed"`

**Test.** crates/vibe-cli/tests/cli_change.rs (new test target, auto-discovered): `change_show_json_is_the_library_report`, `change_check_exits_one_on_stale_origin_and_zero_when_unavailable`, `change_commands_leave_the_project_unchanged`.

**Read first.**
- crates/vibe-cli/src/commands/requirements.rs (run and emit: is_json / is_quiet / ctx.summary; imports `anyhow::Context as _` beside `crate::output::Context`)
- crates/vibe-cli/src/commands/facts.rs (FactsArgs: clap Args and Subcommand defined inside the commands module)
- crates/vibe-cli/src/main.rs (the `Command::Remap` arm: the `Ok(Some(code)) => return ExitCode::from(...)` pattern)
- crates/vibe-cli/src/command_progress.rs (TOP_LEVEL_COMMANDS, the `Command::Requirements(_) => FiniteFallback(...)` arm)
- crates/vibe-cli/tests/cli_requirements.rs (UserScratch::vibe, fn snapshot to copy)
- crates/vibe-requirements/src/change/mod.rs (from entries 1 and 2: change_report, list_changes, check_changes)

**Steps.**
1. Create crates/vibe-cli/src/commands/change.rs with first item `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-COMMANDS");`. Define `#[derive(Debug, clap::Args)] pub struct ChangeArgs { #[command(subcommand)] pub command: ChangeCommand }` and `#[derive(Debug, clap::Subcommand)] pub enum ChangeCommand { List(ChangeListArgs), Show(ChangeShowArgs), Check(ChangeCheckArgs) }`. Variant doc lines: List `/// List the authored changes under the selected node's spec root.`, Show `/// Show one authored change: digests, documents, work items and origin drift.`, Check `/// Check authored changes against the installed project; exit 1 on a refusal or a stale or missing origin capture.`. Argument structs, each `#[derive(Debug, clap::Args)]`: `ChangeListArgs { #[arg(long, default_value = ".")] pub path: PathBuf }`, `ChangeShowArgs { pub id: String, #[arg(long, default_value = ".")] pub path: PathBuf }`, `ChangeCheckArgs { pub ids: Vec<String>, #[arg(long, default_value = ".")] pub path: PathBuf }`.
2. In change.rs add `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-COMMANDS")] pub fn run(ctx: &crate::output::Context, args: ChangeArgs) -> anyhow::Result<Option<u8>>`. Import `anyhow::Context as _` (the name `Context` is taken by crate::output::Context). Resolve `vibe_workspace::Workspace::discover_selected(&path).with_context(|| format!("resolving the selected node at `{}`", path.display()))?`. Call the library: `list_changes`, `change_report` or `check_changes`. Under `ctx.is_json()`, print `println!("{}", serde_json::to_string_pretty(&doc)?)`. Otherwise build the text below as one String; under `ctx.is_quiet()` call `ctx.summary(text.lines().last().unwrap_or_default())`, else `print!("{text}")`. Return `Ok(Some(crate::exit_code::GENERAL))` when a check document has `passed == false`, otherwise `Ok(None)`. Library errors become `anyhow::Error` via `?`, so `show` of an unreadable change exits 1 through the ordinary error path.
3. Text format, one item per line ending in `\n`. list: `<id> <title>`, or `<id> (invalid)`; an empty list prints `no authored changes`. show: `change <id>: <title>`, `  slug: <slug>`, `  node: <selected_node>`, `  intent: <intent_digest>`, `  work: <work_digest>`; then per document `  document <role> <path> <present|absent>`; per work item `  item <id> after <comma-joined predecessors, or -> needs <comma-joined kind:target, or ->`; per origin `  origin <package> locked <version, or -> manifest <status>`, followed by `    <address> <status>` per document. A status word is a private `fn status_word(s: CaptureStatus) -> &'static str` match giving `matched`, `stale`, `missing`, `unavailable`. check: per change `<id> passed` or `<id> failed`, then `  <problem>` per problem, and a last line `<n> passed, <m> failed`.
4. In crates/vibe-cli/src/commands/mod.rs add `pub mod change;` between `pub mod cache;` and `pub mod check;`. In crates/vibe-cli/src/cli/command.rs add directly after `Requirements(RequirementsArgs),`: ```
    /// Read authored changes (PROP-071): `list`, `show <ID>` and
    /// `check [ID...]` report digests, documents, work items and whether
    /// each origin capture still matches the installed bytes. Read-only;
    /// `--json` prints the report document exactly.
    Change(crate::commands::change::ChangeArgs),
``` In main.rs add directly after the `Command::Requirements(args) => commands::requirements::run(&ctx, args),` arm: ```
            Command::Change(args) => match commands::change::run(&ctx, args) {
                Ok(Some(code)) => {
                    return ExitCode::from(code);
                }
                Ok(None) => Ok(()),
                Err(error) => Err(error),
            },
``` In command_progress.rs insert `"change",` between `"cache",` and `"check",` in TOP_LEVEL_COMMANDS, and add `Command::Change(_) => FiniteFallback("Command change"),` directly after the `Command::Requirements(_) => FiniteFallback("Command requirements"),` arm.
5. Write crates/vibe-cli/tests/cli_change.rs. It rebuilds entry 2's drift fixture with plain std::fs (it cannot import cfg(test) code from vibe-requirements): the same `ID`, `TEXT` (read with `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../formats/corpora/change-manifest/e1/valid/representative.toml"))`), `WORK`, `SLOT_MANIFEST`, `FEAT`, vibe.lock text, slot (`vibe_workspace::vibedeps::slot_abs_path` with `vibe_core::Group::parse("org.fixture")`), change directory under `vibe_core::layout::current_specs_root()` and change.toml with digests from `vibe_workspace::vibedeps::sha256_bytes` (vibe-workspace and vibe-core are normal dependencies of vibe-cli, so the test target can use them; Cargo.toml does not change). Copy the `snapshot` walker from cli_requirements.rs into this file. Run the binary as `vibe_test_support::UserScratch::new().vibe().args(["--json", "change", "show", ID]).arg("--path").arg(root.path())` (and the matching shapes). Test 1 `change_show_json_is_the_library_report`: exit success, stdout parses as one JSON value equal to `serde_json::to_value(vibe_requirements::change::change_report(&vibe_workspace::Workspace::discover_selected(root.path()).unwrap(), ID).unwrap()).unwrap()`. Test 2 `change_check_exits_one_on_stale_origin_and_zero_when_unavailable`: keep the lock bytes, delete vibe.lock and run `--json change check`: exit code 0 and `passed == true`; write the kept lock bytes back, append one byte to the slot FEAT-001.md and run `--json change check`: exit code 1, JSON `passed == false`, and `changes[0].problems[0]` equals `origin feat:org.fixture/reusable-intent@=1.0.0 document spec://org.fixture/reusable-intent/FEAT-001#DELIVERY is stale`. Test 3 `change_commands_leave_the_project_unchanged`: take `snapshot(root.path())`, run list, show ID and check with and without `--json`, and assert the snapshot is byte-identical afterwards.
6. Insert this line in PROP-071 directly before the closing `</spec>` line (after entry 2's line): ```
  <p><CHANGE-COMMANDS fact="true" status="impl/done">vibe change list, vibe change show ID and vibe change check [ID...] read the selected node named by --path, default the current directory, with the incumbent default spec root. With the global --json flag each prints exactly the vibe-change-list/1, vibe-change-report/1 or vibe-change-check/1 document and nothing else. show exits 1 when the change cannot be read; check exits 1 when its report has not passed and 0 otherwise. No command writes to the project.</CHANGE-COMMANDS></p>
```

**Decisions already made.**
- The args live in commands/change.rs, as `crate::commands::facts::FactsArgs` does. The cli.rs hub is not touched.
- Exit policy: list and show exit 0 on success. check exits 1 (exit_code::GENERAL) only when its document has `passed == false`; unavailable captures exit 0 (owner question 2). Existing exit codes are unchanged. The early `return ExitCode::from(code)` mirrors the Remap arm.
- The `--json` output is the bare document with no envelope (the shape `vibe requirements --json` uses).
- Progress policy: FiniteFallback("Command change"), with no new progress task.
- The existing lib test `top_level_policy_inventory_matches_clap` (command_progress/tests.rs) must keep passing; inserting "change" in sorted position does that. It is not part of the check.
- File budgets: command_progress.rs is 578 lines today (+2), main.rs 469 (+7), cli/command.rs 361 (+5), commands/mod.rs 99 (+1). commands/change.rs must stay under 600 lines.
- No `.unwrap()` or `.expect()` outside tests (vibe-cli is in conform `gated`); `unwrap_or_default` is allowed.
- Entry 2 runs first; this entry edits PROP-071 after it.

**Not in this entry.** An MCP tool (entry 4), a `--spec-root` flag, `vibe change new/ask/answer`, quiet or progress redesign, and docs-package command pages.

**Depends on.** ADV-SDD-CHANGE-READ-2

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml: add unit CHANGE-COMMANDS (status impl/done), exact text in the last step.

Entries that write the same file run one after another, never in parallel.
