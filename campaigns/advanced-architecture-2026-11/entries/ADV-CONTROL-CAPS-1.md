# ADV-CONTROL-CAPS-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The `cli-capabilities` format is registered with epoch 1, and vibe-wire has a generated `CapabilitiesReport` type that reads a capabilities document back without loss.

**Write.** `schemas/capabilities_report.jtd.json`, `formats/REGISTRY.toml`, `crates/vibe-wire/src/generated/**`, `crates/vibe-wire/tests/capabilities_report_wire.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-wire --test capabilities_report_wire`

**Test.** crates/vibe-wire/tests/capabilities_report_wire.rs: (1) `capabilities_report_round_trips`: parses the sample document from the decisions into `vibe_wire::generated::capabilities_report::CapabilitiesReport`, serialises it back to `serde_json::Value`, and asserts it equals the sample parsed as `Value`. It also asserts `report.platform.os == "linux"`, `report.commands[0].flags[0].name == "--plan-only"` and `report.formats[0].epoch == 1`. (2) `capabilities_format_is_registered_at_epoch_one`: asserts `FormatId::CliCapabilities.id() == "cli-capabilities"` and `FormatId::CliCapabilities.epoch() == 1`. Before the work the test target does not exist, so the command fails.

**Read first.**
- schemas/doc_surface.jtd.json:1-120
- formats/REGISTRY.toml:1-40
- formats/REGISTRY.toml:2120-2140
- crates/vibe-cli/tests/install_steps_wire.rs:1-60
- crates/vibe-wire/tests/format_registry_completeness.rs:1-60
- xtask/src/main.rs:1-30

**Steps.**
1. Create schemas/capabilities_report.jtd.json with exactly the schema text given in the decisions, pretty-printed with 2-space indent.
2. Append the registry record from the decisions to the end of formats/REGISTRY.toml.
3. Run `cargo xtask codegen` from the repository root. It needs the vendored tools/jtd-codegen/jtd-codegen binary, which is a Linux x86-64 executable, so run this step on Linux x86-64. It regenerates crates/vibe-wire/src/generated/ (it adds the module `capabilities_report`, the `FormatId::CliCapabilities` variant and its properties). Do not hand-edit generated files.
4. Run `git status --short`. If it shows changes outside schemas/capabilities_report.jtd.json, formats/REGISTRY.toml and crates/vibe-wire/src/generated/ (the new capabilities_report module, the format_id file, and the generated mod.rs), stop and report the list. Do not revert or edit those files.
5. Create crates/vibe-wire/tests/capabilities_report_wire.rs with the two tests named in `test`. Put the sample document in a `const SAMPLE: &str` inside the test file.
6. Run the check, then run `cargo test -p vibe-wire --test format_registry_completeness` once, to confirm that the registry and the enum still agree.

**Decisions already made.**
- Schema file text (write verbatim; backticks are plain backticks, there are no backslashes before them): {"metadata":{"description":"What one vibe executable supports, printed by `vibe capabilities --json`: its version and commit, its platform, every command with the flags it accepts, every registered format with its epoch, and a closed list of named features. A client turns behaviour on by feature, not by version number. Source of truth for `crates/vibe-wire/src/generated/capabilities_report/`.","spec":{"implements":"spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary"},"rustOptions":{"package":"vibe_wire::capabilities_report"}},"properties":{"protocol":{"type":"string"},"ok":{"type":"boolean"},"version":{"type":"string"},"commit":{"type":"string","metadata":{"description":"The source commit the binary was built from, empty when the build did not record one."}},"platform":{"ref":"capability_platform"},"commands":{"elements":{"ref":"capability_command"},"metadata":{"x-empty":"emit"}},"formats":{"elements":{"ref":"capability_format"},"metadata":{"x-empty":"emit"}},"features":{"elements":{"type":"string"},"metadata":{"x-empty":"emit"}}},"definitions":{"capability_platform":{"properties":{"os":{"type":"string"},"arch":{"type":"string"}}},"capability_command":{"properties":{"path":{"type":"string"},"flags":{"elements":{"ref":"capability_flag"},"metadata":{"x-empty":"emit"}}}},"capability_flag":{"properties":{"name":{"type":"string"},"value":{"type":"string"}}},"capability_format":{"properties":{"id":{"type":"string"},"epoch":{"type":"uint32"}}}}}
- Registry record (append verbatim, one key per line, the same layout as [format.cli-agent-activation]): [format.cli-capabilities] then epoch = 1, schema = "schemas/capabilities_report.jtd.json", recoverable = true, foreign_parsers = "many", unknown_fields = "allow", corpus = "none", sunset = "none". Put this one-line comment above it, written with plain backticks and no backslashes: # `vibe capabilities --json`: what this executable supports.
- Generated names: module `vibe_wire::generated::capabilities_report`, root type `CapabilitiesReport`, definitions `CapabilityPlatform`, `CapabilityCommand`, `CapabilityFlag`, `CapabilityFormat`; enum variant `FormatId::CliCapabilities`.
- Sample document: {"protocol":"vibe-capabilities/1","ok":true,"version":"1.0.12","commit":"","platform":{"os":"linux","arch":"x86_64"},"commands":[{"path":"vibe install","flags":[{"name":"--plan-only","value":""},{"name":"--prefer-hosting","value":"github|gitverse|host:HOST"}]}],"formats":[{"id":"cli-capabilities","epoch":1}],"features":["project-plan-only"]}
- No `error_codes` field: it arrives with the stable-codes capability, not here.
- If codegen touches files outside the new module, format_id and generated mod.rs, that is pre-existing drift: stop and report it. Do not fix it.

**Not in this entry.** Not in this entry: a corpus directory under formats/corpora, a FORMAT-CHANGES note, an `error_codes` field, any vibe-cli code, or repairing codegen drift in other generated trees (for example the specmap crate).

Entries that write the same file run one after another, never in parallel.
