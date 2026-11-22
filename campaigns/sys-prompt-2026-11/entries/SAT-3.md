# SAT-3

Planned at commit `57b1e5a37`. Line numbers are hints; find each place by the quoted text or the item name.

**Outcome.** Resolve agent type bodies from the lane file and the locked package slot

**Write.** `crates/vibe-workspace/src/system_prompt/agent_types.rs`, `crates/vibe-workspace/src/system_prompt/agent_types/sources.rs`, `crates/vibe-workspace/src/system_prompt/agent_types/sources_tests.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-workspace --lib system_prompt::agent_types::sources_tests`

**Test.** crates/vibe-workspace/src/system_prompt/agent_types/sources_tests.rs: executor_rules_come_from_the_locked_working_rules_slot, missing_package_and_missing_lane_are_unavailable

**Read first.**
- crates/vibe-workspace/src/system_prompt/agent_types.rs
- crates/vibe-workspace/src/vibedeps.rs (slot_abs_path, in_place_slot_abs_path, is_in_place_slot)
- crates/vibe-core/src/package_ref.rs (Group::parse, Group::as_str)

**Steps.**
1. First fast-forward this worktree to the lead's branch named in the packet.
2. In agent_types.rs add `#[path = "agent_types/sources.rs"] mod sources;`, `pub use sources::{AgentTypeSources, LockedSlot};` and `#[cfg(test)] #[path = "agent_types/sources_tests.rs"] mod sources_tests;`. If SAT-2 landed first, keep its lines; the two entries add different lines.
3. In sources.rs (scope! and #[spec] edges to PROP-076#SYS-WORKER-TYPES):
`#[derive(Debug, Clone, PartialEq, Eq)] pub struct LockedSlot { pub group: vibe_core::Group, pub name: String, pub version: semver::Version }`
`#[derive(Debug, Clone)] pub struct AgentTypeSources { pub lane: Result<Vec<u8>, String>, pub workspace_root: PathBuf, pub locked: Result<Vec<LockedSlot>, String> }`. Document `workspace_root` as the WORKSPACE root (where vibe.lock and vibevm/vibedeps live), never a member node's directory.
`impl AgentTypeSources { pub fn resolve(&self, body: &AgentTypeBody) -> Result<Vec<u8>, String> }` with a doctest.
4. Test 1: a tempdir as workspace_root. Build the slot dir with crate::vibedeps::slot_abs_path(root, &group, "working-rules", &"1.0.12".parse().unwrap()), create it, and write <slot>/rules/worker.md = b"RULES\n". Set locked = Ok(vec![LockedSlot { group: Group::parse("org.vibevm.world").unwrap(), name: "working-rules".into(), version: "1.0.12".parse().unwrap() }]). Assert resolve(&builtin_agent_types()[1].body) == Ok(b"RULES\n".to_vec()).
Test 2:
- locked Ok(vec![]) -> Err containing `org.vibevm.world/working-rules`;
- locked Err("no lock") -> Err("no lock");
- the slot locked but worker.md absent -> Err containing "vibe install";
- lane Err("x") -> resolve(&AgentTypeBody::ProjectLane) == Err("x").
5. Run the check; commit.

**Decisions already made.**
- ProjectLane -> self.lane.clone().
- PackageFile { group, name, path }:
- locked Err(reason) -> Err(reason.clone()).
- Find the slot whose group.as_str() == group and name == name. None -> Err(format!("`{group}/{name}` is not installed in this project, so this type has no body; installing it provides one")).
- dir = vibedeps::in_place_slot_abs_path(root, &slot.group, name) when vibedeps::is_in_place_slot(root, &slot.group, name), else vibedeps::slot_abs_path(root, &slot.group, name, &slot.version).
- file = dir joined with each '/'-separated segment of path.
- Read with std::fs::read. NotFound -> Err(format!("`{}` is missing; run `vibe install`", crate::path_to_slash(file.strip_prefix(root).unwrap_or(&file)))). Any other io error -> Err(error.to_string()).
- No lockfile parsing here: the CLI builds `locked` from vibe_core::manifest::Lockfile (SAT-4).

**Not in this entry.** Not in this entry: reading vibe.lock, package-manifest-declared types, the CLI.

**Depends on.** SAT-1

**Lead side (not the executor's).**

After applying: regenerate specmap.json on main.

Entries that write the same file run one after another, never in parallel.
