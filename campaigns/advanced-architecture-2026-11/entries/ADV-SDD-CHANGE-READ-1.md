# ADV-SDD-CHANGE-READ-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe_requirements::change::change_report` returns a `vibe-change-report/1` document for one saved change: id, title, slug, selected node, intent and work digests, each declared document with whether its file is present, and each work item. `list_changes` returns a `vibe-change-list/1` document naming every change directory and whether it reads as valid.

**Write.** `crates/vibe-requirements/src/change/report.rs`, `crates/vibe-requirements/src/change/report_cases.rs`, `crates/vibe-requirements/src/change/mod.rs`, `crates/vibe-requirements/Cargo.toml`, `Cargo.lock`, `vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml`

**Check.** `cargo test -p vibe-requirements --lib change::report_cases:: 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-requirements/src/change/report_cases.rs: `show_report_carries_digests_documents_and_work` and `list_names_valid_and_invalid_changes` (module `change::report_cases`, cfg(test)). Today the module does not exist, so the filter runs 0 tests and the grep fails.

**Read first.**
- crates/vibe-requirements/src/change/read.rs (read_saved_change, SavedChange, SavedChangeError, fn inputs: how a document path is read)
- crates/vibe-requirements/src/change/identity.rs (ChangeIdentityAnalysis { intent, work, graph }, SavedChange::identity_analysis)
- crates/vibe-requirements/src/change/m_02_b_2_cases.rs (fixture, WORK, TEXT: the fixture to copy, but NOT its docs/custom-specs spec root)
- crates/vibe-core/src/manifest/change/mod.rs (ChangeManifest accessors, ChangeDocument, ChangeDocumentRole, ChangeId::parse, CHANGES_DIR)
- crates/vibe-core/src/manifest/change/work.rs (WorkGraph::items, WorkItem, WorkNeed; WorkNeedKind is progress_core ArtifactKind with as_str)
- crates/vibe-safefs/src/project.rs (Project::open, Project::dir_if_present) and project/enumerate.rs (Project::child_names)

**Steps.**
1. Add `serde.workspace = true` under `[dependencies]` in crates/vibe-requirements/Cargo.toml (the workspace serde has the derive feature). Running the check updates Cargo.lock; commit that change too.
2. Create crates/vibe-requirements/src/change/report.rs. First item: `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-REPORT");`. Define `pub const CHANGE_REPORT_PROTOCOL: &str = "vibe-change-report/1";` and `pub const CHANGE_LIST_PROTOCOL: &str = "vibe-change-list/1";`. Define these structs, each `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]`, fields in this order: `ChangeReport { pub protocol: &'static str, pub id: String, pub title: String, pub slug: String, pub selected_node: String, pub intent_digest: String, pub work_digest: String, pub documents: Vec<ChangeDocumentRow>, pub work: Vec<ChangeWorkRow> }`; `ChangeDocumentRow { pub role: &'static str, pub path: String, pub normative: bool, pub input: bool, pub present: bool }`; `ChangeWorkRow { pub id: String, pub predecessors: Vec<String>, pub needs: Vec<ChangeNeedRow>, pub touches: Vec<String> }`; `ChangeNeedRow { pub kind: &'static str, pub target: String }`; `ChangeList { pub protocol: &'static str, pub changes: Vec<ChangeListRow> }`; `ChangeListRow { pub id: String, pub title: Option<String>, pub valid: bool }`.
3. In report.rs add `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-REPORT")] pub fn change_report(selected: &vibe_workspace::SelectedWorkspace, id: &str) -> Result<ChangeReport, SavedChangeError>`. Body: `let saved = read_saved_change(&selected.selected_root, selected.selected.as_str(), None, id)?;` then `let analysis = saved.identity_analysis()?;` and `let manifest = &saved.manifest;`. Fields: `id: manifest.id().as_str().to_string()`, `title: manifest.title().to_string()`, `slug: manifest.slug().to_string()`, `selected_node: manifest.selected_node().to_string()`, `intent_digest: analysis.intent.digest().to_string()`, `work_digest: analysis.work.digest().to_string()`. Documents in declared order from `manifest.documents()`; `role` is a match on `ChangeDocumentRole` giving `intent`, `refinement`, `acceptance`, `work`, `closure`. Open `let project = vibe_safefs::Project::open(&selected.selected_root).map_err(|_| SavedChangeError::Unavailable("held selected node"))?;` once. For each document `present` is `matches!(project.read_source_bytes(&vibe_core::machine_json_path(&manifest.directory().join(&doc.path))), Ok(Some(_)))` (read_source_bytes takes `&str`; any error or absence is `false`). Work rows come from `analysis.graph.items()` in iteration order: `id`, `predecessors` and `touches` cloned, needs as `ChangeNeedRow { kind: need.kind.as_str(), target: need.target.clone() }`.
4. In report.rs add `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-REPORT")] pub fn list_changes(selected: &vibe_workspace::SelectedWorkspace) -> Result<ChangeList, SavedChangeError>`. The current signature is `pub fn dir_if_present(&self, components: &[&str]) -> Result<Option<Pinned>>`, so build the components like this: ```
let mut owned: Vec<String> = vibe_core::layout::current_specs_root()
    .components()
    .map(|c| c.as_os_str().to_string_lossy().into_owned())
    .collect();
owned.push(CHANGES_DIR.to_string());
let parts: Vec<&str> = owned.iter().map(String::as_str).collect();
``` Open `Project::open(&selected.selected_root)` and call `project.dir_if_present(&parts)`. `Ok(None)` gives an empty `changes` vector. `Ok(Some(dir))` continues with `project.child_names(&dir)` (signature `pub fn child_names(&self, directory: &Pinned) -> Result<Vec<String>>`). Map every safefs error (open, dir_if_present, child_names) to `SavedChangeError::Unavailable("changes directory")`. Skip every name for which `ChangeId::parse(&name)` is `Err`. For each remaining name call `read_saved_change(&selected.selected_root, selected.selected.as_str(), None, &name)`: `Ok(saved)` gives `ChangeListRow { id: name, title: Some(saved.manifest.title().to_string()), valid: true }`, `Err(_)` gives `ChangeListRow { id: name, title: None, valid: false }`. Sort rows by `id`.
5. Edit crates/vibe-requirements/src/change/mod.rs (11 lines today): add `mod report;`, `pub use report::{CHANGE_LIST_PROTOCOL, CHANGE_REPORT_PROTOCOL, ChangeDocumentRow, ChangeList, ChangeListRow, ChangeNeedRow, ChangeReport, ChangeWorkRow, change_report, list_changes};` and `#[cfg(test)] mod report_cases;`.
6. Write crates/vibe-requirements/src/change/report_cases.rs with `use super::*;`, `const ID: &str = "chg-0123456789abcdef0123456789abcdef";`, the same `TEXT` include_str! and `WORK` constant as m_02_b_2_cases.rs, and its own `fixture() -> tempfile::TempDir`. The fixture writes vibe.toml exactly ```
[project]
group='org.fixture'
name='alpha'
version='1.0.0'
``` and the change directory `root.path().join(vibe_core::layout::current_specs_root()).join("changes").join(ID)` (do NOT copy m_02_b_2's `docs/custom-specs` root). In it: change.toml = `format!("{TEXT}\n[[documents]]\nrole='work'\npath='work.toml'\nnormative=false\ninput=true\n")`, intent.md = `# Intent\n@fact:INTENT Shared intent.\n`, acceptance.md = `# Acceptance\n@fact:ACCEPT Local criterion.\n`, work.toml = WORK, and no closure.md. Get `let sel = vibe_workspace::Workspace::discover_selected(root.path()).unwrap();`. Test 1 `show_report_carries_digests_documents_and_work` asserts: protocol `vibe-change-report/1`, id ID, title `Initial user title`, slug `first-title`, selected_node `.`; with `let a = read_saved_change(&sel.selected_root, ".", None, ID).unwrap().identity_analysis().unwrap();` the report's intent_digest equals `a.intent.digest()` and work_digest equals `a.work.digest()`; document roles are `["intent", "acceptance", "closure", "work"]`, closure `present == false`, the other three true; work ids `["work-a", "work-b", "work-c"]`; work-b predecessors `["work-a"]` and needs `[ChangeNeedRow { kind: "verification", target: "tests/a.rs".into() }]`; `serde_json::to_value(&report).unwrap()["protocol"] == "vibe-change-report/1"`. Test 2 `list_names_valid_and_invalid_changes`: first a SEPARATE tempdir holding only the vibe.toml above gives `list_changes` with protocol `vibe-change-list/1` and empty `changes`. Then on `fixture()` add `<specs root>/changes/chg-ffffffffffffffffffffffffffffffff/change.toml` containing `not toml` and a file `<specs root>/changes/README.md` containing `x`, and assert exactly 2 rows in this order: ID with `valid == true, title == Some("Initial user title")`, then `chg-ffffffffffffffffffffffffffffffff` with `valid == false, title == None`.
7. Insert this line in vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml directly before the closing `</spec>` line: ```
  <p><CHANGE-REPORT fact="true" status="impl/done">A read-only report observes one saved change of the selected node through the saved-change reader and its identity analysis. It carries protocol vibe-change-report/1, the change ID, title, slug and selected node, the intent and whole-work digests, every declared document with its role, normative and input flags and whether its file is present, and every work item with its predecessors, needs and touches in canonical order. The list report, protocol vibe-change-list/1, names every entry under configured-spec-root/changes whose name is a change ID, with its title and whether it reads as a valid change. Neither report writes, syncs, installs or fetches anything, and neither combines its observations into a completion verdict.</CHANGE-REPORT></p>
```

**Decisions already made.**
- The report types are Rust-owned serde documents identified by a `protocol` field, following the existing `vibe-<name>/1` convention (for example `vibe-progress-event/1`). They are not registered in formats/REGISTRY.toml and get no JTD schema, so no codegen and no tools/jtd-codegen are needed.
- The spec root is always the incumbent default (`None` passed to read_saved_change, which resolves to `vibe_core::layout::current_specs_root()`). There is no CLI or manifest override (PROP-071 CHANGE-DIRECTORY-IDENTITY).
- The node root is `selected.selected_root` and the node string passed to the reader is `selected.selected.as_str()` (`.` for a workspace root). The report's `selected_node` is the manifest's own `manifest.selected_node()`, which the reader has already checked against it.
- Production code has no unwrap or expect (conform NoUnwrapInDomain; vibe-requirements is in `gated`). Tests may unwrap.
- clippy -D warnings: every new pub item is re-exported from change/mod.rs; there are no unused fields or test-only imports in report.rs.
- `fact` is always the string `true`; this entry creates the CHANGE-REPORT anchor. specmap.json is not regenerated (self-check does not run `cargo xtask specmap --check`).
- File budgets: report.rs and report_cases.rs are new and must stay under 600 lines; mod.rs grows from 11 lines.

**Not in this entry.** Origin drift and the check report (entry 2), any CLI or MCP surface, requirements-query fact axes per need, a migration preview, and registering the report in formats/REGISTRY.toml.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml: add unit CHANGE-REPORT (status impl/done), exact text in the last step.

Entries that write the same file run one after another, never in parallel.
