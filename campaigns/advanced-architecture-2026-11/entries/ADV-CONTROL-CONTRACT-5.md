# ADV-CONTROL-CONTRACT-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe registry sync -g --json` prints a document in a newly registered format, cli-registry-sync-home-report, that carries scope, home, and refreshed registries with their clones, and the document conforms.

**Write.** `schemas/registry_sync_home_report.jtd.json`, `formats/REGISTRY.toml`, `crates/vibe-wire/src/generated/**`, `crates/vibe-cli/tests/cli_registry_sync.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_registry_sync sync_g_report_conforms_to_its_registered_format`

**Test.** crates/vibe-cli/tests/cli_registry_sync.rs: new test sync_g_report_conforms_to_its_registered_format. It returns early with the eprintln skip message when !git_available(), as the neighbouring tests do. It builds Home::new() and calls home.release_1_1_0(), then runs home.vibe(&["--json","registry","sync","-g"]) and parses it with the file's json(). It asserts that common::wire::conforms::<vibe_wire::generated::registry_sync_home_report::RegistrySyncHomeReport>(&report) passes, that report["refreshed"][0]["clones"] is a non-empty array, and that report["scope"] == "user". Before the entry the test does not compile, because the generated module does not exist.

**Read first.**
- crates/vibe-cli/src/commands/registry/sync_home.rs:20-60
- crates/vibe-cli/src/commands/registry/sync.rs:20-45
- crates/vibe-cli/tests/cli_registry_sync.rs:1-135
- crates/vibe-cli/tests/common/wire.rs

**Steps.**
1. Write schemas/registry_sync_home_report.jtd.json with the exact content given in the decisions, as 2-space-indented JSON with a trailing newline.
2. In formats/REGISTRY.toml, insert directly after the [format.cli-registry-sync-report] record a record [format.cli-registry-sync-home-report] with: epoch = 1, schema = "schemas/registry_sync_home_report.jtd.json", recoverable = true, foreign_parsers = "many", corpus = "none", sunset = "none". Put a one-line comment above it: `# vibe registry sync -g --json`: the user home's refresh.
3. Run `cargo xtask codegen` (see the jtd-codegen prerequisite in the decisions; if it reports "jtd-codegen not found", stop and report), then `cargo xtask check-codegen` (it must print clean) and `cargo test -p vibe-wire --test format_registry_completeness`.
4. Append the test described in `test` to crates/vibe-cli/tests/cli_registry_sync.rs.
5. Run the check and cargo fmt -p vibe-cli.

**Decisions already made.**
- Prerequisite provided by the lead, not by this entry: the jtd-codegen binary at tools/jtd-codegen/jtd-codegen (jtd-codegen.exe on Windows), installed per vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md, or jtd-codegen on PATH. tools/.gitignore ignores it. The executor does not install, copy or commit it.
- schemas/registry_sync_home_report.jtd.json = {"metadata":{"description":"Wire format for `vibe registry sync -g --json`: the user home's registries that were refreshed or left alone.","spec":{"implements":"spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT"},"rustOptions":{"package":"vibe_wire::registry_sync_home_report"}},"properties":{"ok":{"type":"boolean"},"command":{"type":"string","metadata":{"description":"Always `\"registry:sync\"` for this report."}},"scope":{"type":"string","metadata":{"description":"Always `\"user\"` for this report."}},"home":{"type":"string"},"refreshed":{"elements":{"ref":"refreshed_registry"},"metadata":{"x-empty":"emit"}},"skipped":{"elements":{"ref":"skipped_home_entry"},"metadata":{"x-empty":"emit"}}},"definitions":{"refreshed_registry":{"properties":{"registry":{"type":"string"},"ref":{"type":"string"},"via":{"type":"string","metadata":{"description":"`index`, `git` or `repository`."}},"clones":{"elements":{"ref":"refreshed_clone"},"metadata":{"x-empty":"emit"}}}},"refreshed_clone":{"properties":{"group":{"type":"string"},"name":{"type":"string"},"via":{"type":"string"},"ref":{"type":"string"}}},"skipped_home_entry":{"properties":{"registry":{"type":"string"},"reason":{"type":"string"}},"optionalProperties":{"group":{"type":"string","metadata":{"x-default":null}},"name":{"type":"string","metadata":{"x-default":null}}}}}}
- This is a separate format, not a union with cli-registry-sync-report: the -g report has a different shape (registry, ref, via, clones), and a client knows which one it asked for.
- The existing Home fixture of cli_registry_sync.rs gives a non-empty refreshed[0].clones. Its real output round-tripped through this schema in a scratch codegen copy.
- This entry runs after ADV-CONTROL-CONTRACT-4 only because both edit formats/REGISTRY.toml and regenerate format_id. It needs common::wire::conforms from ADV-CONTROL-CONTRACT-1.

**Not in this entry.** Not in this entry: fixing the project-scoped `registry sync --json` schema (its emitted entries carry `group` where schemas/registry_sync_report.jtd.json says `kind`), changing sync_home.rs, or installing jtd-codegen.

**Depends on.** ADV-CONTROL-CONTRACT-1, ADV-CONTROL-CONTRACT-4

Entries that write the same file run one after another, never in parallel.
