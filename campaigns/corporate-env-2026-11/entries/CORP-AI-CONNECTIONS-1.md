# CORP-AI-CONNECTIONS-1

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Human-mode keyring list, details and references render as masked tables, not raw JSON

**Write.** `crates/vibe-cli/src/commands/secrets.rs`, `crates/vibe-cli/src/commands/secrets/human.rs`, `crates/vibe-cli/tests/cli_secrets_human.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_secrets_human human_list_details_and_references_render_masked_tables`

**Test.** crates/vibe-cli/tests/cli_secrets_human.rs :: human_list_details_and_references_render_masked_tables (#[cfg(unix)]). Imports: `use std::{io::Write, path::Path, process::{Command, Stdio}};` plus `std::os::unix::fs::PermissionsExt` for the 0o700 mode. Use a fixture with the same shape as the `Fixture` struct in crates/vibe-cli/tests/cli_secrets.rs: a 0o700 tempdir, env_clear, HOME and VIBE_SETTINGS set to the tempdir, PATH=/usr/bin:/bin, binary env!("CARGO_BIN_EXE_vibe"), args prefixed with `secrets`. Steps:
(a) `list` without --json on the empty store: exit 0, and stdout contains `Local keyring is empty.`
(b) `create --kind api-key --label Synthetic --value-stdin --json` with stdin `SYNTHETIC_HUMAN_SECRET_DO_NOT_PRINT`. Parse the id from items[0].id.
(c) `details <id>` without --json: exit 0. Stdout contains `store revision 1`, the id, `Synthetic`, `api-key` and `stored (hidden)`, and does not contain `"protocol"`.
(d) `list` without --json: stdout contains the id and `stored (hidden)`.
(e) `references <id>` without --json: stdout contains `No registered consumers reference <id>.` and does not contain `"protocol"`.
(f) The sentinel is absent from stdout and stderr of every run.
(g) `details <id> --json` still parses as JSON and has protocol `vibe-secrets/1`.

**Read first.**
- crates/vibe-cli/src/commands/secrets.rs
- crates/vibe-cli/src/output/language/table.rs
- crates/vibe-cli/tests/cli_secrets.rs
- crates/vibe-secrets/src/lib.rs

**Steps.**
1. Precondition: tools/cargo-test-ran.sh must exist in the checkout you work in (it is untracked until the lead commits it). If it is missing, stop and report a packet defect.
2. Create crates/vibe-cli/src/commands/secrets/human.rs. Its first two lines are exactly:
```
//! Human rendering of keyring metadata; value bytes never reach it.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
```
3. In human.rs add `pub(super) fn items(store_revision: u64, rows: &[(String, String, String, String)]) -> String`, where each tuple is (id, label, kind, current_revision). Empty rows return exactly `Local keyring is empty.` Otherwise start with `let mut table = crate::output::language::Table::new(vec!["ID".into(), "Label".into(), "Kind".into(), "Revision".into(), "Value".into()]);`, then for each tuple `table = table.row(vec![id.clone(), label.clone(), kind.clone(), revision.clone(), "stored (hidden)".into()]);` (Table is a by-value builder; there is no push/add_row), and return `format!("Local keyring — store revision {store_revision}\n{}", table.render())`.
4. In human.rs add `pub(super) fn references(id: &str, consumers: &[String]) -> String`. Empty consumers return exactly `No registered consumers reference {id}.` Otherwise return `Consumers of {id}:` followed by one line per consumer, each formatted `  {consumer}` (two leading spaces), joined with `\n`.
5. In secrets.rs add `mod human;` after the `use` block. In fn `render`, keep the `rows` JSON values, the NotFound check, the `report` and the `ctx.emit_json` branch unchanged. Inside the human `else` branch, replace `println!("{}", serde_json::to_string_pretty(&report)?);` with:
```
        let tuples: Vec<(String, String, String, String)> = snapshot
            .items
            .iter()
            .filter(|(id, _)| selected.is_none_or(|s| s == *id))
            .map(|(id, item)| {
                (
                    id.as_str().to_owned(),
                    item.label.clone(),
                    item.kind.clone(),
                    item.current_revision.as_str().to_owned(),
                )
            })
            .collect();
        println!("{}", human::items(snapshot.store_revision, &tuples));
```
(`snapshot` is only borrowed by the existing code, so it is still usable here.)
6. In fn `execute`, arm `SecretsCommand::References`, keep the JSON branch. Replace the human `println!("{}", serde_json::to_string_pretty(&report)?);` with `println!("{}", human::references(id.as_str(), &consumers));`.
7. Write the test file as described in `test`. Run `cargo fmt --all` and `cargo clippy -p vibe-cli --all-targets -- -D warnings` and fix every warning. Run `wc -l crates/vibe-cli/src/commands/secrets.rs crates/vibe-cli/src/commands/secrets/human.rs crates/vibe-cli/tests/cli_secrets_human.rs` and confirm each is at or below 600 lines.

**Decisions already made.**
- The JSON output and protocol strings are unchanged. Only the human (non --json) branch changes.
- Reuse the existing box-drawn `crate::output::language::Table` with its by-value `row` builder. Do not build any new widget.
- The value column is the literal `stored (hidden)`. No length, digest or prefix of the value is shown, and value bytes are never read for rendering.
- The interactive Details action keeps calling `render`, so it gets the table automatically.
- specmap.json is not regenerated in this entry; tools/self-check.sh does not run `cargo xtask specmap --check`, so the tracked index going stale fails no gate today.
- The test file is new and #[cfg(unix)], because the store requires a user-only directory.
- Executors run where tools/cargo-test-ran.sh exists: the primary checkout, or a worktree made after the lead commits that script.

**Not in this entry.** Not in this entry: human rendering for `vibe ai connections` or `vibe ai bindings`, colour or styling changes to Table, changes to the interactive menu, and regenerating specmap.json.

Entries that write the same file run one after another, never in parallel.
