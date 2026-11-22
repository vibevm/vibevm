# ADV-CLIENTS-LIVE-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `cargo test -p vibe-cli --test cli_live_clients -- --ignored live_deploy_ --nocapture` deploys a skill and a plugin into an isolated home for each installed claude, codex and opencode, checks the client's own listing, undeploys, checks the foreign neighbour and that the real home gained no live-* entry, and prints one `LIVE <client> <version> deploy=<ok|fail> listed=<ok|fail|unverifiable|not-run> undeploy=<ok|fail|not-run>` line per client. A client set to `skip` prints `SKIP <client>: ...` and passes.

**Write.** `crates/vibe-cli/tests/cli_live_clients.rs`, `crates/vibe-cli/tests/cli_live_clients/harness.rs`

**Check.** `VIBE_LIVE_CLAUDE_BIN=skip VIBE_LIVE_CODEX_BIN=skip VIBE_LIVE_OPENCODE_BIN=skip cargo test -p vibe-cli --test cli_live_clients -- --ignored live_deploy_ live_harness_ 2>&1 | grep -F "test result: ok. 4 passed"`

**Test.** New file crates/vibe-cli/tests/cli_live_clients.rs. It holds four #[ignore] tests: live_deploy_claude, live_deploy_codex and live_deploy_opencode (each calls run_deploy_lane(Client::X)), plus live_harness_reads_versions_and_listings. That last one is pure: it spawns nothing and checks the harness parsers against fixed literals. The check passes two name filters (`live_deploy_` and `live_harness_`), which together match exactly these four tests; a later skill-lane test named `live_skill_...` matches neither, so the count stays 4 after later entries land. The check sets every client to `skip`, so it runs the three SKIP paths plus the pure test on any OS, with no real client and no real home involved. Before this entry the command fails, because cargo exits 101 with `no test target named cli_live_clients`. After it, 4 tests pass. The real acceptance, outside the check, is a maintainer run with real binaries.

