# ADV-SDD-SPEC-SECTION-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe --json show section` with a refused address exits 1 and prints on stderr a JSON error object with `ok: false` and a `kind` that names the cause (e.g. `not-installed`, `anchor-not-found`).

**Write.** `crates/vibe-cli/src/output/structured.rs`, `crates/vibe-cli/tests/cli_show_section_refusal.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_show_section_refusal`

**Test.** crates/vibe-cli/tests/cli_show_section_refusal.rs (new; header `mod common;\n\nuse common::UserScratch;\nuse serde_json::Value;`). Use the same OWN.md fixture as entry 3: vibe.toml `[project]\nname = "host"\ngroup = "org.demo"\nversion = "0.1.0"\n` and `vibevm/vibespecs/common/OWN.md` = `# Own {#root}\n\n## Commits {#commits}\n\nCOMMIT_RULES\n`. Commands are `UserScratch::new().vibe()` with `.env_remove("VIBE_UNATTENDED").env_remove("VIBE_INVOKED_BY")`. A helper parses, as JSON, the last stderr line that starts with '{'. Tests: (1) unknown_package_refusal_carries_kind: `vibe --json show section spec://no.such/pkg/X#a --path <dir>` → status.code()==Some(1); JSON ok==false, kind=="not-installed", error contains "no.such/pkg". (2) missing_anchor_refusal_carries_kind: `spec://org.demo/host/common/OWN#nosuch` → status.code()==Some(1); kind=="anchor-not-found".

**Read first.**
- crates/vibe-cli/src/output/structured.rs:95-150
- crates/vibe-cli/src/output/lines.rs:139-176
- crates/vibe-workspace/src/spec_section.rs
- crates/vibe-cli/tests/cli_show_section.rs

**Steps.**
1. Insert at the top of `stamp_structured_error` in crates/vibe-cli/src/output/structured.rs: `if let Some(refusal) = err.downcast_ref::<vibe_workspace::spec_section::SectionRefusal>() && let Value::Object(map) = payload { map.insert("kind".into(), Value::String(refusal.kind.as_str().into())); }`.
2. Write crates/vibe-cli/tests/cli_show_section_refusal.rs with the two tests named in `test`, then run the check.

**Decisions already made.**
- Field name: `kind`, with the kebab-case value from RefusalKind::as_str. The existing `ok` and `error` fields of the JSON error envelope are unchanged.
- The envelope stays on stderr like every other --json error, and the exit code stays 1.
- No `code` or `remedy` fields are added here.

**Not in this entry.** The general stable-code/remedy envelope for all machine failures (a separate ADV-CONTROL capability), any exit-code mapping in exit_code.rs, or regenerating specmap.json (entry 5).

**Depends on.** ADV-SDD-SPEC-SECTION-1, ADV-SDD-SPEC-SECTION-3

Entries that write the same file run one after another, never in parallel.
