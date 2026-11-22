# ADV-CLIENTS-LIVE-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `cargo test -p vibe-cli --test cli_live_clients -- --ignored --exact live_skill_install_qwen --nocapture` projects a declared skill into an isolated `<home>/.qwen/skills/live-skill` with `vibe skill install --agent qwen --scope user`, removes it with `vibe skill uninstall`, keeps the foreign neighbour, and prints `LIVE qwen <version> deploy=<ok|fail> listed=unverifiable undeploy=<ok|fail|not-run>`. With VIBE_LIVE_QWEN_BIN=skip it prints `SKIP qwen: ...` and passes.

**Write.** `crates/vibe-cli/tests/cli_live_clients.rs`, `crates/vibe-cli/tests/cli_live_clients/harness.rs`

**Check.** `VIBE_LIVE_QWEN_BIN=skip cargo test -p vibe-cli --test cli_live_clients -- --ignored --exact live_skill_install_qwen 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** New #[ignore] test `live_skill_install_qwen` in crates/vibe-cli/tests/cli_live_clients.rs. Before this entry the --exact filter matches nothing (`ok. 0 passed`), so the grep fails. After it, the SKIP path passes as 1 test on any OS. ADV-CLIENTS-LIVE-1's check (filters `live_deploy_` and `live_harness_`) does not match the new test and still reports `ok. 4 passed`.

**Read first.**
- crates/vibe-cli/tests/cli_live_clients.rs (run_deploy_lane, listing, plant_foreign: the lane to mirror)
- crates/vibe-cli/tests/cli_live_clients/harness.rs (Client, locate, IsolatedHome, RealHomeSentinel, summary)
- crates/vibe-cli/tests/cli_skill.rs (project() fixture and run_json: the `[package]` + `[[skill]]` manifest and the JSON results[0].status contract)
- crates/vibe-cli/src/cli/skill.rs (SkillInstallArgs/SkillUninstallArgs flags: --agent, --scope, --yes, --path)
- crates/vibe-agent-projection/src/agents/home_paths.rs (QwenCode user skills root: <home>/.qwen/skills)

**Steps.**
1. This entry runs after ADV-CLIENTS-LIVE-1, which created both files; the two entries share both files and run one after another. In harness.rs add `Qwen` to `enum Client` with word "qwen", override_var "VIBE_LIVE_QWEN_BIN" and skills_relative ".qwen/skills". Extend every exhaustive match on Client (word, override_var, skills_relative, client_command): in client_command, Qwen sets no extra variable. IsolatedHome::new gains no new subdirectory: vibe creates `.qwen/skills` itself, and plant_foreign already create_dir_all's its own path.
2. In cli_live_clients.rs, in `fn listing`, add the arm `Client::Qwen => unreachable!("qwen has no deploy lane; see run_skill_lane")`. If any client dispatch in run_deploy_lane or its helpers was written as if/else on the client rather than an exhaustive match, first turn it into an exhaustive match with no `_` arm, then add the same unreachable! arm for Qwen.
3. In harness.rs RealHomeSentinel::capture, add the home-relative path .qwen/skills/live-skill to the skill-dir existence probes.
4. In cli_live_clients.rs add `fn skill_project(root: &Path)`. It writes skills/live-skill/SKILL.md with the same bytes as deploy_project and the vibe.toml literal in the decisions, creating parent dirs with create_dir_all.
5. In cli_live_clients.rs add `fn run_skill_lane(client: Client)`, in this order. (1) locate; on Skip, eprintln and return. (2) Capture the sentinel, then create the IsolatedHome, a UserScratch and a project tempdir, and call skill_project and plant_foreign(home, client). (3) Get the version via client_command `--version` and version_token. (4) Run `--json skill install --agent qwen --scope user --yes --path <project>` through vibe_command(bin.parent()). deploy=ok when the exit is 0, stdout parses as JSON with results[0].status == "created", and `<home>/.qwen/skills/live-skill/SKILL.md` exists; otherwise deploy=fail and eprintln vibe's stderr. listed=unverifiable always. (5) If deploy=ok, run `--json skill uninstall --agent qwen --scope user --yes --path <project>`. undeploy=ok when the exit is 0, results[0].status == "removed", `<home>/.qwen/skills/live-skill` is absent, and the foreign SKILL.md is still "foreign neighbour\n"; otherwise undeploy=fail. If deploy failed, undeploy=not-run. (6) println! the summary, call sentinel.assert_unchanged(), and assert deploy=="ok" and undeploy=="ok" with the summary as the message.
6. Add `#[test] #[ignore = "live: needs the real client binary — cargo test -p vibe-cli --test cli_live_clients -- --ignored --nocapture"] fn live_skill_install_qwen() { run_skill_lane(Client::Qwen) }`. Extend the //! header to name qwen as the skill-only client.
7. Run `cargo clippy -p vibe-cli --tests -- -D warnings` and `cargo fmt --all`; wc -l both files and keep each under 600 lines (if cli_live_clients.rs would pass 600, move run_skill_lane and skill_project into a new child file crates/vibe-cli/tests/cli_live_clients/skill_lane.rs included with #[path] and add it to the commit).

**Decisions already made.**
- Qwen stays in the skill lane: `vibe skill install --agent qwen --scope user` (Agent::parse_filter accepts "qwen" and "qwen-code"). No deploy enum variant is added, and the test never claims Qwen deploy support; the deploy-lane dispatch answers Qwen with unreachable!, because no deploy test is ever called with Client::Qwen.
- listed is always `unverifiable` for qwen: qwen 0.27.x has no skill listing subcommand, and anything that lists skills needs a model call. Only file placement, the JSON statuses and the neighbour are checked.
- Manifest literal written by skill_project (verified working today):
```
[package]
group = "org.example"
name = "live-clients"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "UPL-1.0"
description = "live client check fixture"
keywords = ["fixture"]

[[skill]]
name = "live-skill"
path = "skills/live-skill"
description = "Live check skill"
include = ["SKILL.md"]
```
- Skill-install JSON: stdout is one JSON document with `results[0].status` equal to "created" on install and "removed" on uninstall, and `results[0].agent` == "qwen-code" (verified today).
- The summary uses the same summary() function, so the words are the same as for the deploy lane.
- This entry shares both files with ADV-CLIENTS-LIVE-1 and runs strictly after it. ADV-CLIENTS-LIVE-1's check is unchanged and still exact, because its filters cannot match `live_skill_install_qwen`.

**Not in this entry.** Not in this entry: a Qwen deploy target or any change to SkillClient/PluginClient in crates/vibe-lifecycle (the package-declared client capability owns that); project-scope qwen projection; the docs 'verified with' table.

**Depends on.** ADV-CLIENTS-LIVE-1

Entries that write the same file run one after another, never in parallel.
