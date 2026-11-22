# SAT-7

Planned at commit `57b1e5a37`. Line numbers are hints; find each place by the quoted text or the item name.

**Outcome.** A soft launch or an opted-out launch withdraws the generated worker type

**Write.** `crates/vibe-workspace/src/system_prompt/agent_types/sync.rs`, `crates/vibe-workspace/src/system_prompt/agent_types.rs`, `crates/vibe-cli/src/commands/agent/launch.rs`, `crates/vibe-cli/src/commands/agent/launch/agent_types.rs`, `crates/vibe-cli/tests/cli_agent_types_launch.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_agent_types_launch launch_removes_the_generated_worker_type`

**Test.** crates/vibe-cli/tests/cli_agent_types_launch.rs::opted_out_launch_removes_the_generated_worker_type, crates/vibe-cli/tests/cli_agent_types_launch.rs::failed_build_soft_launch_removes_the_generated_worker_type

**Read first.**
- crates/vibe-workspace/src/system_prompt/agent_types/sync.rs
- crates/vibe-cli/src/commands/agent/launch.rs (run: the failed-build, opted_in error, !opted_in, compose error and undeliverable branches)
- crates/vibe-cli/src/commands/agent/launch/agent_types.rs
- crates/vibe-cli/tests/cli_agent_types_launch.rs
- crates/vibe-cli/tests/cli_agent_launch.rs (a_failed_build_launches_softly_and_never_delivers_the_older_lane: how the build is broken)

**Steps.**
1. First fast-forward this worktree to the lead's branch named in the packet.
2. sync.rs: add `pub fn withdraw_lane_types(node_dir: &Path, specs: &[AgentTypeSpec]) -> Result<Vec<AgentTypeOutcome>, AgentTypesError>` with a #[spec] edge to PROP-076#SYS-WORKER-TYPES. For each spec whose body is AgentTypeBody::ProjectLane, a vibe-generated file is removed (Removed, reason Some("this launch delivers no lane")). Absent or foreign files are left alone and produce no outcome. Re-export it from agent_types.rs next to sync_agent_types.
3. launch/agent_types.rs: add `pub(super) fn withdraw(ctx: &output::Context, kind: HarnessKind, selected: &prompt::Selected)`. It returns at once unless kind == HarnessKind::Claude. Otherwise it calls withdraw_lane_types(&selected.node_dir, &builtin_agent_types()). A Removed outcome prints `note: subagent type `{file}` removed` unless quiet. An Err prints the same always-on warning shape as refresh.
4. launch.rs: add the one line `agent_types::withdraw(ctx, launcher.kind, &selected);` immediately before each of these five returns, in the code after the `if let Some(selection) = native_selection` early exits (Hermes only):
(1) the failed-build `return soft(`;
(2) the opted_in error `return soft(`;
(3) the !opted_in plain `return exec(`;
(4) the compose error `return soft(`;
(5) the `composed.undeliverable` `return soft(`.
Keep launch.rs under 600 lines.
5. Test opted_out_launch_removes_the_generated_worker_type:
- Machine::new().
- Remove P/<current_boot_dir>/SYSTEM-TEMPLATE.md, as worker_prefix_is_dropped_when_the_project_has_not_opted_in does (no launch before it).
- Create P/.claude/agents/ and write vibe-worker.md = vibe_workspace::system_prompt::agent_types::render_claude_agent_type(&builtin_agent_types()[0], b"OLD LANE\n").
- `vibe agent claude -p hi` succeeds.
- vibe-worker.md no longer exists.
- argv == ["--no-telemetry", "-p", "hi"].
6. Test failed_build_soft_launch_removes_the_generated_worker_type:
- Machine::new().
- `vibe agent claude -p hi` creates P/.claude/agents/vibe-worker.md.
- Break the build exactly as a_failed_build_launches_softly_and_never_delivers_the_older_lane does: remove SYSTEM-TEMPLATE.md and create a directory with that name.
- `vibe agent claude -p hi` succeeds, and stderr contains "could not be built".
- vibe-worker.md no longer exists.
- argv == ["--no-telemetry", "-p", "hi"].
7. Run the check (its filter matches both tests); commit.

**Decisions already made.**
- Only the lane-bodied type is withdrawn. vibe-executor stays, because its body does not depend on the lane.
- Withdrawal never creates a directory or the ignore file and never touches foreign files.
- The not-a-vibe-project soft path has no node and is unchanged. soft() itself is unchanged.

**Not in this entry.** Not in this entry: removing vibe-executor or the ignore file on opt-out, a vibe check rule, changing soft(), docs.

**Depends on.** SAT-6

**Lead side (not the executor's).**

After applying:
- Regenerate specmap.json on main.
- Update docs/system-prompt/README.md and CHANGELOG.md for: `vibe prompt types` (with --check and --json); the launch regeneration and withdrawal; the two built-in types and their bodies; the nested .claude/agents/.gitignore; that types take effect at the next session; that executor packets still carry the quiet clause; and the caution against combining --worker-prefix with either type.
- Run the full gate once for the whole list (tools/self-check.sh, including `cargo xtask conform check`).

Entries that write the same file run one after another, never in parallel.
