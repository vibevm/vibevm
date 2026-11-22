# ADV-FIX-BIN-NEWEST-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe bin list` shows `built (debug)` / `built (release)` / `not built` per row, under `--json` prints one bin:list document whose `binaries[]` entries carry `profile` ("debug", "release" or null when not built), and PROP-025's BIN-LIST fact describes both.

**Write.** `crates/vibe-cli/src/commands/bin.rs`, `vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe commands::bin::tests::bin_list_reports_the_selected_profile`

**Test.** crates/vibe-cli/src/commands/bin.rs, `mod tests`, fn bin_list_reports_the_selected_profile. Uses the existing `declared("tool")` helper to get (dir, bin). (a) With no artifacts: `list_state(&bin) == "not built"`, `list_entry(&bin)["built"] == false`, `list_entry(&bin)["profile"].is_null()`. (b) Writes stub files at bin.debug_artifact() and bin.release_artifact() (create_dir_all parents), sets debug mtime UNIX_EPOCH+1_000_000s and release mtime UNIX_EPOCH+2_000_000s: `list_state(&bin) == "built (release)"`, `list_entry(&bin)["profile"] == "release"`, `["built"] == true`, `["path"] == bin.release_artifact().display().to_string()`, `["name"] == "site"`, `["package"] == "org.vibevm.doc/web"`. (c) Sets debug mtime UNIX_EPOCH+3_000_000s: `list_state(&bin) == "built (debug)"`, `list_entry(&bin)["profile"] == "debug"`. Before the change the filter matches zero tests.

**Read first.**
- crates/vibe-cli/src/commands/bin.rs:1-52
- crates/vibe-cli/src/commands/bin.rs:218-289
- crates/vibe-cli/src/commands/cache/mod.rs:58-97
- crates/vibe-workspace/src/bins.rs:151-280
- vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml:61-76

**Steps.**
1. In crates/vibe-cli/src/commands/bin.rs add `fn list_state(bin: &vibe_workspace::bins::DeclaredBinary) -> String`: if `bin.artifact().exists()` return `format!("built ({})", bin.selected_profile().as_str())`, else `"not built".to_string()`.
2. Add `fn list_entry(bin: &vibe_workspace::bins::DeclaredBinary) -> serde_json::Value` returning `serde_json::json!({"name": bin.decl.name, "package": bin.package, "built": built, "profile": profile, "path": bin.artifact().display().to_string(), "description": bin.decl.description})` where built = bin.artifact().exists() and profile = `Some(bin.selected_profile().as_str())` when built, else `None` (serialises to null).
3. In `run_list`, right after `inventory.finish();` and before the `bins.is_empty()` check, add: if `ctx.is_json()` then `ctx.emit_json(&serde_json::json!({"ok": true, "command": "bin:list", "count": bins.len(), "binaries": bins.iter().map(list_entry).collect::<Vec<_>>()}))?; return Ok(());`. In the human loop replace the `let state = if bin.artifact().exists() { "built" } else { "not built" };` block with `let state = list_state(bin);` and keep the tab-separated println otherwise unchanged.
4. Add the test `bin_list_reports_the_selected_profile` to the existing `mod tests` (add `use super::{list_entry, list_state};` beside `use super::refuse_app_dispatch;`), with a local `fn set_mtime(path: &std::path::Path, secs: u64)` using `fs::OpenOptions::new().write(true).open(path).unwrap().set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))).unwrap()`.
5. In vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml replace the three lines of the BIN-LIST element (lines 64-66, from `      <BIN-LIST fact="true" status="impl/done">` through `  artifact path.</BIN-LIST>`) with exactly the text in this entry's `spec_edit` field. Change nothing else in that file.

**Decisions already made.**
- JSON document shape (new; `vibe bin list` has no JSON output today): {"ok": true, "command": "bin:list", "count": N, "binaries": [{"name", "package", "built": bool, "profile": "debug"|"release"|null, "path": string, "description": string|null}]}, emitted through `ctx.emit_json` like `vibe cache list`.
- In JSON mode an empty binary set emits the document with count 0 and an empty array; the stderr 'no installed package declares a [[binary]]' line and the trailing 'N binaries declared.' stderr line are only printed in human mode.
- Human column text exactly: `built (debug)`, `built (release)`, `not built`; no other column changes.
- Helper names `list_state` and `list_entry`, private to commands/bin.rs.
- BIN-LIST keeps status="impl/done"; its new text is the record of the JSON contract and the profile column.
- `vibe bin exec` and `vibe bin path` are not edited: they already resolve through artifact() and get the new rule from ADV-FIX-BIN-NEWEST-1; exec prints no warning because its stdout/stderr belong to the child.
- Commit: `feat(bin): show the selected build profile in vibe bin list` with body stating the human state column now names debug/release, `--json` emits a bin:list document with a per-binary profile field, and PROP-025 BIN-LIST records both.

**Not in this entry.** Not in this entry: a `--profile` flag, a stale-artifact warning on `vibe bin exec`, an integration test that spawns the vibe binary against a fixture project, JSON output for `vibe bin path` / `vibe bin build`, or editing DISPATCH-FRESHEST or any other PROP-025 fact besides BIN-LIST.

**Depends on.** ADV-FIX-BIN-NEWEST-1

**Spec edit (apply as a step of this entry).**

File vibevm/vibespecs/modules/vibe-workspace/PROP-025-binary-delivery.xml, replace the BIN-LIST element (lines 64-66) with:
      <BIN-LIST fact="true" status="impl/done">`vibe bin list` — every `[[binary]]` declared by the project's
  installed packages, with build state (`built (debug)` / `built (release)` / `not built`,
  naming the profile dispatch selects, see DISPATCH-FRESHEST) and the description. Under
  `--json` it prints one document `{"ok": true, "command": "bin:list", "count": N,
  "binaries": [{"name", "package", "built", "profile", "path", "description"}]}` where
  `profile` is "debug", "release" or null when not built and `path` is the selected artifact path.</BIN-LIST>

Entries that write the same file run one after another, never in parallel.
