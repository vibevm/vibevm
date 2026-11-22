# SAT-5

Planned at commit `57b1e5a37`. Line numbers are hints; find each place by the quoted text or the item name.

**Outcome.** vibe prompt types --check exits non-zero for stale, missing or foreign type files

**Write.** `crates/vibe-cli/src/commands/prompt/types.rs`, `crates/vibe-cli/tests/cli_prompt_types.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_prompt_types prompt_types_check_reports_a_stale_type`

**Test.** crates/vibe-cli/tests/cli_prompt_types.rs::prompt_types_check_reports_a_stale_type

**Read first.**
- crates/vibe-cli/src/commands/prompt/types.rs
- crates/vibe-cli/src/commands/prompt/check.rs (run: report emitted before bail)
- crates/vibe-cli/tests/cli_prompt_types.rs

**Steps.**
1. First fast-forward this worktree to the lead's branch named in the packet.
2. In types::run, after printing the report in check mode, apply the exit rule in decisions.
3. Test prompt_types_check_reports_a_stale_type:
- Reuse the `project` helper.
- Run `["prompt","types","--path",P]`, then `["prompt","types","--check","--path",P]`. The check succeeds, and stdout has a line starting with "current" and containing ".claude/agents/vibe-worker.md".
- Append "edited\n" to P/.claude/agents/vibe-worker.md.
- Run `["prompt","types","--check","--path",P,"--json"]`. It fails (non-zero). Its stdout JSON shows mode "check" and vibe-worker status "stale", and stderr contains "vibe prompt types".
- The file still ends with "edited\n" (check wrote nothing).
4. Run the check; commit.

**Decisions already made.**
- Check-mode exit: collect the outcomes with status Stale, Missing or Foreign. If there are any, bail!("subagent types are not current: {list}; run `vibe prompt types`"), where list is `{file} ({status})` joined by ", ". Current and Unavailable alone exit 0.
- The JSON report (or the human lines) is emitted before the error, as commands/prompt/check.rs::run does.

**Not in this entry.** Not in this entry: a `vibe check` rule, launch behaviour.

**Depends on.** SAT-4

**Lead side (not the executor's).**

After applying: regenerate specmap.json on main.

Entries that write the same file run one after another, never in parallel.
