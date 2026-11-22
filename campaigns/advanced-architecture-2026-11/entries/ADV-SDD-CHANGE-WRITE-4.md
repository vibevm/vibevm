# ADV-SDD-CHANGE-WRITE-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** After the origin package is upgraded, `vibe change rebase <id>` reports each cited document as `changed` and writes nothing; `vibe change rebase <id> --apply` moves the pin and captured digests, keeps comments and both role documents byte-identical, and a following dry run reports `unchanged`.

**Write.** `crates/vibe-cli/src/cli/change.rs`, `crates/vibe-cli/src/cli/change/write.rs`, `crates/vibe-cli/src/commands/change/mod.rs`, `crates/vibe-cli/src/commands/change/rebase.rs`, `crates/vibe-cli/tests/cli_change_rebase.rs`, `vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml`

**Check.** `cargo test -p vibe-cli --test cli_change_rebase 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-cli/tests/cli_change_rebase.rs: `rebase_dry_run_reports_changed_and_writes_nothing`, `rebase_apply_moves_pins_and_keeps_role_documents`

**Read first.**
- crates/vibe-cli/src/commands/change/new.rs and crates/vibe-cli/src/commands/change/home.rs (change_home, ChangeHome; from ADV-SDD-CHANGE-WRITE-3)
- crates/vibe-core/src/manifest/change/rebase.rs (plan_origin_rebase, OriginRebase, ChangeRebasePlan; from ADV-SDD-CHANGE-WRITE-2)
- crates/vibe-requirements/src/change/capture.rs (capture_origin, OriginCapture)
- crates/vibe-requirements/src/change/read.rs (read_saved_change, SavedChange.manifest, SavedChange.manifest_bytes)
- crates/vibe-cli/tests/change_support/mod.rs (ChangeWorld: installed, new_change, upgrade, vibe, change_dir, slot_sha, snapshot)

**Steps.**
1. cli/change/write.rs: add `#[derive(Debug, clap::Args)] pub struct ChangeRebaseArgs` with `pub id: String` (doc `The change id.`), `#[arg(long)] pub apply: bool` (doc `Rewrite the origin captures to the installed packages.`) and `#[arg(long, default_value = ".")] pub path: PathBuf` (doc `Project root; defaults to the current directory.`). In cli/change.rs add to ChangeSubcommand `Rebase(ChangeRebaseArgs)` with doc `/// Compare origin captures with the installed packages; --apply moves them.` In commands/change/mod.rs add `mod rebase;` and the arm `ChangeSubcommand::Rebase(args) => rebase::run(ctx, args),`.
2. Create rebase.rs (line 1 `//! \`vibe change rebase\`: compare and move origin captures.`, line 2 `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-REBASE");`). Report types: `#[derive(serde::Serialize)] struct RebaseReport { change_id: String, applied: bool, origins: Vec<OriginReport> }`, `#[derive(serde::Serialize)] struct OriginReport { pinned: String, installed: Option<String>, manifest: &'static str, documents: Vec<DocumentReport> }`, `#[derive(serde::Serialize)] struct DocumentReport { address: String, status: &'static str }`.
3. `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-REBASE")] pub(super) fn run(ctx: &crate::output::Context, args: crate::cli::ChangeRebaseArgs) -> anyhow::Result<()>`: `let home = super::home::change_home(&args.path)?;` `let saved = vibe_requirements::change::read_saved_change(&home.selected_root, ".", None, &args.id)?;` For each origin in `saved.manifest.origins()`, for each document call `capture_origin(&home.world_root, &home.consumer, &doc.address)`; keep the Ok captures. Document status: `unchanged` if capture.document_sha256 == doc.sha256, `changed` if it differs, `missing` if capture_origin returned Err. The origin's `installed` is the package of its first Ok capture (None if none). Its `manifest` status is `missing` when there is no Ok capture, else `unchanged` or `changed` comparing that capture's manifest_sha256 with origin.manifest_sha256. `pinned` is origin.package.
4. Without --apply: print the report and write nothing; exit 0 whatever the statuses. With --apply: if any document status is `missing`, bail with `violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-REBASE: {address} no longer resolves in the installed world; fix surface: reinstall the origin or edit change.toml` (first missing address). Otherwise first collect, per origin, `Vec<(&str, &str)>` of (doc.address, capture.document_sha256) into a `Vec<Vec<(&str, &str)>>`, then build `Vec<OriginRebase>` borrowing them (package = origin.package, new_package = first capture's package, new_manifest_sha256 = first capture's manifest_sha256). `let text = String::from_utf8(saved.manifest_bytes.clone())?;` `let plan = vibe_core::manifest::change::plan_origin_rebase(&text, &home.context, saved.manifest.directory(), &rebases)?;` If `plan.edits` is empty write nothing. Otherwise `vibe_safefs::Project::open(&home.selected_root)?.write_atomic(&vibe_core::machine_json_path(&saved.manifest.directory().join("change.toml")), plan.after.as_bytes()).map_err(vibe_safefs::PublishError::into_report)?;`
5. Output: under `ctx.is_json()` call `ctx.emit_json(&RebaseReport { .. })?` where applied is true only when --apply was given and the run reached this point (including the no-edit case). Text: per origin `origin <pinned> -> <installed or ?> (manifest <status>)`, then per document `  <status>  <address>`; with --apply a final line `rebased <id>` when edits were written, or `<id> already matches the installed origins` when there were none.
6. Create tests/cli_change_rebase.rs with `mod common;`, `mod change_support;`, `use change_support::{ChangeWorld, ID};`. `rebase_dry_run_reports_changed_and_writes_nothing`: `let w = ChangeWorld::installed(); w.new_change(); w.upgrade(); let before = w.snapshot(ID);` run `w.vibe(&["--json", "change", "rebase", ID])`; assert success; parse stdout; assert `origins[0].installed == "feat:org.example/payments@=1.1.0"`, `origins[0].documents[0].status == "changed"`, `applied == false`, and `w.snapshot(ID) == before`. `rebase_apply_moves_pins_and_keeps_role_documents`: installed, new_change, upgrade; append `# keep me\n` to change.toml; save the bytes of intent.md and acceptance.md; run `w.vibe(&["change", "rebase", ID, "--apply"])` and assert success; assert change.toml contains `@=1.1.0`, `# keep me` and `sha256 = "{w.slot_sha("1.1.0", &common::spec_rel("FEAT-004.md"))}"`; intent.md and acceptance.md are byte-identical; `w.vibe(&["--json", "change", "rebase", ID])` reports `origins[0].documents[0].status == "unchanged"`; and `vibe_requirements::change::read_saved_change(w.project.path(), ".", None, ID).is_ok()`.
7. Apply the spec_edit to PROP-071 (insert the CHANGE-REBASE line immediately before the final `</spec>` line), then run the check.

