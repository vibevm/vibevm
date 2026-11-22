# SWP-4-LAUNCH

Planned at commit `9b197755e`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe agent claude --worker-prefix -p ...` passes the subagent flag naming the parent's lane; refusals for no print mode, other kinds and a duplicate user flag

**Write.** `crates/vibe-cli/src/commands/agent/launch.rs`, `crates/vibe-cli/src/commands/agent/launch/worker_prefix.rs`, `crates/vibe-cli/src/cli/agent.rs`, `crates/vibe-cli/tests/cli_agent_worker_prefix.rs`, `vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_agent_worker_prefix && test "$(grep -c '<SYS-WORKER-PREFIX-DELIVERY fact="true" status="impl/done">' vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml)" = 1`

**Test.** New file crates/vibe-cli/tests/cli_agent_worker_prefix.rs (cli_agent_launch.rs is already 1481 lines, so the tests go in their own file). Tests: worker_prefix_appends_the_subagent_flag_naming_the_parent_lane, without_worker_prefix_the_argv_is_unchanged, worker_prefix_without_print_mode_is_refused, worker_prefix_for_another_kind_is_refused, worker_prefix_refuses_the_users_own_subagent_flag, worker_prefix_is_dropped_when_the_project_has_not_opted_in. Before the work the test target does not exist, so the check fails.

**Read first.**
- crates/vibe-cli/src/commands/agent/launch.rs (run: `let rest`, `native_arguments`, admit_route_kind call, `let composed = composed.flags;`, `command.extend(composed.flags.iter().cloned());`; the `#[path = "launch/route.rs"] mod route;` lines)
- crates/vibe-cli/src/commands/agent/launch/route.rs (child-module header to imitate)
- crates/vibe-cli/src/cli/agent.rs (AgentCommand::Launch doc comment)
- crates/vibe-cli/tests/cli_agent_launch.rs (Machine::new, Machine::shim, Machine::harnessed, Machine::argv, Machine::lane, a_launch_execs_the_argv_prefix_then_the_flags_then_the_users_own_arguments)
- crates/vibe-cli/src/commands/prompt/flags.rs (Composed.lane, added by SWP-3-FLAGS)

