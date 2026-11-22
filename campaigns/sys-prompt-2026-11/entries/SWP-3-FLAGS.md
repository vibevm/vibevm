# SWP-3-FLAGS

Planned at commit `9b197755e`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe prompt flags --harness claude --workers` prints the subagent flag naming the same lane file (via system_prompt::worker_prefix_flags); other kinds get a note and no flag

**Write.** `crates/vibe-workspace/src/system_prompt/harness/workers.rs`, `crates/vibe-workspace/src/system_prompt/harness.rs`, `crates/vibe-workspace/src/system_prompt.rs`, `crates/vibe-workspace/src/system_prompt/harness/tests.rs`, `crates/vibe-cli/src/cli/prompt.rs`, `crates/vibe-cli/src/commands/prompt.rs`, `crates/vibe-cli/src/commands/prompt/flags.rs`, `crates/vibe-cli/tests/cli_prompt.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-workspace --lib worker_prefix && bash tools/cargo-test-ran.sh -p vibe-cli --test cli_prompt flags_with_workers`

**Test.** (1) crates/vibe-workspace/src/system_prompt/harness/tests.rs, directly after claude_takes_exactly_one_file_flag_and_the_exclude_flag: worker_prefix_claude_appends_the_same_lane_file_to_every_subagent and worker_prefix_is_empty_for_every_kind_without_a_subagent_flag. (2) crates/vibe-cli/tests/cli_prompt.rs, after flags_print_flags_only_and_never_a_program_or_lane_bytes: flags_with_workers_add_the_subagent_flag_naming_the_same_lane and flags_with_workers_for_codex_print_no_worker_flag_and_say_so. At 9b197755e neither filter matches any test, so each wrapper call prints `no test ran; the filter matched nothing` and exits 1.

**Read first.**
- crates/vibe-workspace/src/system_prompt/harness.rs (HarnessKind and HarnessKind::ALL, LaunchFlags, the `#[path = "harness/notes.rs"] mod notes;` and `pub use notes::{...};` lines)
- crates/vibe-workspace/src/system_prompt/harness/notes.rs (module header: doc + specmark::scope! line to imitate)
- crates/vibe-workspace/src/system_prompt/harness/tests.rs (LANE_PATH, flags(), claude_takes_exactly_one_file_flag_and_the_exclude_flag)
- crates/vibe-workspace/src/system_prompt.rs (the `pub use harness::{...}` list)
- crates/vibe-cli/src/commands/prompt/flags.rs (run, struct Composed, compose_with_personal)
- crates/vibe-cli/src/cli/prompt.rs (PromptCommand::Flags)
- crates/vibe-cli/src/commands/prompt.rs (the PromptCommand::Flags dispatch arm)
- crates/vibe-cli/tests/cli_prompt.rs (Project::opted_in, Project::prompt, Project::boot, flags_print_flags_only_and_never_a_program_or_lane_bytes)

**Steps.**
1. Fast-forward the worktree to the lead's branch first.
2. harness.rs is 587 lines (limit 600): put the new workspace code in a new child module, not in harness.rs. Create crates/vibe-workspace/src/system_prompt/harness/workers.rs with exactly:
```rust
//! The flag that hands a launch's composed lane to every subagent it
//! starts, so same-type workers of one wave share one prefix
//! (`#SYS-WORKER-PREFIX-DELIVERY`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-PREFIX-DELIVERY");

use std::path::Path;

use specmark::spec;

use super::HarnessKind;

/// The flags that append the composed lane file to every subagent a launch
/// starts. `lane` is the same file the parent launch names, so the workers'
/// prefix is never a second composition. Empty for every kind with no such
/// flag.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-PREFIX-DELIVERY")]
pub fn worker_prefix_flags(kind: HarnessKind, lane: &Path) -> Vec<String> {
    match kind {
        HarnessKind::Claude => vec![
            "--append-subagent-system-prompt-file".to_owned(),
            lane.display().to_string(),
        ],
        HarnessKind::Codex
        | HarnessKind::OpenCode
        | HarnessKind::Qwen
        | HarnessKind::Hermes
        | HarnessKind::Pi => Vec::new(),
    }
}
```
3. In harness.rs, directly after the existing `pub use notes::{...};` block, add:
```rust
#[path = "harness/workers.rs"]
mod workers;
pub use workers::worker_prefix_flags;
```
4. In crates/vibe-workspace/src/system_prompt.rs add `worker_prefix_flags` to the `pub use harness::{...}` list (rustfmt order).
5. In harness/tests.rs, directly after claude_takes_exactly_one_file_flag_and_the_exclude_flag, add:
```rust
#[test]
fn worker_prefix_claude_appends_the_same_lane_file_to_every_subagent() {
    let workers = worker_prefix_flags(HarnessKind::Claude, Path::new(LANE_PATH));
    assert_eq!(workers, ["--append-subagent-system-prompt-file", LANE_PATH]);
    // The same file the parent launch names: no second composition.
    let parent = flags(HarnessKind::Claude, HarnessDelivery::Launch);
    assert_eq!(parent.flags[1], workers[1]);
}

#[test]
fn worker_prefix_is_empty_for_every_kind_without_a_subagent_flag() {
    for kind in HarnessKind::ALL
        .into_iter()
        .filter(|kind| *kind != HarnessKind::Claude)
    {
        assert!(
            worker_prefix_flags(kind, Path::new(LANE_PATH)).is_empty(),
            "{}",
            kind.as_str()
        );
    }
}
```
6. cli/prompt.rs, PromptCommand::Flags: add after the `path` field:
```rust
        /// Also print the flag that appends the same lane file to every
        /// subagent the launch starts (Claude Code only, print mode only);
        /// a harness without such a flag gets a note and no flag.
        #[arg(long)]
        workers: bool,
