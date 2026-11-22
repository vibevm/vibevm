# ADV-SDD-DERIVE-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe derive show <name>` reports each file of a copy as unchanged, modified, deleted or added against its recorded base hashes, and `vibe derive list` lists every copy with its count of changed files.

**Write.** `crates/vibe-cli/src/commands/derive.rs`, `crates/vibe-cli/src/commands/derive/record.rs`, `crates/vibe-cli/src/commands/derive/show.rs`, `crates/vibe-cli/tests/cli_derive_show.rs`

**Check.** `cargo test -p vibe-cli --test cli_derive_show -- --exact derive_show_reports_local_edits 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_derive_show.rs :: derive_show_reports_local_edits (new test file)

**Read first.**
- crates/vibe-cli/src/commands/derive.rs (DeriveArgs, run)
- crates/vibe-cli/src/commands/derive/record.rs (DeriveRecords, DeriveRecord)
- crates/vibe-cli/src/cli/scrape.rs (ScrapeArgs with args_conflicts_with_subcommands and Option subcommand)
- crates/vibe-cli/tests/cli_derive/support.rs (Fixture)
- crates/vibe-workspace/src/vibedeps/slot_record.rs (sha256_file)

**Steps.**
1. In derive.rs: add `#[command(args_conflicts_with_subcommands = true)]` to DeriveArgs, add field `#[command(subcommand)] pub command: Option<DeriveCommand>`, change `source: String` to `source: Option<String>`, and add `#[derive(Debug, Subcommand)] pub enum DeriveCommand { /// List the tracked local copies and how many of their files changed. List { #[arg(long, default_value = ".")] path: PathBuf }, /// Show each file of one copy: unchanged, modified, deleted or added. Show { name: String, #[arg(long, default_value = ".")] path: PathBuf } }`. Add `mod show;`.
2. In run, match `args.command` first, each subcommand arm resolving its own root from the variant's own `path` field: `Some(DeriveCommand::List { path }) => show::list(ctx, &super::resolve_project_root(&path)?)`, `Some(DeriveCommand::Show { name, path }) => show::show(ctx, &super::resolve_project_root(&path)?, &name)`; `None` with `args.source` Some -> the existing copy flow (which uses `args.path`); `None` with source None -> bail "`vibe derive` needs `<group>/<name>:<path-in-package>` or a subcommand; see `vibe derive --help`".
3. In record.rs add `impl DeriveRecords { pub(crate) fn find(&self, name: &str) -> anyhow::Result<&DeriveRecord> }` that bails "no derived copy named `{name}`; run `vibe derive list`".
4. Create derive/show.rs (scope! line): `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "kebab-case")] pub(super) enum LocalStatus { Unchanged, Modified, Deleted, Added }`; `pub(super) struct LocalFile { pub(super) path: String, pub(super) status: LocalStatus }`; `pub(super) fn walk(dir: &Path) -> anyhow::Result<BTreeMap<String, String>>` (recursive, uses std::fs::symlink_metadata, keeps regular files only, skips symlinks and other types, keys are '/'-joined paths relative to dir, values `vibedeps::sha256_file(..).map_err(anyhow::Error::msg)?`; absent dir gives an empty map); `pub(super) fn local_status(root: &Path, record: &DeriveRecord) -> anyhow::Result<Vec<LocalFile>>` (walk(&root.join(&record.to)); each record row: same hash -> Unchanged, other hash -> Modified, missing -> Deleted; leftover walked files -> Added; sorted by path).
5. In show.rs add `pub(super) fn list(ctx: &output::Context, root: &Path) -> Result<()>`: JSON `{ok: true, command: "derive list", copies: [{name, package, version, path, to, changed}]}` where changed = count of files whose status is not Unchanged; human: one `ctx.summary` line per copy `{name}  {package}@{version}:{path} -> {to}  ({changed} changed)` or `no derived copies`. Add `pub(super) fn show(ctx: &output::Context, root: &Path, name: &str) -> Result<()>`: JSON `{ok: true, command: "derive show", name, package, version, path, to, files: [{path, status}]}`; human: `{name}: {package}@{version}:{path} -> {to}` then one line per file `{status} {path}` with the kebab-case status word.
6. Create tests/cli_derive_show.rs with the same `#[path = "cli_derive/support.rs"]` + `#[allow(dead_code, reason = "each derive test binary uses part of the fixture")] mod support;` header and test `derive_show_reports_local_edits`: installed(); json derive `org.fixture/style:templates --to local/t`; write local/t/a.txt = "alpha edited\n", remove local/t/b.txt, write local/t/new.txt = "new\n"; json(["derive", "show", "style-templates"]) files equal, in order, a.txt modified, b.txt deleted, img.bin unchanged, new.txt added; json(["derive", "list"]) copies has one row with name "style-templates" and changed 3; fail(["derive", "show", "nope"]).
7. Also rerun `cargo test -p vibe-cli --test cli_derive -- --exact derive_copies_a_tree_and_records_it` (the copy flow now reads `source` as Option) and `cargo clippy -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- Status words are exactly `unchanged`, `modified`, `deleted`, `added`.
- Symlinks and non-regular files under `to` are ignored by the walk, never reported or followed.
- `vibe derive list|show` read only the project; they do not need the package to be installed.
- Each subcommand variant carries its own `--path`; its arm resolves the project root from that field, never from DeriveArgs.path.
- Shares derive.rs with entries 1, 3, 4, 5: runs after entry 1 and before entries 3, 4, 5, one after another in id order.
- The four registration files are not touched: the new subcommands live inside DeriveArgs.
- New file show.rs starts with the same specmark::scope! line as entry 1; no spec edits.

**Not in this entry.** Not in this entry: upstream comparison (`diff`), any write to the project, and a `--name` filter on list.

**Depends on.** ADV-SDD-DERIVE-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