**Read first.**
- crates/vibe-cli/tests/cli_deploy.rs (fixture style: write_project, document; how `vibe deploy --profile X --path P` and `vibe undeploy --profile X --path P` are driven)
- crates/vibe-test-support/src/lib.rs (UserScratch::new, UserScratch::vibe: the VIBE_SETTINGS/cache isolation every vibe child must use)
- crates/vibe-cli/tests/cli_live_e2e.rs (the opt-in #[ignore = "live: ..."] convention)
- crates/vibe-cli/src/commands/deploy/clients.rs (resolve_clients and is_executable_file: vibe finds `claude`/`codex`/`opencode` on the child's PATH, absolute dirs only, execute bit required on unix)
- crates/vibe-lifecycle/src/mechanism/deploy/plugin/client.rs (PluginClient::private_root: CLAUDE_CONFIG_DIR=<home>/.claude, CODEX_HOME=<home>/.codex; supported_version / version_pair pins)
- crates/vibe-lifecycle/src/mechanism/deploy/skill/client.rs (SkillClient::skills_relative: .claude/skills, .agents/skills, .config/opencode/skills)

**Steps.**
1. Create crates/vibe-cli/tests/cli_live_clients/harness.rs. Define `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Client { Claude, Codex, OpenCode }` with `pub fn word(self) -> &'static str` returning "claude"/"codex"/"opencode", `pub fn override_var(self) -> &'static str` returning "VIBE_LIVE_CLAUDE_BIN"/"VIBE_LIVE_CODEX_BIN"/"VIBE_LIVE_OPENCODE_BIN", and `pub fn skills_relative(self) -> &'static str` returning ".claude/skills"/".agents/skills"/".config/opencode/skills". Write each as an exhaustive `match self` with no `_` arm. Add no Qwen variant here, because an unconstructed variant is dead code and fails clippy -D warnings.
2. In harness.rs add `pub enum Located { Skip(String), Found(PathBuf) }` and `pub fn locate(client: Client) -> Located`. The rules, in this order: (a) the override var equals exactly `skip`: return Skip("<VAR>=skip"). (b) cfg!(windows): return Skip("the Windows home cannot be isolated: vibe resolves the profile folder, not USERPROFILE"). (c) the override var is set to anything else: it must be an absolute path to an executable file (per `fn is_runnable` below) whose file_stem equals client.word(), otherwise panic!("<VAR>=<value> must be `skip` or an absolute path to an executable file named <word>"). Return Found(path). (d) no override: walk std::env::split_paths(PATH), skipping non-absolute dirs. The first `<dir>/<word>` for which is_runnable is true is Found. (e) otherwise Skip("not found"). The test prints `SKIP <word>: <reason>` with eprintln! and returns.
3. In harness.rs add the private `fn is_runnable(path: &Path) -> bool`, mirroring vibe's is_executable_file in crates/vibe-cli/src/commands/deploy/clients.rs: path.metadata() must succeed and be is_file(); then inside a `#[cfg(unix)] { use std::os::unix::fs::PermissionsExt as _; metadata.permissions().mode() & 0o111 != 0 }` block, and a `#[cfg(not(unix))] { true }` block. The import lives only inside the cfg(unix) block, so no platform has an unused import.
4. In harness.rs add `pub struct IsolatedHome { dir: tempfile::TempDir }`. `IsolatedHome::new()` creates the tempdir and the subdirectories `.claude`, `.codex` and `.config`. `path()` returns the root. `client_command(&self, client: Client, bin: &Path) -> assert_cmd::Command` builds assert_cmd::Command::new(bin) with env_clear(), then sets HOME, USERPROFILE and XDG_CONFIG_HOME=<home>/.config, then an exhaustive `match client`: Claude sets CLAUDE_CONFIG_DIR=<home>/.claude, Codex sets CODEX_HOME=<home>/.codex, OpenCode sets nothing more. It copies PATH, TMPDIR, TEMP, TMP and LANG from the test process when present, and sets .timeout(Duration::from_secs(120)). `vibe_command(&self, user: &vibe_test_support::UserScratch, bin_dir: &Path) -> assert_cmd::Command` starts from user.vibe(), sets HOME, USERPROFILE, XDG_CONFIG_HOME and CODEX_HOME the same way, calls env_remove("CLAUDE_CONFIG_DIR"), sets PATH to bin_dir joined in front of the inherited PATH with std::env::join_paths, and sets .timeout(Duration::from_secs(600)).
5. In harness.rs add these pure functions. `pub fn version_token(stdout: &str) -> String`: scan the lines of stdout in order and, inside each line, its whitespace-separated tokens; return the first token that starts with an ASCII digit, or with 'v' followed by an ASCII digit, with a leading 'v' removed; if no token matches, return "unknown". `pub fn claude_lists(stdout: &str, plugin: &str) -> bool`: stdout parses (serde_json) as a JSON array with an element whose "id" string starts with "<plugin>@". `pub fn codex_lists(stdout: &str, plugin: &str) -> bool`: stdout parses as a JSON object whose "installed" array has an element whose "pluginId" starts with "<plugin>@" or whose "name" equals plugin. Both return false on a parse error. `pub fn summary(word: &str, version: &str, deploy: &str, listed: &str, undeploy: &str) -> String` returns exactly `LIVE {word} {version} deploy={deploy} listed={listed} undeploy={undeploy}`.
6. In harness.rs add `pub struct RealHomeSentinel { home: PathBuf, seen: Vec<(String, bool)> }`. `capture()` reads the real HOME of the test process (std::env::var_os("HOME"); panic if it is unset) and records, for each probe, a bool: (i) for each of these home-relative skill dirs, whether symlink_metadata succeeds (the path exists): .claude/skills/live-skill, .agents/skills/live-skill, .config/opencode/skills/live-skill; (ii) for each of these home-relative client files, whether the file exists and its contents (read with std::fs::read, lossily as UTF-8) contain the substring "live-plugin": .claude/plugins/installed_plugins.json, .codex/config.toml, .config/opencode/opencode.json. A missing or unreadable file records false. The probe key is the home-relative path string. `assert_unchanged(&self)` re-captures and, on the first probe whose bool differs, panics with `isolation breach: <real home>/<path> changed (<before> -> <after>); a live-* entry reached the real home`. It never records timestamps or sizes, so a real claude/codex session running beside the test cannot trip it.
7. Create crates/vibe-cli/tests/cli_live_clients.rs with a //! header that names the run command, says the file is opt-in and never part of CI, and says which lanes are expected red today (copy the 'Expected today' decision). Include the harness with `#[path = "cli_live_clients/harness.rs"] mod harness;` and `use vibe_test_support::UserScratch;` (linking that crate isolates VIBE_SETTINGS; do not add `mod common`). Add `fn deploy_project(root: &Path, client: Client)`. It writes skills/live-skill/SKILL.md, plugin/plugin.json, plugin/skills/live-plugin-skill/SKILL.md and vibe.toml with the exact bytes in the decisions (manifest: replace every {client} with client.word()), creating parent dirs with create_dir_all. Add `fn plant_foreign(home: &Path, client: Client)`, which create_dir_all's `<home>/<skills_relative>/foreign-skill` and writes SKILL.md there = "foreign neighbour\n".
8. In cli_live_clients.rs add `fn listing(home: &IsolatedHome, client: Client, bin: &Path) -> Option<bool>` as an exhaustive `match client` with no `_` arm: Claude runs `plugin list --json` through client_command and returns Some(claude_lists(stdout, "live-plugin")); Codex runs `plugin list --json` and returns Some(codex_lists(stdout, "live-plugin")); OpenCode returns None (unverifiable). Every client-specific choice in run_deploy_lane goes through this function or another exhaustive match, never through if/else on the client.
9. In cli_live_clients.rs add `fn run_deploy_lane(client: Client)`, in this order. (1) Call locate; on Skip, eprintln and return. (2) Capture the sentinel, then create the IsolatedHome, a UserScratch and a project tempdir, and call deploy_project and plant_foreign. (3) Run `--version` through client_command and compute version_token(stdout). (4) Run `deploy --profile live --offline --path <project>` through vibe_command(bin.parent()). deploy=ok when the exit is 0; on failure eprintln vibe's stderr. (5) If deploy=ok: listed = match listing(...) { Some(true) => "ok", Some(false) => "fail", None => "unverifiable" }. If deploy failed: listed=not-run and undeploy=not-run. (6) If deploy=ok, run `undeploy --profile live --path <project>`. undeploy=ok when all of these hold: exit 0; listing(...) is not Some(true); `<home>/<skills_relative>/live-skill` does not exist; the foreign SKILL.md bytes are still "foreign neighbour\n". Otherwise undeploy=fail. Then println! the summary line, call sentinel.assert_unchanged(), and assert deploy=="ok", listed is "ok" or "unverifiable", and undeploy=="ok", with the summary line as the assert message. Add the tests live_deploy_claude, live_deploy_codex and live_deploy_opencode, each `#[test] #[ignore = "live: needs the real client binary — cargo test -p vibe-cli --test cli_live_clients -- --ignored --nocapture"]`. Add live_harness_reads_versions_and_listings (same ignore attribute) asserting: version_token("2.1.330 (Claude Code)\n")=="2.1.330"; version_token("codex-cli 0.167.0\n")=="0.167.0"; version_token("opencode v2.2.7\n")=="2.2.7"; version_token("no digits\n")=="unknown"; claude_lists(r#"[{"id":"live-plugin@vibevm-0fc6","scope":"user"}]"#, "live-plugin") is true; claude_lists("[]", "live-plugin") is false; codex_lists(r#"{"installed":[{"pluginId":"live-plugin@vibevm-1"}],"available":[]}"#, "live-plugin") is true; codex_lists(r#"{"installed":[],"available":[{"name":"live-plugin"}]}"#, "live-plugin") is false; summary("claude","2.1.330","ok","ok","ok")=="LIVE claude 2.1.330 deploy=ok listed=ok undeploy=ok".
10. Run `cargo clippy -p vibe-cli --tests -- -D warnings` and `cargo fmt --all`; wc -l both new files and keep each under 600 lines.

**Decisions already made.**
- Two files only: cli_live_clients.rs (fixtures and tests) and cli_live_clients/harness.rs (lookup, isolation, parsers, sentinel, summary), each under 600 lines (conform max_file_lines).
- Every test is #[ignore] with a reason string. There is no VIBE_LIVE_CLIENTS toggle, no CI wiring, and the file does not touch tools/self-check.sh.
- The override value `skip` deliberately skips a client. Any other override must be an absolute path to an executable file whose stem is the client's command word, because vibe itself locates the command word on PATH; the harness puts that file's directory first on vibe's PATH. The harness's executable test mirrors vibe's is_executable_file (is_file plus, on unix, a mode & 0o111 bit), so the harness and vibe pick the same binary.
- Windows: every client is SKIPPED, because vibe deploy's client_authority uses dirs::home_dir (dirs 5.0.1 win.rs: known_folder_profile), which ignores USERPROFILE, so the home cannot be isolated.
- Every client-specific choice is an exhaustive match on Client with no `_` arm, so a later variant cannot fall silently into an existing branch: the compiler names every place it must be handled.
- Isolation proof: RealHomeSentinel records, for the real home, whether each live-skill dir exists and whether each real client config file mentions "live-plugin", before and after. It does not compare timestamps or sizes, because a real Claude Code or Codex session running beside the test legitimately touches those files. There is no separate pre-deploy destination assertion, because the plan JSON carries only home-relative identities such as `home:.claude/skills/...`.
- Proof that the client sees the plugin: claude `plugin list --json` (array, `id` = `live-plugin@<marketplace>`) and codex `plugin list --json` (object with `installed`/`available`; both commands work in an isolated home today). OpenCode has no listing that works without its managed service, so it is listed=unverifiable and only exit codes, file absence and the neighbour are checked. A `--version` result alone never counts as a pass.
- The test FAILS (red) when deploy, listing or undeploy fails: such a failure is a product finding the maintainer files into BACKLOG.md. Never weaken the test or patch the provider inside this entry.
- Expected today on a maintainer run with real clients: claude green; codex and opencode red, because PluginClient::version_pair pins codex (0, 148) and opencode (1, 17) while the installed clients are codex-cli 0.167.x and opencode v2.2.x (probe_version refuses with ClientVersion); opencode is also red because client_environment (crates/vibe-lifecycle/src/process/mod.rs) does not pass PATH, so a `#!/usr/bin/env node` launcher cannot start. These reds are the product findings this lane exists to surface; they are not failures of this entry, and the executor must not change crates/vibe-lifecycle to turn them green. The entry's own gate is the `check` command (all SKIP paths plus the pure test).
- Manifest literal written by deploy_project (replace {client} with the client word; verified working for claude today):
```
[project]
name = "live-clients"
version = "0.1.0"

[[artifacts.package]]
id = "live-skill"
mechanism = "package:static-skill"
outputs = [{ id = "live-skill.md", kind = "skill" }]
config = { source = "skills/live-skill" }

[[artifacts.package]]
id = "live-plugin"
mechanism = "package:agent-plugin"
inputs = []
outputs = [{ id = "live-plugin.dir", kind = "agent-plugin" }]
config = { source = "plugin" }

[[artifacts.package]]
id = "p-{client}"
mechanism = "package:{client}-plugin"
inputs = [{ artifact = "live-plugin.dir" }]
outputs = [{ id = "p-{client}.dir", kind = "directory" }]
config = { components = ["skills"] }

[[deploy.target]]
id = "{client}-skill"
artifact = "live-skill.md"
mechanism = "deploy:{client}-skill"
config = { name = "live-skill" }

[[deploy.target]]
id = "{client}-plugin"
artifact = "p-{client}.dir"
mechanism = "deploy:{client}-plugin"
config = { name = "live-plugin" }

[deploy.profiles.live]
targets = ["{client}-skill", "{client}-plugin"]
```
- Fixture file bytes: skills/live-skill/SKILL.md = "---\nname: live-skill\ndescription: Live check skill.\n---\n\nBody.\n"; plugin/plugin.json = "{\n  \"name\": \"live-plugin\",\n  \"version\": \"0.1.0\",\n  \"description\": \"Live check plugin.\"\n}\n"; plugin/skills/live-plugin-skill/SKILL.md = "---\nname: live-plugin-skill\ndescription: Live check plugin skill.\n---\n\nBody.\n".
- The summary line is printed with println! and has exactly this form: `LIVE <word> <version> deploy=<ok|fail> listed=<ok|fail|unverifiable|not-run> undeploy=<ok|fail|not-run>`.
- The check uses two name filters, `live_deploy_` and `live_harness_`, rather than the broader `live_`, so a later `live_skill_install_*` test never changes this entry's count of 4.
- Clippy -D warnings applies to this test target: no unused items or imports. The only OS-specific import (PermissionsExt) sits inside a #[cfg(unix)] block.

**Not in this entry.** Not in this entry: the qwen skill lane (ADV-CLIENTS-LIVE-2); any change to crates/vibe-lifecycle or crates/vibe-cli/src. That includes the stale supported_version pins (codex 0.148.x, opencode 1.17.x), client_environment dropping PATH, and Windows home resolution. Also not here: filing those findings into BACKLOG.md (the maintainer does that after a real run), a fake client executable, CI wiring, and the 'verified client versions' table in the vibevm-docs deploy page (filled in by hand by the maintainer after a real run).

Entries that write the same file run one after another, never in parallel.
