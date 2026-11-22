# ADV-FIX-EXAMPLES-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe package` succeeds for a package skill projected to two agents that share one skill directory (codex and pi both use `.agents/skills`): the run records one artifact for that directory instead of failing with "names a file this set already declares".

**Write.** `crates/vibe-agent-projection/src/pkgskill/projection/binding.rs`, `crates/vibe-agent-projection/src/pkgskill/receipt/state.rs`, `crates/vibe-orchestrator/src/dispatch/backends.rs`, `crates/vibe-cli/tests/lifecycle_package/binding.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_lifecycle_package codex_and_pi_share_one_skill_directory`

**Test.** crates/vibe-cli/tests/lifecycle_package/binding.rs, new test fn `codex_and_pi_share_one_skill_directory_and_package_succeeds`: user = UserScratch::new(); project = support::project(true, &["codex", "pi"], false); fake_home = tempdir; `let (_, first) = run_json(&user, project.path(), fake_home.path())` (run_json already asserts exit success); assert report_status(&first, SKILL_KEY) == "ok"; let artifacts = &lifecycle_state(project.path()).execution[SKILL_KEY].artifacts; assert artifacts.len() == 1; assert artifacts[0].path == vibe_core::machine_json_path(&project.path().join(".agents/skills/demo")); assert artifacts[0].id == "org.example/lifecycle-skills#skill:demo:codex"; assert fs::read_to_string(project.path().join(".agents/skills/demo/SKILL.md")) == "first\n"; then `let (_, second) = run_json(...)` and assert report_status(&second, SKILL_KEY) == "fresh".

**Read first.**
- crates/vibe-agent-projection/src/pkgskill/projection/binding.rs:35-125
- crates/vibe-agent-projection/src/pkgskill/receipt/state.rs:340-367
- crates/vibe-orchestrator/src/dispatch/backends.rs:110-150
- crates/vibe-cli/tests/lifecycle_package/support.rs:1-60
- crates/vibe-cli/tests/lifecycle_package/binding.rs:1-30
- crates/vibe-agent-projection/src/pkgskill/receipt/containment.rs:10-25

**Steps.**
1. Run the check: it runs zero tests today. (Reproduced by hand: a project with `[[skill]] agents = ["codex", "pi"]` makes `vibe package` exit 1 with `artifact path … names a file this set already declares`.)
2. In binding.rs add to `impl ProjectSkillBinding` a method `pub fn artifact_rows(&self) -> Vec<(String, String)>` returning `(self.artifact_id(target.agent), vibe_core::machine_json_path(&target.path))` for each target in `self.targets` order, skipping a target whose `receipt::fold_key(machine_json_path(&target.path))` was already seen (first target wins). Tag it `#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE")]`.
3. In crates/vibe-orchestrator/src/dispatch/backends.rs `execute`, build `artifacts` from `binding.artifact_rows()` (kind "agent-skill") instead of iterating `binding.targets`.
4. In receipt/state.rs `artifacts_match`, build `expected` from `binding.artifact_rows()` (kind "agent-skill") instead of iterating `binding.targets`.
5. Add the test; run the check until it passes.

**Decisions already made.**
- De-duplication happens only in the artifact list; `binding.targets`, receipts and the files written are unchanged.
- Two targets are the same artifact when their `receipt::fold_key` of the machine JSON path is equal (the same fold the reply validator uses; it is `vibe_safefs::path_identity_key`).
- The surviving artifact id is the first target's in `binding.targets` order (codex before pi).
- backends.rs `execute` and state.rs `artifacts_match` are the only two places that build the artifact list; there is no third to find.

**Not in this entry.** Removing pi or codex from `binding.targets`, changing receipt/activation schemas, touching `vibe extensions` target_paths output, or running test suites wider than the check (the lead runs the regression suite).

Entries that write the same file run one after another, never in parallel.