```
7. commands/prompt.rs dispatch arm `PromptCommand::Flags { harness, node, path }`: destructure `workers` too and call `flags::run(ctx, &select(&path, node.as_deref())?, &harness, workers)`. Other matches on `PromptCommand::Flags { .. }` need no change.
8. flags.rs, struct Composed: add a field after `undeliverable`:
```rust
    /// The lane file the flags name, when they name one. `vibe prompt flags
    /// --workers` and `vibe agent --worker-prefix` hand this same path to a
    /// harness's subagents, so the workers' prefix is never a second
    /// composition (`#SYS-WORKER-PREFIX-DELIVERY`).
    pub(crate) lane: Option<PathBuf>,
```
and add `use std::path::PathBuf;`. Set it in all four `Composed { .. }` literals inside compose_with_personal: the three early returns get `lane: None`; the final `Ok(Composed { .. })` gets `lane: Some(lane_path)` (move it there; if lane_path is still borrowed later, clone it). The `Composed` structs in supplied_base.rs and qwen_identity.rs are different types; leave them.
9. flags.rs, run: add parameter `workers: bool`. Right after `let composed = compose(selected, kind, launcher.as_ref(), &[])?;` (before the match that moves its fields) add `let lane = composed.lane.clone();`. After the Qwen-note block, before `let report = ...`, add (here `composed` is the LaunchFlags):
```rust
    if workers {
        match lane
            .as_deref()
            .map(|lane| worker_prefix_flags(kind, lane))
        {
            Some(extra) if !extra.is_empty() => {
                composed.flags.extend(extra);
                composed.notes.push(
                    "`--append-subagent-system-prompt-file` appends this same lane file to every \
                     subagent of the session, and Claude Code reads it only in print mode (`-p`)"
                        .to_owned(),
                );
            }
            Some(_) => composed.notes.push(format!(
                "`--workers`: kind `{}` has no flag that appends a file to its subagents' system \
                 prompt, so no worker flag is printed",
                kind.as_str()
            )),
            None => composed.notes.push(
                "`--workers`: this launch passes no lane, so no worker flag is printed".to_owned(),
            ),
        }
    }
```
Import `worker_prefix_flags` from vibe_workspace::system_prompt.
10. cli_prompt.rs: add after flags_print_flags_only_and_never_a_program_or_lane_bytes two tests. (a) flags_with_workers_add_the_subagent_flag_naming_the_same_lane: Project::opted_in(); prompt("build") succeeds; prompt("flags").args(["--harness","claude","--workers","--json"]) succeeds; parse report["flags"] into Vec<String>; let lane = project.boot("AGENT-LANE.md").display().to_string(); assert the flags equal ["--append-system-prompt-file", lane, "--exclude-dynamic-system-prompt-sections", "--append-subagent-system-prompt-file", lane] (as Strings). (b) flags_with_workers_for_codex_print_no_worker_flag_and_say_so: build, then run `flags --harness codex --json` with and without `--workers`; assert the two `flags` arrays are equal and that some entry of the --workers report's `notes` contains "has no flag that appends a file to its subagents' system prompt".
11. Run `cargo fmt --all`, the check, the existing `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_prompt flags_print_flags_only` and `bash tools/cargo-test-ran.sh -p vibe-workspace --lib claude_takes_exactly_one_file_flag` (both must stay green), then `cargo clippy -p vibe-workspace -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- This entry carries the whole owner intent of SWP-3-FLAGS: the workspace function and the `--workers` CLI option ship together (the earlier split into a separate SWP-3-B entry is folded back, so the set stays the owner's four ids).
- Function name worker_prefix_flags(kind: HarnessKind, lane: &Path) -> Vec<String>, public, re-exported from vibe_workspace::system_prompt, in child module harness/workers.rs because harness.rs is at 587 of 600 lines.
- Claude flag literal: `--append-subagent-system-prompt-file` followed by the lane path. Verified present in Claude Code 2.1.330 (hidden from --help; built-in description 'Read a system prompt from a file and append it to every Task-tool subagent's system prompt (only works with --print)').
- The match is exhaustive with no wildcard arm, so a new kind forces a decision.
- Option name `--workers` (bool, no value) on `vibe prompt flags`. With it, Claude's line is the parent flags followed by `--append-subagent-system-prompt-file <lane>`: one pasteable line for `claude $(vibe prompt flags --harness claude --workers) -p ...`.
- The worker flag names Composed.lane, the exact path the parent flags name (including a personal lane when one was composed), never a second composition.
- No wire/schema change: the pair goes into the existing `flags` array and the explanation into `notes`; the protocol tag stays vibe-prompt-flags/1.
- A kind without a subagent flag prints its usual flags unchanged plus one note; it is not a refusal (this command only prints), and `vibe prompt flags` does no print-mode check (the refusal lives in vibe agent, SWP-4-LAUNCH).
- The printable stale-lane case still gets the worker flag (it points at the same bytes the parent flags point at, beside the existing stale note).
- No environment variable is printed or assigned: Claude Code 2.1.330 sets CLAUDE_CODE_ENABLE_APPEND_SUBAGENT_PROMPT=1 itself when --append-subagent-system-prompt-file names a non-empty file (verified in the 2.1.330 bundle), so the flag is enough.
- The spec marker names SYS-WORKER-PREFIX-DELIVERY, whose anchor SWP-2-SPEC creates; specmap.json is regenerated by the lead on main.

**Not in this entry.** Not in this entry: changing LaunchInputs or launch_flags, vibe agent / launch.rs, a JTD schema or generated wire type change, docs/system-prompt/README.md, a print-mode check in `vibe prompt flags`, or any environment assignment.

**Depends on.** SWP-2-SPEC

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
