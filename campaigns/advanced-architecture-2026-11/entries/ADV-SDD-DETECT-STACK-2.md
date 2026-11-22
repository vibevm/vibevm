# ADV-SDD-DETECT-STACK-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The `--plan-only` proposal lists the agent instruction files (CLAUDE.md, AGENTS.md, GEMINI.md) as keep or create, and every other file init would write as create, keep or append, matching what a real `vibe init` then reports.

**Write.** `crates/vibe-cli/src/commands/init/proposal.rs`, `crates/vibe-cli/src/commands/init/mod.rs`, `crates/vibe-cli/tests/cli_init_detect.rs`

**Check.** `cargo test -p vibe-cli --test cli_init_detect plan_writes_ 2>&1 | grep -F 'test result: ok. 1 passed'`

**Test.** crates/vibe-cli/tests/cli_init_detect.rs: `plan_writes_list_matches_what_init_creates` (does not exist yet; the filter runs 0 tests before the work, so grep fails).

**Read first.**
- crates/vibe-cli/src/commands/init/proposal.rs: struct Proposal, fn new
- crates/vibe-cli/src/commands/init/mod.rs: fn create_project (steps 1 to 6, the files it writes)
- crates/vibe-cli/src/commands/init/helpers.rs: fn generate_boot_artifacts, fn ensure_local_settings_gitignored, fn ensure_file
- crates/vibe-core/src/layout.rs: current_boot_dir, current_boot_index, BOOT_ARTIFACTS_LOCK
- crates/vibe-cli/tests/cli_init_detect.rs

**Steps.**
1. Runs after ADV-SDD-DETECT-STACK-1 (same files). In proposal.rs add `#[derive(serde::Serialize)] pub(super) struct PlannedFile { path: String, action: &'static str }` and two fields on Proposal: `agent_files: Vec<PlannedFile>` and `writes: Vec<PlannedFile>`. Change the constructor to `pub(super) fn new(detection: super::detect::Detection, root: &std::path::Path) -> Self`.
2. In proposal.rs add `fn planned_writes(root: &Path) -> Vec<PlannedFile>` producing exactly these paths in this order, each spelled with `vibe_core::machine_json_path`: `current_boot_dir().join("00-core.md")`, `current_boot_dir().join("90-user.md")`, `vibe.toml` (Manifest::FILENAME), `vibe.lock` (Lockfile::FILENAME), `.vibe/.gitignore`, `.gitignore`, `vibe_core::layout::current_boot_index()`, `current_boot_dir().join(vibe_core::layout::BOOT_ARTIFACTS_LOCK)`. Action is `create` when `root.join(path).exists()` is false, else `keep`; the one exception is `.gitignore`: when it exists and `std::fs::read_to_string` succeeds and no line contains `settings.local.toml`, the action is `append` (this mirrors helpers::ensure_local_settings_gitignored).
3. In proposal.rs add `fn planned_agent_files(root: &Path) -> Vec<PlannedFile>` for `CLAUDE.md`, `AGENTS.md`, `GEMINI.md` in that order: `keep` when the file exists (its text is kept and only the generated <vibevm> block is added), `create` otherwise.
4. In proposal.rs `report`, human mode: after the stack hints add one `ctx.hint(&format!("agent file: {path} · {action}"))` per agent file and one `ctx.hint(&format!("would write: {path} · {action}"))` per write, before the skipped hints.
5. In mod.rs update the one caller to `proposal::Proposal::new(detection, &root)`.
6. Add test `plan_writes_list_matches_what_init_creates`: fixture `Cargo.toml` (as in the scan test), `CLAUDE.md`=`# Notes\nmine\n`, `.gitignore`=`target/\n`. Plan JSON: agent_files == `[{"path":"CLAUDE.md","action":"keep"},{"path":"AGENTS.md","action":"create"},{"path":"GEMINI.md","action":"create"}]`; writes contains `{"path":".gitignore","action":"append"}` and `{"path":"vibe.toml","action":"create"}`. Then run plain `vibe init --json --path <dir>` through the same UserScratch, assert success, and build a BTreeMap path->action from its `outcomes`. Build the plan map from writes plus agent_files with create->`created`, keep->`kept`, append->`created`. Assert the two maps are equal.

**Decisions already made.**
- writes[] holds the eight non-agent files; agent_files[] holds the three agent files; together they equal the paths a plain `vibe init` reports for a new project (the test is the drift guard).
- Action vocabulary: writes `create|keep|append`, agent_files `keep|create`.
- Existence is checked with Path::exists and the .gitignore read with std::fs::read_to_string, exactly as init's own writers check; nothing is written.
- The static boot lane file is not listed: a new project has no spec_format, so init creates none (observed: `vibe init --json` reports 11 outcomes).

**Not in this entry.** Not in this entry: refactoring create_project to share the path list, changing the InitReport wire schema, listing nested-package or app scaffold files, and next_commands.

**Depends on.** ADV-SDD-DETECT-STACK-1

**Spec edit (apply as a step of this entry).**

None.

Entries that write the same file run one after another, never in parallel.
