# ADV-SDD-DERIVE-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe derive remove <name>` deletes an unedited copy and its record, refuses with nothing removed when files were edited, and with `--force` drops the record while keeping the edited files.

**Write.** `crates/vibe-cli/src/commands/derive.rs`, `crates/vibe-cli/src/commands/derive/remove.rs`, `crates/vibe-cli/tests/cli_derive_remove.rs`

**Check.** `cargo test -p vibe-cli --test cli_derive_remove -- --exact derive_remove_keeps_edited_bytes 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_derive_remove.rs :: derive_remove_keeps_edited_bytes (new test file)

**Read first.**
- crates/vibe-cli/src/commands/derive.rs (DeriveCommand, run)
- crates/vibe-cli/src/commands/derive/show.rs (local_status, LocalStatus)
- crates/vibe-cli/src/commands/derive/record.rs (DeriveRecords::read, find, write)
- crates/vibe-cli/tests/cli_derive/support.rs (Fixture)

**Steps.**
1. In derive.rs add `mod remove;` and the variant `/// Drop a copy's record and delete the files you did not edit. Remove { name: String, #[arg(long)] force: bool, #[arg(long, default_value = ".")] path: PathBuf }` with the arm `Some(DeriveCommand::Remove { name, force, path }) => remove::run_remove(ctx, &super::resolve_project_root(&path)?, &name, force)`.
2. Create derive/remove.rs (scope! line). `pub(super) fn run_remove(ctx: &output::Context, root: &Path, name: &str, force: bool) -> Result<()>`: records = DeriveRecords::read(root)?; record = records.find(name)?.clone(); statuses = `super::show::local_status(root, &record)?`; edited = paths with status Modified or Added. If edited is non-empty and !force: bail "derived copy `{name}` has edited files ({comma-separated paths}); nothing was removed. Rerun with --force to drop the record and keep those files".
3. Delete every Unchanged file with std::fs::remove_file(root.join(&record.to).join(path)) and collect them as `removed`; `kept` = edited; Deleted rows need nothing. Then prune empty directories: a private `fn prune_empty_dirs(dir: &Path)` that recurses into child directories (std::fs::read_dir, symlink_metadata, directories only) and then calls `let _ = std::fs::remove_dir(dir);` so only empty directories go; call it on root.join(&record.to).
4. Drop the record: `records.copies.retain(|copy| copy.name != name); records.write(root)?;` (with no copies left the file holds only `schema = 1`). Emit JSON `{ok: true, command: "derive remove", name, removed, kept}`; human `removed {name}: {removed.len()} files deleted, {kept.len()} kept`.
5. Create tests/cli_derive_remove.rs (support header as in entry 2) with test `derive_remove_keeps_edited_bytes`: installed(); json derive `org.fixture/style:templates --to local/t`; write local/t/a.txt = "alpha edited\n"; fail(["derive", "remove", "style-templates"]) then local/t/b.txt exists and records() still has one copy; json(["derive", "remove", "style-templates", "--force"]) has removed ["b.txt", "img.bin"] and kept ["a.txt"]; local/t/a.txt == "alpha edited\n", local/t/b.txt absent, records() has no `copy` key; json derive `org.fixture/style:templates --to local/u`; json(["derive", "remove", "style-templates"]) succeeds (the name is free again) and local/u does not exist.

**Decisions already made.**
- `--force` never deletes user bytes: it only allows dropping the record while modified and added files stay on disk.
- A record file with no copies left is kept as `schema = 1`, not deleted.
- Directory pruning removes only empty directories under and including `to`; it never follows symlinks.
- Shares derive.rs with entries 1 to 4: depends_on is entry 4 so the derive.rs edits land one after another (its functional prerequisite is entry 2's local_status and find).
- remove.rs starts with the same specmark::scope! line; no spec edits.

**Not in this entry.** Not in this entry: deleting edited files, an `--all` flag, removing files outside `to`, or touching vibe.toml.

**Depends on.** ADV-SDD-DERIVE-4

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
