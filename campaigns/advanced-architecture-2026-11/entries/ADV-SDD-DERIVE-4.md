# ADV-SDD-DERIVE-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe derive rebase <name>` previews updates and conflicts; `--accept` refuses with nothing changed while conflicts exist, and `--accept --keep-local` copies upstream into the unedited files, keeps edited ones and moves the recorded base to the installed version.

**Write.** `crates/vibe-cli/src/commands/derive.rs`, `crates/vibe-cli/src/commands/derive/rebase.rs`, `crates/vibe-cli/tests/cli_derive_rebase.rs`

**Check.** `cargo test -p vibe-cli --test cli_derive_rebase -- --exact derive_rebase_takes_upstream_only_where_unedited 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_derive_rebase.rs :: derive_rebase_takes_upstream_only_where_unedited (new test file)

**Read first.**
- crates/vibe-cli/src/commands/derive.rs (DeriveCommand, run)
- crates/vibe-cli/src/commands/derive/diff.rs (compare, ThreeWay, DiffRow)
- crates/vibe-cli/src/commands/derive/source.rs (installed, members)
- crates/vibe-cli/src/commands/derive/record.rs (DeriveRecords::read, find, write, DeriveFileRow)
- crates/vibe-safefs/src/file/project_methods.rs (Project::write_atomic)
- crates/vibe-cli/tests/cli_derive/support.rs (Fixture)

**Steps.**
1. In derive.rs add `mod rebase;` and the variant `/// Take upstream changes into the files you did not edit. Without --accept, only shows the plan. Rebase { name: String, #[arg(long)] accept: bool, #[arg(long, requires = "accept")] keep_local: bool, #[arg(long, default_value = ".")] path: PathBuf }` with the arm `Some(DeriveCommand::Rebase { name, accept, keep_local, path }) => rebase::run_rebase(ctx, &super::resolve_project_root(&path)?, &name, accept, keep_local)`.
2. Create derive/rebase.rs (scope! line). `pub(super) fn run_rebase(ctx: &output::Context, root: &Path, name: &str, accept: bool, keep_local: bool) -> Result<()>`: records = DeriveRecords::read(root)?; record = records.find(name)?.clone(); comparison = `super::diff::compare(root, &record)?`; upstream rows = `super::source::members(&super::source::installed(root, &record.package)?.record, &record.path)`; if upstream rows are empty bail "`{path}` no longer exists in {package}@{version}; remove the copy with `vibe derive remove`". updates = paths with status UpstreamChanged or UpstreamAdded; conflicts = paths with status BothChanged.
3. Without --accept: emit and return Ok. JSON `{ok: true, command: "derive rebase", name, from_version: record.version, to_version: upstream_version, applied: false, updates, conflicts}`; human `{name}: {from} -> {to}` then `update {path}` and `conflict {path}` lines. Exit code 0 even with conflicts.
4. With --accept and non-empty conflicts and no --keep-local: bail "derived copy `{name}` has conflicts in {comma-separated paths}; nothing was changed. Rerun with --accept --keep-local to keep your versions and move the base to {to_version}" before writing anything.
5. Otherwise apply: project = vibe_safefs::Project::open(root)?; for each update row, upstream bytes present -> `project.write_atomic(&format!("{}/{}", record.to, row.path), bytes).map_err(PublishError::into_report)?`; upstream absent -> `std::fs::remove_file(root.join(&record.to).join(&row.path))`. Then set the stored copy's version to the upstream version and its files to the upstream rows as DeriveFileRow { path: rel, sha256 } (conflict files are left untouched on disk and take the upstream hash as their new base), `records.write(root)?`, and emit the same report with applied: true.
6. Create tests/cli_derive_rebase.rs (support header as in entry 2) with test `derive_rebase_takes_upstream_only_where_unedited`: installed(); json derive `org.fixture/style:templates --to local/t`; publish("1.1.0", [("templates/a.txt", b"alpha up\n"), ("templates/b.txt", b"beta two\n"), ("templates/img.bin", &[0, 159, 146, 150])]); upgrade_to("1.1.0"); write local/t/a.txt = "alpha local\n"; json(["derive", "rebase", "style-templates"]) has applied false, updates ["b.txt"], conflicts ["a.txt"] and local/t/b.txt still "beta\n"; fail(["derive", "rebase", "style-templates", "--accept"]) leaves local/t/b.txt "beta\n" and records() version "1.0.0"; json(["derive", "rebase", "style-templates", "--accept", "--keep-local"]) has applied true; local/t/b.txt == "beta two\n", local/t/a.txt == "alpha local\n", records() version "1.1.0" and the a.txt row sha256 equals the SHA-256 of "alpha up\n"; json(["derive", "show", "style-templates"]) reports a.txt modified and b.txt unchanged.

**Decisions already made.**
- Dry run is the default; `--keep-local` requires `--accept` (clap `requires`).
- User-edited bytes are never overwritten or deleted: conflicts are only listed; an `--accept` with conflicts and without `--keep-local` changes nothing and exits non-zero (the ordinary error path).
- After an applied rebase the recorded file rows are exactly the current upstream rows; files removed upstream drop out of the record, and a conflict file kept locally then shows as `modified` against the new base.
- Shares derive.rs with entries 1, 2, 3, 5: runs after entry 3 and before entry 5.
- rebase.rs starts with the same specmark::scope! line; no spec edits; diff.rs is read, not changed.

**Not in this entry.** Not in this entry: three-way text merging or conflict markers inside files, a rebase to a version other than the installed one, and a project-wide `--all`.

**Depends on.** ADV-SDD-DERIVE-3

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
