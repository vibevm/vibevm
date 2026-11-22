# ADV-SDD-DERIVE-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** After the package moves from 1.0.0 to 1.1.0, `vibe derive diff <name>` labels each file same, local-changed, upstream-changed, both-changed or upstream-added and gives a line diff of local against current upstream for text files.

**Write.** `crates/vibe-cli/src/commands/derive.rs`, `crates/vibe-cli/src/commands/derive/diff.rs`, `crates/vibe-cli/src/commands/vvm/site.rs`, `crates/vibe-cli/src/commands/vvm/site/diff.rs`, `crates/vibe-cli/tests/cli_derive_diff.rs`

**Check.** `cargo test -p vibe-cli --test cli_derive_diff -- --exact derive_diff_separates_upstream_and_local_changes 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_derive_diff.rs :: derive_diff_separates_upstream_and_local_changes (new test file)

**Read first.**
- crates/vibe-cli/src/commands/derive.rs (DeriveCommand, run)
- crates/vibe-cli/src/commands/derive/source.rs (installed, members, read_member)
- crates/vibe-cli/src/commands/derive/show.rs (walk)
- crates/vibe-cli/src/commands/vvm/site/diff.rs (line_diff)
- crates/vibe-cli/tests/cli_derive/support.rs (Fixture::publish, upgrade_to)

**Steps.**
1. Make the existing line diff reusable: in commands/vvm/site.rs change `mod diff;` to `pub(crate) mod diff;`; in commands/vvm/site/diff.rs change `pub(super) fn line_diff(old: &str, new: &str) -> String` to `pub(crate) fn line_diff(old: &str, new: &str) -> String`. No other change there.
2. Create derive/diff.rs (scope! line): `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "kebab-case")] pub(super) enum ThreeWay { Same, LocalChanged, UpstreamChanged, BothChanged, UpstreamAdded }`; `pub(super) struct DiffRow { pub(super) path: String, pub(super) status: ThreeWay, pub(super) local: Option<Vec<u8>>, pub(super) upstream: Option<Vec<u8>> }`; `pub(super) struct Comparison { pub(super) upstream_version: String, pub(super) rows: Vec<DiffRow> }`; `pub(super) fn compare(root: &Path, record: &DeriveRecord) -> anyhow::Result<Comparison>`: installed = `super::source::installed(root, &record.package)?` (its errors pass through), upstream = `super::source::members(&installed.record, &record.path)` read with `super::source::read_member(&installed.slot, member)`, local = `super::show::walk(&root.join(&record.to))?` plus the bytes of each local file, base = record file rows. Row set = union of base, local and upstream paths, sorted.
3. Classify each path with B = base hash, L = local hash, U = upstream hash (each optional): L == U (both present and equal, or both absent) -> Same; else B absent: L absent -> UpstreamAdded, U absent -> LocalChanged, otherwise BothChanged; else with B present: L != B and U == B -> LocalChanged; L == B and U != B -> UpstreamChanged; both differ -> BothChanged. An absent side counts as different from B.
4. Add `fn is_text(bytes: &[u8]) -> bool { !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok() }` and `pub(super) fn run_diff(ctx: &output::Context, root: &Path, name: &str) -> Result<()>`: JSON `{ok: true, command: "derive diff", name, package, base_version: record.version, upstream_version, files: [{path, status, text, diff}]}` where text = every present side is text, and diff = `Some(crate::commands::vvm::site::diff::line_diff(local_or_empty, upstream_or_empty))` when status is not Same and text is true, else null. Human: header `{name}: {package} {base_version} -> {upstream_version}`, then for each non-Same row `{status} {path}` followed by the diff text when present.
5. In derive.rs add `mod diff;` and the variant `/// Compare a copy with the installed version of its package, file by file. Diff { name: String, #[arg(long, default_value = ".")] path: PathBuf }` with the arm `Some(DeriveCommand::Diff { name, path }) => diff::run_diff(ctx, &super::resolve_project_root(&path)?, &name)`.
6. Create tests/cli_derive_diff.rs (support header as in entry 2) with test `derive_diff_separates_upstream_and_local_changes`: installed(); json derive `org.fixture/style:templates --to local/t`; write local/t/a.txt = "alpha local\n"; publish("1.1.0", [("templates/a.txt", b"alpha\n"), ("templates/b.txt", b"beta two\n"), ("templates/img.bin", &[0, 159, 146, 150]), ("templates/c.txt", b"gamma\n")]); upgrade_to("1.1.0"); json(["derive", "diff", "style-templates"]): base_version "1.0.0", upstream_version "1.1.0", a.txt local-changed, b.txt upstream-changed with diff exactly "-beta\n+beta two\n", c.txt upstream-added, img.bin same with text false and diff null.
7. Run `cargo clippy -p vibe-cli --all-targets -- -D warnings`: DiffRow fields and Comparison fields not read by run_diff are read by entry 4; if clippy flags one as dead here, read it in run_diff (the `local`/`upstream` bytes are read for `text` and `diff`, so all fields are used) rather than adding an allow.

**Decisions already made.**
- The diff is two-way: old = local copy, new = current upstream (`-` lines exist only locally, `+` lines only upstream). The old base bytes are never read, so the command works on any machine without the old version. This is the planner's choice tied to entry 1's record decision; it is part of the owner question on the record form.
- Text means valid UTF-8 without a NUL byte; anything else gets a status and `diff: null`.
- A file present locally and upstream with equal bytes is `same` even when both differ from the base.
- When the package is no longer installed, the error from source::installed is the answer; `vibe derive show` still works.
- Shares derive.rs with entries 1, 2, 4, 5: runs after entry 2 and before entries 4 and 5, one after another in id order.
- The only edits outside derive are the two visibility changes in vvm/site.rs (30 lines) and vvm/site/diff.rs (115 lines); the existing test in site/diff.rs stays unchanged.
- diff.rs starts with the same specmark::scope! line; no spec edits.

**Not in this entry.** Not in this entry: writing any file (that is rebase), Markdown or XML structural merges, reading `.vibe/cache` or the registry for old-base bytes, and moving line_diff to another crate.

**Depends on.** ADV-SDD-DERIVE-2

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
