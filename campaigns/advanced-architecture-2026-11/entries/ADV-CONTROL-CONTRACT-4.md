# ADV-CONTROL-CONTRACT-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe skill list --json`, `vibe skill install|uninstall --json` and the `install:closure-diff` document print registered formats (cli-skill-list-report, cli-skill-projection-report, cli-closure-diff-report), and the Library's invocations of them conform.

**Write.** `schemas/skill_list_report.jtd.json`, `schemas/skill_projection_report.jtd.json`, `schemas/closure_diff_report.jtd.json`, `formats/REGISTRY.toml`, `crates/vibe-wire/src/generated/**`, `crates/vibe-cli/tests/client_contract_skill_diff.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test client_contract_skill_diff`

**Test.** crates/vibe-cli/tests/client_contract_skill_diff.rs: library_skill_verbs_and_closure_diff_conform. On LibraryProject::new(): (a) call install_wal(), take the document whose command is "install:closure-diff", assert it conforms to vibe_wire::generated::closure_diff_report::ClosureDiffReport, and assert its "added" array is non-empty. (b) call install_wal() again, take its "install:closure-diff" document, assert it conforms and that "unchanged" == true. (c) run `skill list --path P`; it conforms to skill_list_report::SkillListReport with count == 1. (d) run `skill install --path P --agent claude --scope project --offline`, then `skill uninstall --path P --agent claude --scope project --offline`; both conform to skill_projection_report::SkillProjectionReport with count == 1. Before the entry the test does not compile, because the three generated modules do not exist.

**Read first.**
- crates/vibe-cli/src/commands/skill/mod.rs:70-120
- crates/vibe-agent-projection/src/pkgskill.rs:168-180
- crates/vibe-cli/src/commands/install/closure_diff.rs:178-220
- schemas/application_describe.jtd.json:1030-1040
- crates/vibe-cli/tests/common/library.rs

**Steps.**
1. Write the three schema files with the exact content given in the decisions, as 2-space-indented JSON with a trailing newline.
2. In formats/REGISTRY.toml, insert directly after the records added by ADV-CONTROL-CONTRACT-3 three records named cli-skill-list-report, cli-skill-projection-report and cli-closure-diff-report. Each has: epoch = 1, schema = "schemas/<file>.jtd.json", recoverable = true, foreign_parsers = "many", corpus = "none", sunset = "none". Each record gets a one-line `#` comment naming its command.
3. Run `cargo xtask codegen` (see the jtd-codegen prerequisite in the decisions; if it reports "jtd-codegen not found", stop and report), then `cargo xtask check-codegen` (it must print clean) and `cargo test -p vibe-wire --test format_registry_completeness`.
4. Create crates/vibe-cli/tests/client_contract_skill_diff.rs with `mod common;` and the test described in `test`.
5. Run the check and cargo fmt -p vibe-cli.

**Decisions already made.**
- Prerequisite provided by the lead, not by this entry: the jtd-codegen binary at tools/jtd-codegen/jtd-codegen (jtd-codegen.exe on Windows), installed per vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md, or jtd-codegen on PATH. tools/.gitignore ignores it. The executor does not install, copy or commit it.
- schemas/skill_list_report.jtd.json = {"metadata":{"description":"Wire format for `vibe skill list --json`: the skills the project and its installed packages declare.","spec":{"implements":"spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT"},"rustOptions":{"package":"vibe_wire::skill_list_report"}},"properties":{"ok":{"type":"boolean"},"command":{"type":"string","metadata":{"description":"Always `\"skill:list\"` for this report."}},"project":{"type":"string"},"count":{"type":"uint32"},"skills":{"elements":{"ref":"declared_skill"},"metadata":{"x-empty":"emit"}}},"definitions":{"declared_skill":{"properties":{"name":{"type":"string"},"package":{"type":"string"},"origin":{"type":"string"},"source":{"type":"string"},"description":{"type":"string","nullable":true},"agents":{"elements":{"type":"string"},"metadata":{"x-empty":"emit"}}}}}}
- schemas/skill_projection_report.jtd.json = {"metadata":{"description":"Wire format for `vibe skill install --json` and `vibe skill uninstall --json`: one row per skill, agent and scope.","spec":{"implements":"spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT"},"rustOptions":{"package":"vibe_wire::skill_projection_report"}},"properties":{"ok":{"type":"boolean"},"command":{"type":"string","metadata":{"description":"`skill:install` or `skill:uninstall`."}},"project":{"type":"string"},"count":{"type":"uint32"},"results":{"elements":{"ref":"skill_result"},"metadata":{"x-empty":"emit"}}},"definitions":{"skill_result":{"properties":{"skill":{"type":"string"},"agent":{"type":"string"},"scope":{"type":"string"},"path":{"type":"string","nullable":true},"status":{"type":"string"},"note":{"type":"string","nullable":true}}}}}
- schemas/closure_diff_report.jtd.json = {"metadata":{"description":"Wire format for the `<command>:closure-diff` document `vibe install --json` and `vibe update --json` print before their final report. An unchanged closure carries `unchanged` and `packages`; a changed one carries `added`, `removed`, `changed` and `lanes`.","spec":{"implements":"spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT"},"rustOptions":{"package":"vibe_wire::closure_diff_report"}},"properties":{"command":{"type":"string","metadata":{"description":"`<command>:closure-diff`, for example `install:closure-diff`."}}},"optionalProperties":{"unchanged":{"type":"boolean","metadata":{"x-default":null}},"packages":{"type":"uint32","metadata":{"x-default":null}},"added":{"elements":{"type":"string"},"metadata":{"x-empty":"preserve"}},"removed":{"elements":{"type":"string"},"metadata":{"x-empty":"preserve"}},"changed":{"elements":{"type":"string"},"metadata":{"x-empty":"preserve"}},"lanes":{"elements":{"ref":"lane"},"metadata":{"x-empty":"preserve"}}},"definitions":{"lane":{"properties":{"lane":{"type":"string"},"before":{"ref":"byte_count","nullable":true},"after":{"ref":"byte_count","nullable":true}}},"byte_count":{"type":"float64","metadata":{"description":"An exact non-negative byte count. JTD has no uint64 form, so generated Rust narrows the numeric form to u64.","x-rust-type":"u64"}}}}
- added, removed, changed and lanes use x-empty "preserve" because the changed arm writes empty arrays (for example "removed": []) and the unchanged arm writes none. byte_count follows the existing u64 pattern in schemas/application_describe.jtd.json.
- One format serves both skill install and skill uninstall, because both go through emit_reports in commands/skill/mod.rs.
- Codegen ran clean in a scratch copy with these schemas, and both closure-diff arms and all three skill outputs round-tripped.
- This entry runs after ADV-CONTROL-CONTRACT-3 only because both edit formats/REGISTRY.toml and regenerate crates/vibe-wire/src/generated/format_id. There is no code dependency.

**Not in this entry.** Not in this entry: making the skill install `path` a machine_json_path, switching the emitters to the generated types, installing jtd-codegen, or registering `skill enable`/`disable`/`status`, which already use cli-agent-activation.

**Depends on.** ADV-CONTROL-CONTRACT-1, ADV-CONTROL-CONTRACT-3

Entries that write the same file run one after another, never in parallel.
