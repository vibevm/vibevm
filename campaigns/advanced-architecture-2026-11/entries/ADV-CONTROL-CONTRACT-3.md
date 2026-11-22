# ADV-CONTROL-CONTRACT-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe check --json`, `vibe cache add --json` and `vibe cache list --json` each print a document in a newly registered format (cli-check-report, cli-cache-add-report, cli-cache-list-report), and the Library's invocations of them conform.

**Write.** `schemas/check_report.jtd.json`, `schemas/cache_add_report.jtd.json`, `schemas/cache_list_report.jtd.json`, `formats/REGISTRY.toml`, `crates/vibe-wire/src/generated/**`, `crates/vibe-cli/tests/client_contract_check_cache.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test client_contract_check_cache`

**Test.** crates/vibe-cli/tests/client_contract_check_cache.rs: library_check_and_cache_verbs_conform. On LibraryProject::new(): (a) run `cache add org.vibevm.world/wal@=1.0.0 --path P`; it conforms to vibe_wire::generated::cache_add_report::CacheAddReport, and inserted == ["org.vibevm.world/wal@1.0.0"]. (b) run `cache list --offline`; it conforms to cache_list_report::CacheListReport with count == 1. (c) call install_wal(), then run `check --path P`; it conforms to check_report::CheckReport with ok == true. (d) delete P/vibevm/vibedeps/org.vibevm.world.wal (std::fs::remove_dir_all), then call LibraryProject::output with ["check","--path",P]. Assert a non-zero exit, that stdout conforms to CheckReport, that summary.error >= 1, and that at least one finding carries a "path" member. Before the entry the test does not compile, because the three generated modules do not exist.

**Read first.**
- crates/vibe-cli/src/commands/check.rs:60-145
- crates/vibe-cli/src/commands/cache/mod.rs:30-100
- crates/vibe-cli/src/commands/cache/add.rs:432-460
- formats/REGISTRY.toml:770-800
- schemas/list_report.jtd.json
- crates/vibe-cli/tests/common/library.rs

**Steps.**
1. Write the three schema files with the exact content given in the decisions, as 2-space-indented JSON with a trailing newline.
2. In formats/REGISTRY.toml, insert directly after the [format.cli-uninstall-report] record three records named cli-check-report, cli-cache-add-report and cli-cache-list-report. Each has: epoch = 1, schema = "schemas/<file>.jtd.json", recoverable = true, foreign_parsers = "many", corpus = "none", sunset = "none". Each record gets a one-line `#` comment naming its command.
3. Run `cargo xtask codegen` (see the jtd-codegen prerequisite in the decisions; if it reports "jtd-codegen not found", stop and report). Expected changes: new modules check_report, cache_add_report and cache_list_report, plus generated/mod.rs, generated/format_id/mod.rs and generated/format_id/properties.rs. Then `cargo xtask check-codegen` must print clean, and `cargo test -p vibe-wire --test format_registry_completeness` must pass.
4. Create crates/vibe-cli/tests/client_contract_check_cache.rs with `mod common;` and the test described in `test`.
5. Run the check and cargo fmt -p vibe-cli.

**Decisions already made.**
- Prerequisite provided by the lead, not by this entry: the jtd-codegen binary at tools/jtd-codegen/jtd-codegen (jtd-codegen.exe on Windows), installed per vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md, or jtd-codegen on PATH. tools/.gitignore ignores it. The executor does not install, copy or commit it.
- schemas/check_report.jtd.json = {"metadata":{"description":"Wire format for `vibe check --json`: the project's consistency findings and their counts, printed on stdout whether or not the check found errors.","spec":{"implements":"spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT"},"rustOptions":{"package":"vibe_wire::check_report"}},"properties":{"ok":{"type":"boolean"},"command":{"type":"string","metadata":{"description":"Always `\"check\"` for this report."}},"project":{"type":"string"},"summary":{"ref":"summary"},"findings":{"elements":{"ref":"finding"},"metadata":{"x-empty":"emit"}}},"definitions":{"summary":{"properties":{"error":{"type":"uint32"},"warning":{"type":"uint32"},"info":{"type":"uint32"}}},"finding":{"properties":{"check":{"type":"string"},"severity":{"type":"string"},"message":{"type":"string"}},"optionalProperties":{"path":{"type":"string","metadata":{"x-default":null}},"line":{"type":"uint32","metadata":{"x-default":null}},"column":{"type":"uint32","metadata":{"x-default":null}},"field":{"type":"string","metadata":{"x-default":null}},"package":{"type":"string","metadata":{"x-default":null}}}}}}
- schemas/cache_add_report.jtd.json = {"metadata":{"description":"Wire format for `vibe cache add --json`: what the pre-warm put into the machine store.","spec":{"implements":"spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT"},"rustOptions":{"package":"vibe_wire::cache_add_report"}},"properties":{"ok":{"type":"boolean"},"command":{"type":"string","metadata":{"description":"Always `\"cache:add\"` for this report."}},"store":{"type":"string"},"source":{"type":"string","metadata":{"description":"`project` when the registries came from a project, `user` otherwise."}},"inserted":{"elements":{"type":"string"},"metadata":{"x-empty":"emit"}},"already_present":{"elements":{"type":"string"},"metadata":{"x-empty":"emit"}},"unwarmed_subjects":{"elements":{"ref":"unwarmed_subject"},"metadata":{"x-empty":"emit"}},"count":{"type":"uint32"}},"definitions":{"unwarmed_subject":{"properties":{"package":{"type":"string"},"reason":{"type":"string"}}}}}
- schemas/cache_list_report.jtd.json = {"metadata":{"description":"Wire format for `vibe cache list --json`: every package version the machine store holds.","spec":{"implements":"spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT"},"rustOptions":{"package":"vibe_wire::cache_list_report"}},"properties":{"ok":{"type":"boolean"},"command":{"type":"string","metadata":{"description":"Always `\"cache:list\"` for this report."}},"root":{"type":"string"},"count":{"type":"uint32"},"packages":{"elements":{"ref":"cached_package"},"metadata":{"x-empty":"emit"}}},"definitions":{"cached_package":{"properties":{"group":{"type":"string"},"name":{"type":"string"},"version":{"type":"string"}}}}}
- No enums in these schemas. Severity, source and command stay plain strings, so no x-vocabulary is needed.
- With these three schemas, codegen ran clean in a scratch copy, and real outputs round-tripped, including a failing check with findings.
- A failing `vibe check --json` prints the report on stdout and an error envelope on stderr, with exit 1. The test asserts only the stdout document and the non-zero exit.

**Not in this entry.** Not in this entry: switching the check and cache emitters to the generated types, adding corpora under formats/corpora, installing jtd-codegen, or registering `cache path`, `cache clean`, `cache check` or `cache import`.

**Depends on.** ADV-CONTROL-CONTRACT-1

Entries that write the same file run one after another, never in parallel.
