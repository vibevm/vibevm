# ADV-SDD-BOOT-DELTA-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The project install:plan wire document can carry an optional `boot_delta` array of {lane, before, after} rows, where before or after is null for an absent lane, and it round-trips byte-exact through the generated InstallPlan type.

**Write.** `schemas/install/e2/plan.jtd.json`, `crates/vibe-wire/src/generated/install/e2/plan/mod.rs`, `crates/vibe-wire/tests/project_install_plan_corpus.rs`, `crates/vibe-cli/src/commands/install/preview.rs`, `crates/vibe-cli/src/commands/install/report.rs`, `crates/vibe-cli/src/commands/install/report/hostings.rs`

**Check.** `cargo check -p vibe-cli --all-targets && cargo test -p vibe-wire --test project_install_plan_corpus install_plan_boot_delta_rows_roundtrip 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-wire/tests/project_install_plan_corpus.rs :: install_plan_boot_delta_rows_roundtrip (new; the name filter matches 0 tests today, so the grep fails before the work)

**Read first.**
- schemas/install/e2/plan.jtd.json (top-level metadata x-wire-order, optionalProperties.hostings, definitions.remap_input_required)
- schemas/application_plan.jtd.json definition `byte_count` (the float64 + x-rust-type u64 precedent)
- crates/vibe-wire/tests/project_install_plan_corpus.rs fn observed_and_legacy_live_project_events_roundtrip
- tools/jtd-codegen/README.md (the generator binary is gitignored and must be present at tools/jtd-codegen/jtd-codegen, or jtd-codegen.exe on Windows)
- crates/vibe-cli/src/commands/install/preview.rs fn prepare (the InstallPlan literal at its end)
- crates/vibe-cli/src/commands/install/report.rs (the InstallPlan literal passed to hostings::with_hostings)

**Steps.**
1. Check that the generator binary exists at tools/jtd-codegen/jtd-codegen (jtd-codegen.exe on Windows). If it is missing, stop with a failure note naming tools/jtd-codegen/README.md. Do not hand-write the generated file.
2. In schemas/install/e2/plan.jtd.json, top-level metadata x-wire-order: after the element "hostings", add "boot_delta" as the last element.
3. In the same file's definitions object, after the `remap_input_required` definition, add the two definitions given in decisions: `boot_delta_row` and `byte_count`.
4. In the same file's top-level optionalProperties, after `hostings`, add the `boot_delta` member given in decisions.
5. Run `cargo xtask codegen` from the repository root. The only generated file that changes must be crates/vibe-wire/src/generated/install/e2/plan/mod.rs. It must now contain `pub boot_delta: Option<Vec<BootDeltaRow>>` on InstallPlan, a `pub struct BootDeltaRow` with fields `lane: String`, `before: Option<ByteCount>`, `after: Option<ByteCount>` (serde attributes come from the generator), and `pub type ByteCount = u64;`. Do not hand-edit the generated file.
6. Each InstallPlan struct literal in crates/vibe-cli/src/commands/install/preview.rs (fn prepare), report.rs and report/hostings.rs has the line `hostings: None,`. Directly after it, add `boot_delta: None,` with the same indentation. There are exactly three sites; `grep -rn "InstallPlan {" crates --include=*.rs` outside generated/ lists exactly these three.
7. Append the test given in decisions to crates/vibe-wire/tests/project_install_plan_corpus.rs. No new import is needed: InstallPlan is already imported.
8. Run `cargo fmt --all`. rustfmt may reflow the lines you added; that is expected. If it changes the generated file, stop with a failure note (the generator output is meant to be rustfmt-clean). Then run `cargo clippy -p vibe-wire -p vibe-cli --all-targets -- -D warnings` and fix only warnings in this entry's files.
9. Run `git status --short`: only the six files in write may be changed. Then run the check.

**Decisions already made.**
- Definitions to insert (JSON; keep the file's 2-space indentation):
```
"boot_delta_row": {
  "metadata": {
    "description": "One watched boot lane whose size the candidate install changes: sampled on the project now and on a scratch copy after the candidate install.",
    "x-wire-order": ["lane", "before", "after"]
  },
  "properties": {
    "lane": {"type": "string", "metadata": {"description": "Lane path relative to the workspace root, forward-slashed."}},
    "before": {"ref": "byte_count", "nullable": true, "metadata": {"description": "Size now; null when the lane does not exist."}},
    "after": {"ref": "byte_count", "nullable": true, "metadata": {"description": "Size after the candidate install; null when the lane would not exist."}}
  }
},
"byte_count": {
  "type": "float64",
  "metadata": {
    "description": "An exact non-negative byte count. JTD has no uint64 form, so generated Rust narrows the numeric form to u64.",
    "x-rust-type": "u64"
  }
}
```
- optionalProperties member to insert:
```
"boot_delta": {
  "elements": {"ref": "boot_delta_row"},
  "metadata": {
    "x-default": null,
    "x-empty": "preserve",
    "description": "Present only for install --plan-only --boot-delta: every watched boot lane whose size the candidate install changes, in watched-lane order; empty when no lane changes. Computing it may fill the ordinary package cache."
  }
}
```
- Test to append:
```
#[test]
fn install_plan_boot_delta_rows_roundtrip() {
    let text = r#"{"command":"install:plan","packages":[],"boot_delta":[{"lane":"vibevm/vibespecs/boot/INDEX.md","before":814,"after":963},{"lane":"vibevm/vibespecs/boot/STATIC.xml","before":null,"after":1976}]}"#;
    let parsed: InstallPlan = serde_json::from_str(text).unwrap();
    let rows = parsed.boot_delta.clone().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].before, Some(814));
    assert_eq!(rows[1].before, None);
    assert_eq!(rows[1].after, Some(1976));
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        serde_json::from_str::<serde_json::Value>(text).unwrap()
    );
}
```
- before/after are required-nullable rather than optional, so an absent lane serialises as null. That is the same shape as the `lanes` rows of the existing `install:closure-diff` document emitted by closure_diff::emit_closure_diff.
- The format epoch stays 2 (an additive optional field). formats/EPOCHS.toml has public = false, so wire-diff only reports. Do not add a break note or a corpus file.
- If `cargo xtask codegen` refuses the schema, stop with a failure note quoting its error. Do not change xtask.
- Order: this entry runs before ADV-SDD-BOOT-DELTA-3, which also writes preview.rs.

**Not in this entry.** Not in this entry: producing boot_delta from any command, a corpus file under formats/corpora, a change to the frozen v1 schema schemas/install_plan.jtd.json, or regenerating specmap.json.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