**Steps.**
1. Fast-forward the worktree to the lead's branch first.
2. launch.rs is 529 lines (limit 600): new logic goes in a new child module crates/vibe-cli/src/commands/agent/launch/worker_prefix.rs (module doc, `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-PREFIX-DELIVERY");`) with two functions:
- `pub(super) fn take(rest: &mut Vec<String>) -> Result<bool>`: considering only the elements before the first literal `--`, removes every element equal to `--worker-prefix` and returns whether any was removed; an element there starting with `--worker-prefix=` is refused with `bail!("`--worker-prefix` takes no value; it hands subagents the same lane file the launch passes")`.
- `pub(super) fn admit(kind: HarnessKind, native_arguments: &[String]) -> Result<()>`: let `options` be the elements of native_arguments before the first literal `--` (all of them when there is none). Refuse, in this order, (1) kind != Claude: `bail!("`--worker-prefix` is supported only for launchers of kind `claude`, the one harness with a flag that appends a file to every subagent; this launcher is kind `{}`", kind.as_str())`; (2) no element of `options` equal to `-p` or `--print`: `bail!("`--worker-prefix` needs print mode: Claude Code reads `--append-subagent-system-prompt-file` only with `-p` / `--print`, so add `-p` or launch without `--worker-prefix`")`; (3) for flag in ["--append-subagent-system-prompt-file", "--append-subagent-system-prompt"], any element of native_arguments equal to flag or starting with `{flag}=`: `bail!("`{flag}` is already in your own arguments, and `--worker-prefix` passes the subagent flag itself — drop one of the two")`.
3. launch.rs edits: (a) after the route module lines add
```rust
#[path = "launch/worker_prefix.rs"]
mod worker_prefix;
```
(b) change `let rest: Vec<String> = argv.collect();` to `let mut rest: Vec<String> = argv.collect();` followed by `let worker_prefix = worker_prefix::take(&mut rest)?;`; (c) directly after `native_arguments.extend(rest.iter().cloned());` add
```rust
    if worker_prefix {
        worker_prefix::admit(launcher.kind, &native_arguments)?;
    }
```
(d) directly before `let composed = composed.flags;` add `let lane = composed.lane.clone();`; (e) directly after `command.extend(composed.flags.iter().cloned());` add
```rust
    if worker_prefix && let Some(lane) = &lane {
        command.extend(worker_prefix_flags(launcher.kind, lane));
    }
```
and add `worker_prefix_flags` to the existing `use vibe_workspace::system_prompt::{...}` list. Every other path (soft, not opted in, read delivery) already uses the stripped `rest`, so the option never reaches the harness and no worker flag is passed there.
4. cli/agent.rs, AgentCommand::Launch doc comment: after the line containing "`vibe agent claude --model opus -- -p` works." ... wait for the sentence to end at "delivery remain unavailable.", then add these lines before `#[command(external_subcommand)]`:
```
    ///
    /// `--worker-prefix` (kind `claude`, with `-p`/`--print` before any
    /// `--`) also passes `--append-subagent-system-prompt-file` naming the
    /// same lane file, so every subagent of the session starts with the
    /// same prefix bytes.
```
5. Create crates/vibe-cli/tests/cli_agent_worker_prefix.rs (`#![cfg(unix)]`, `mod common;`, `use common::UserScratch;`). Its Machine copies only what it needs from cli_agent_launch.rs: Machine::new (init the project, write SYSTEM-TEMPLATE.md into vibe_core::layout::current_boot_dir(), write agents.toml), the shim script of Machine::shim for `my-claude` only, the env set of Machine::harnessed (PATH, SHIM_ARGV, SHIM_ENV, SHIM_VERSION set to "codex-cli 0.167.0", SHIM_DEBUG_CONFIG set to "[]", CODEX_HOME under bin), argv() returning Option<Vec<String>> (None when the shim never wrote the file), and lane(). Launchers TOML:
```toml
[launchers.claude]
kind = "claude"
command = ["my-claude", "--no-telemetry"]
```
Tests: (1) worker_prefix_appends_the_subagent_flag_naming_the_parent_lane: agent ["claude","--worker-prefix","-p","hi"] succeeds; argv == ["--no-telemetry","--append-system-prompt-file",lane,"--exclude-dynamic-system-prompt-sections","--append-subagent-system-prompt-file",lane,"-p","hi"]. (2) without_worker_prefix_the_argv_is_unchanged: agent ["claude","-p","hi"]; argv == the same list without the two subagent entries. (3) worker_prefix_without_print_mode_is_refused: agent ["claude","--worker-prefix"] fails, stderr contains "needs print mode", argv() is None. (4) worker_prefix_for_another_kind_is_refused: agent ["codex","--worker-prefix","exec"] (built-in codex launcher) fails, stderr contains "supported only for launchers of kind `claude`". (5) worker_prefix_refuses_the_users_own_subagent_flag: agent ["claude","--worker-prefix","-p","--append-subagent-system-prompt-file","x.md"] fails, stderr contains "is already in your own arguments", argv() is None. (6) worker_prefix_is_dropped_when_the_project_has_not_opted_in: delete SYSTEM-TEMPLATE.md, agent ["claude","--worker-prefix","-p","hi"] succeeds, argv == ["--no-telemetry","-p","hi"]. Add no further tests.
6. Apply spec_edit to PROP-076 (status flip plus the appended sentence).
7. Run `cargo fmt --all`, the check, the existing `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_agent_launch a_launch_execs_the_argv_prefix` (must stay green), and `cargo clippy -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- `--worker-prefix` takes no value: it always uses the lane file of this same launch (Composed.lane), never a second composition.
- Claude Code reads the subagent flag only in print mode, so vibe agent REFUSES before anything starts when neither `-p` nor `--print` appears in the launcher's standing args or the user's args before the first `--`. A `-p` after `--` is a positional prompt to Claude Code (the doc comment's `vibe agent claude --model opus -- -p` form), so it does not count as print mode. It does not skip with a note.
- A non-claude kind with --worker-prefix is refused; all three refusals happen after the launcher is resolved and before the workspace is opened, so they also apply in a non-project directory.
- The option is recognised only before the first `--`, is removed from the argv on every path, and is never passed to the harness.
- A soft launch, an opted-out project and read delivery pass no worker flag (there is no lane), with no extra note; the existing soft-launch warning or opt-in note stands.
- Without --worker-prefix the argv is byte-identical to today.
- No environment variable is set: Claude Code 2.1.330 sets CLAUDE_CODE_ENABLE_APPEND_SUBAGENT_PROMPT=1 itself when --append-subagent-system-prompt-file names a non-empty file (verified in the 2.1.330 bundle), so passing the flag is enough.
- specmap.json is regenerated by the lead on main after this commit lands.

**Not in this entry.** Not in this entry: a SubagentStart hook, Codex/OpenCode/Qwen/Hermes/Pi support, a live run against real Claude Code, editing cli_agent_launch.rs, or docs/system-prompt/README.md.

**Depends on.** SWP-3-FLAGS, SWP-2-SPEC

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml replace
```
<SYS-WORKER-PREFIX-DELIVERY fact="true" status="spec/done">
```
with
```
<SYS-WORKER-PREFIX-DELIVERY fact="true" status="impl/done">
```
and replace
```
passes no worker flag either.</SYS-WORKER-PREFIX-DELIVERY></p>
```
with
```
passes no worker flag either. **Implemented (SWP-3, SWP-4).** `system_prompt::worker_prefix_flags` composes the flag, `vibe prompt flags --workers` prints it, and `vibe agent` takes `--worker-prefix`; the argument vector is proved against a recording shim in `crates/vibe-cli/tests/cli_agent_worker_prefix.rs`. Whether Claude Code delivers the file to its subagents was not measured live here.</SYS-WORKER-PREFIX-DELIVERY></p>
```

Entries that write the same file run one after another, never in parallel.
