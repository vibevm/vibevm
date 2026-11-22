# ADV-SDD-DERIVE-6

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe tree --plain` marks each package that a tracked copy came from with ` [derived: <to>]`, while `vibe tree --json` stays unchanged.

**Write.** `crates/vibe-cli/src/commands/tree/mod.rs`, `crates/vibe-cli/tests/cli_derive_tree.rs`

**Check.** `cargo test -p vibe-cli --test cli_derive_tree -- --exact tree_names_derived_copies 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_derive_tree.rs :: tree_names_derived_copies (new test file)

**Read first.**
- crates/vibe-cli/src/commands/tree/mod.rs (run)
- crates/vibe-cli/src/commands/tree/model.rs (PackageTree, Package.provenance_suffix which is #[serde(skip)])
- crates/vibe-cli/src/commands/tree/plain.rs (row name format that appends provenance_suffix)
- crates/vibe-cli/src/commands/derive/record.rs (DeriveRecords::read, copies, package, to)
- crates/vibe-cli/tests/cli_derive/support.rs (Fixture)

**Steps.**
1. In commands/tree/mod.rs replace the line `    let tree = build::build_tree_bound(&root, format)?;` with `    let mut tree = build::build_tree_bound(&root, format)?;` followed by `    annotate_derived(&root, &mut tree);`.
2. Add to tree/mod.rs a private `fn annotate_derived(root: &Path, tree: &mut model::PackageTree)`: `let Ok(records) = super::derive::record::DeriveRecords::read(root) else { return; };` then for each copy (already sorted by name) and each package in `tree.packages` with `format!("{}/{}", package.group, package.name) == copy.package`, `package.provenance_suffix.push_str(&format!(" [derived: {}]", copy.to));`.
3. Create tests/cli_derive_tree.rs (support header as in entry 2) with test `tree_names_derived_copies`: installed(); json derive `org.fixture/style:templates --to local/t`; let out = vibe(["tree", "--plain"]); assert success and that stdout contains `org.fixture/style [derived: local/t]`; let tree = json(["tree"]); assert the serialized JSON text does not contain `derived`.

**Decisions already made.**
- The annotation rides on the existing text-only `provenance_suffix`, so the frozen v1 `vibe tree --json` schema and every Package literal stay unchanged.
- A missing or invalid vibe-derive.toml gives no annotation and never fails `vibe tree`.
- The suffix text is exactly ` [derived: <to>]`, one per copy, in copy-name order; the TUI shows it too because it renders the same field.
- Writes no file shared with entries 2 to 5; it only reads derive/record.rs, so it may run in parallel with them after entry 1.
- tree/mod.rs is 305 lines and grows by about 15.

**Not in this entry.** Not in this entry: a new field in the tree JSON model or schema, per-file rows for copied files in the tree, and any change to the TUI code.

**Depends on.** ADV-SDD-DERIVE-1

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