**Decisions already made.**
- This entry runs after ADV-SDD-CHANGE-WRITE-3, one after the other: it shares cli/change.rs, cli/change/write.rs, commands/change/mod.rs and PROP-071 with it.
- Status words are exactly `unchanged`, `changed` and `missing`. The flag is `--apply`.
- No text diff of old and new origin documents: `vibe install` of the new version prunes the old slot (observed: '1 stale slot(s) pruned'), and reading the old version would need the registry cache or the network.
- Writes go only to change.toml through Project::write_atomic, so a failed write leaves the old file.
- rebase.rs is a new file; commands/change/mod.rs grows by 2 lines, cli/change.rs by 2, cli/change/write.rs by about 12. Run `wc -l` on each; none may exceed 600.
- No generated file, wire type or codegen step.
- Clippy -D warnings applies. Do not add #[allow].

**Not in this entry.** Not in this entry: showing old or new unit text or a unified diff, editing intent.md or acceptance.md, reading registry caches or fetching, rebasing a single origin by flag, and an MCP tool.

**Depends on.** ADV-SDD-CHANGE-WRITE-1, ADV-SDD-CHANGE-WRITE-2, ADV-SDD-CHANGE-WRITE-3

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml insert this line immediately before the final line `</spec>`:
```
  <p><CHANGE-REBASE fact="true" status="impl/done">vibe change rebase id compares every origin pin, manifest digest and document digest with the installed world and reports each cited document as unchanged, changed or missing without writing. With --apply it rewrites only the origin package, manifest_sha256 and document sha256 tokens through the exact token replay primitive, revalidates the result, and leaves every other byte of change.toml and every role document unchanged. A missing origin document refuses --apply. The text of the previous origin version is not shown.</CHANGE-REBASE></p>
```

Entries that write the same file run one after another, never in parallel.
