# ADV-FIX-CONVFB-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The cli-install-report wire type parses and re-emits an optional conversion_fallbacks array of {package, version, file, kept_format (markdown|xml), reason} rows, and the CLI's InstallReport literal still compiles with an empty list.

**Write.** `schemas/install_report.jtd.json`, `crates/vibe-wire/src/generated/**`, `crates/vibe-wire/tests/install_report_wire.rs`, `crates/vibe-cli/src/commands/install/draft.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-wire --test install_report_wire conversion_fallbacks && cargo check -p vibe-cli --tests`

**Test.** crates/vibe-wire/tests/install_report_wire.rs, new #[test] fn conversion_fallbacks_round_trip. It builds a document with with_root_facts(json!({"command":"install","ok":true,"project":"/p","conversion_fallbacks":[{"package":"org.acme/pkg","version":"1.2.0","file":"vibevm/vibespecs/PROP-001.md","kept_format":"markdown","reason":"line 5: twice"}]})) and deserialises it to InstallReport. It asserts conversion_fallbacks.len()==1, [0].kept_format == InstallConversionKeptFormat::Markdown and [0].file == "vibevm/vibespecs/PROP-001.md", and that serde_json::to_value of the report contains the same conversion_fallbacks array. It also asserts that a report whose conversion_fallbacks is empty serialises without the key. The second half of the check (cargo check -p vibe-cli --tests) proves the workspace still compiles.

**Read first.**
- schemas/install_report.jtd.json
- crates/vibe-wire/tests/install_report_wire.rs:1-80,100-125,185-215
- crates/vibe-wire/src/generated/install_report/mod.rs:1-120
- crates/vibe-cli/src/commands/install/draft.rs:100-140
- tools/jtd-codegen/README.md

**Steps.**
1. schemas/install_report.jtd.json: add optionalProperties.conversion_fallbacks = {"elements":{"ref":"install_conversion_fallback"},"metadata":{"x-empty":"omit","description":"Spec documents a transformed install (spec_format xml or markdown) kept in their source format because conversion failed, one row per file, in materialisation order. A slot skipped as already current reports no rows."}}. Add definitions.install_conversion_fallback = {"metadata":{"x-wire-order":["package","version","file","kept_format","reason"],"description":"One unconverted spec document: the package coordinate, its version, the slot-relative file, the format it was kept in, and the converter's error."},"properties":{"package":{"type":"string"},"version":{"type":"string"},"file":{"type":"string"},"kept_format":{"ref":"install_conversion_kept_format"},"reason":{"type":"string"}}}
2. Add definitions.install_conversion_kept_format = {"metadata":{"x-vocabulary":"closed","description":"The format an unconverted spec document was kept in."},"enum":["markdown","xml"]}
3. Run `cargo xtask codegen` from the repository root (the binary is tools/jtd-codegen/jtd-codegen) and commit the regenerated files under crates/vibe-wire/src/generated/. Never hand-edit them. The generated InstallReport gains a field `conversion_fallbacks: Vec<InstallConversionFallback>` and has no Default, so every literal must name it.
4. Compile fixes, not assertion changes: add `conversion_fallbacks: Vec::new(),` to the two existing InstallReport literals in crates/vibe-wire/tests/install_report_wire.rs (`let bare = InstallReport {` near line 105 and `let report = InstallReport {` near line 192), and to the InstallReport literal in crates/vibe-cli/src/commands/install/draft.rs InstallDraft::into_report (near line 106). ADV-FIX-CONVFB-4 fills the draft.rs value.
5. Write the new test and run both commands of the check.
6. Run `cargo xtask specmap` (the schema definitions are indexed with line numbers) and commit specmap.json.

**Decisions already made.**
- The JSON member is conversion_fallbacks, optional and omitted when empty (x-empty omit), so existing documents and parsers are unaffected and the format stays at epoch 1
- Generated type names come from the definitions install_conversion_fallback and install_conversion_kept_format (expected InstallConversionFallback and InstallConversionKeptFormat::{Markdown, Xml})
- kept_format wire values: "markdown" | "xml"
- Only install_report changes in this entry
- The only CLI change is `conversion_fallbacks: Vec::new(),` in the into_report literal, so vibe-cli keeps compiling even if ADV-FIX-CONVFB-4 is delayed
- Adding the new field to the two existing wire-test literals is a compile fix; their assertions are not changed

**Not in this entry.** Adding the same member to update_report or reinstall_report, and any other CLI code (mapping real rows into the report or rendering them belongs to ADV-FIX-CONVFB-4).

Entries that write the same file run one after another, never in parallel.
