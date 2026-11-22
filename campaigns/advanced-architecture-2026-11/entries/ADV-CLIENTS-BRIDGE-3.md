# ADV-CLIENTS-BRIDGE-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe skill install` prints `warning: skill <name> (<file>) references <ref>, which is not shipped` once per unresolved reference, exits 0, and its --json report carries an `unresolved_references` array.

**Write.** `crates/vibe-cli/src/commands/skill/mod.rs`, `crates/vibe-cli/tests/cli_skill.rs`

**Check.** `cargo test -p vibe-cli --test cli_skill 2>&1 | grep -F 'test result: ok. 4 passed'`

**Test.** crates/vibe-cli/tests/cli_skill.rs: new test `install_warns_once_about_an_unshipped_script_reference_and_reports_it_in_json`. The file has 3 tests today, so the check's grep for `4 passed` fails before the work.

**Read first.**
- crates/vibe-cli/src/commands/skill/mod.rs: fn run_install, fn run_uninstall, fn emit_reports (`fn emit_reports(ctx: &output::Context, command: &str, project_root: &Path, reports: &[PackageSkillReport]) -> Result<()>`), fn render
- crates/vibe-cli/src/output/lines.rs: Context::step (prints `  {text}` to stdout; silent in JSON and quiet modes)
- crates/vibe-cli/tests/cli_skill.rs: fn project, fn run_json, test list_and_project_dry_run_keep_the_exact_json_contract_and_library_parity
- crates/vibe-agent-projection/src/pkgskill/projection/closure.rs: struct UnresolvedReference and DeclaredSkillProjection::unresolved_references (added by ADV-CLIENTS-BRIDGE-2)

**Steps.**
1. In crates/vibe-cli/src/commands/skill/mod.rs add `UnresolvedReference` to the `use vibe_agent_projection::pkgskill::{...}` import. Give fn emit_reports a fifth parameter `unresolved: Option<&[UnresolvedReference]>`. Inside the `if ctx.is_json()` branch, bind the existing `serde_json::json!({...})` object to `let mut document`, and when `unresolved` is `Some(rows)` set `document["unresolved_references"] = serde_json::to_value(rows)?;` before calling `ctx.emit_json(&document)?`.
2. In fn run_install, right after `let plan = prepare_declared_skill_projection(&project_root, &filter, scope)?;` add `let unresolved = plan.unresolved_references()?;`. After the loop that renders `previews`, add a loop over `&unresolved` that calls `ctx.step(&format!("warning: skill {} ({}) references {}, which is not shipped", r.skill, r.file, r.reference));`. Pass `Some(&unresolved)` to both emit_reports calls in run_install and `None` to both emit_reports calls in run_uninstall.
3. In crates/vibe-cli/tests/cli_skill.rs, test list_and_project_dry_run_keep_the_exact_json_contract_and_library_parity: add `"unresolved_references": [],` to the expected `skill:install` JSON object (the fixture references nothing). Change nothing else in that test.
4. Add test `install_warns_once_about_an_unshipped_script_reference_and_reports_it_in_json`: build a tempdir project with the same vibe.toml text as fn project except that the [[skill]] table has no `include` line (name "demo", path "skills/demo", description "Demo skill"); write skills/demo/SKILL.md = "Run `scripts/run.sh` and `scripts/missing.sh`.\n" and skills/demo/scripts/run.sh = "#!/bin/sh\n". (1) `let value = run_json(&user, &["install", "--agent", "claude", "--scope", "project", "--dry-run"], project.path());` and assert `value["unresolved_references"] == json!([{"skill": "demo", "file": "SKILL.md", "reference": "scripts/missing.sh"}])`. (2) Run human mode with `user.vibe().args(["skill", "install", "--agent", "claude", "--scope", "project", "--yes", "--path"]).arg(project.path()).output().unwrap()`, assert `status.success()`, assert stdout contains `warning: skill demo (SKILL.md) references scripts/missing.sh, which is not shipped` exactly once (`matches(..).count() == 1`), and assert no stdout line containing `warning:` contains `scripts/run.sh`.
5. Run `cargo fmt -p vibe-cli`, `cargo clippy -p vibe-cli --all-targets -- -D warnings`, then the check.

**Decisions already made.**
- The warning is a human stdout line printed through Context::step once, right after the preview lines, so it also appears on --dry-run and before the confirmation prompt; it is not repeated after apply.
- JSON mode prints no warning text (successful JSON commands must keep stderr empty, which run_json asserts); the data is the `unresolved_references` array of `{skill, file, reference}` objects.
- `unresolved_references` is always present in `skill:install` JSON (an empty array when nothing dangles) and absent from `skill:uninstall` JSON.
- Exit code and result rows are unchanged by warnings.

**Not in this entry.** Not in this entry: a --strict flag or nonzero exit for dangling references, warnings in `vibe skill enable/status` or MCP tools, an offline Spec Kit acceptance run (it needs the external bridge repository).

**Depends on.** ADV-CLIENTS-BRIDGE-2

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
