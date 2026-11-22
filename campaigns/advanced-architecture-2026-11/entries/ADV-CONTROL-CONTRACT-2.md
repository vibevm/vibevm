# ADV-CONTROL-CONTRACT-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe list --json` output conforms to its registered format cli-list-report, because the schema now declares the members the command really prints (bridge, authors, via_redirect, embedded_sources, features, subskills_active, describes, language).

**Write.** `schemas/list_report.jtd.json`, `crates/vibe-wire/src/generated/list_report/mod.rs`, `crates/vibe-cli/tests/client_contract_list.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test client_contract_list`

**Test.** crates/vibe-cli/tests/client_contract_list.rs: library_project_list_conforms. On LibraryProject::new() it calls install_wal(), then runs `list --path P --offline` through LibraryProject::run. It asserts that exactly one document is printed, that it passes common::wire::conforms::<vibe_wire::generated::list_report::ListReport>, and that packages[0]["bridge"] == false. Before the schema change it fails: conforms panics with "the document and its registered wire type disagree", and the differing member is packages (bridge and authors are lost on the way back).

**Read first.**
- schemas/list_report.jtd.json
- crates/vibe-cli/src/commands/list.rs:38-160
- crates/vibe-wire/src/generated/list_report/mod.rs
- crates/vibe-cli/tests/common/library.rs
- xtask/src/codegen/mod.rs:56-81

**Steps.**
1. In schemas/list_report.jtd.json, add to definitions.list_entry.properties the member "bridge": {"type":"boolean","metadata":{"description":"True iff the package is a bridge package."}}.
2. Add to definitions.list_entry.optionalProperties exactly these members: {"authors":{"elements":{"type":"string"},"metadata":{"x-empty":"omit","description":"The package's declared authors; absent when it declares none."}},"via_redirect":{"type":"string","metadata":{"x-default":null}},"embedded_sources":{"elements":{"ref":"embedded_source"},"metadata":{"x-empty":"omit"}},"features":{"elements":{"type":"string"},"metadata":{"x-empty":"omit"}},"subskills_active":{"elements":{"ref":"locked_subskill"},"metadata":{"x-empty":"omit"}},"describes":{"type":"string","metadata":{"x-default":null}},"language":{"type":"string","metadata":{"x-default":null}}}.
3. Add to definitions: {"embedded_source":{"properties":{"name":{"type":"string"},"kind":{"type":"string"},"source_url":{"type":"string"},"resolved_commit":{"type":"string"},"tree_oid":{"type":"string"},"content_hash":{"type":"string"},"upstream_authors":{"elements":{"type":"string"},"metadata":{"x-empty":"emit"}},"upstream_license":{"type":"string"},"license_path":{"type":"string"},"license_url":{"type":"string"},"license_file_sha256":{"type":"string"}},"optionalProperties":{"source_ref":{"type":"string","metadata":{"x-default":null}}}},"locked_subskill":{"properties":{"path":{"type":"string"},"delivery":{"type":"string"}},"optionalProperties":{"describes":{"type":"string","metadata":{"x-default":null}}}}}. Keep the file's existing formatting: 2-space indent, other lines untouched.
4. Run `cargo xtask codegen` (see the jtd-codegen prerequisite in the decisions; if it reports "jtd-codegen not found", stop and report). Only crates/vibe-wire/src/generated/list_report/mod.rs may change; then `cargo xtask check-codegen` must print clean.
5. Create crates/vibe-cli/tests/client_contract_list.rs with `mod common;` and the test described in `test`.
6. Run the check and cargo fmt -p vibe-cli.

**Decisions already made.**
- Prerequisite provided by the lead, not by this entry: the jtd-codegen binary at tools/jtd-codegen/jtd-codegen (jtd-codegen.exe on Windows), installed per vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md, or jtd-codegen on PATH (xtask find_jtd_codegen looks there next). tools/.gitignore ignores it, so it never appears in a diff. The executor does not install, copy or commit it.
- Fix the schema, not the emitter: the emitter's members are real data a client reads, and no Rust code consumes vibe_wire::generated::list_report (verified by grep).
- bridge is a required member because the emitter always writes it. An optional boolean with x-default false would drop `false` on the way back and fail the round trip.
- The format's epoch stays 1. The break window is open (formats/EPOCHS.toml break_window_open = true, public = false), and the format is recoverable output, never stored.
- The test uses the ADV-CONTROL-CONTRACT-1 fixture and oracle unchanged.

**Not in this entry.** Not in this entry: changing crates/vibe-cli/src/commands/list.rs, installing jtd-codegen, or the project-scoped registry sync schema, whose refreshed entries also drift.

**Depends on.** ADV-CONTROL-CONTRACT-1

Entries that write the same file run one after another, never in parallel.
