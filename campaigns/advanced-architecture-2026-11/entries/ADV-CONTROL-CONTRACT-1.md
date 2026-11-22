# ADV-CONTROL-CONTRACT-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A vibe-cli test runs the Library's project verbs (init, install --plan-only, install, tree, extensions, update --all, uninstall) with the Library's exact flags. It proves each stdout document reads into its registered wire type and back with nothing lost, and that stderr stays empty.

**Write.** `crates/vibe-cli/tests/common/library.rs`, `crates/vibe-cli/tests/common/mod.rs`, `crates/vibe-cli/tests/common/wire.rs`, `crates/vibe-cli/tests/client_contract_project.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test client_contract_project`

**Test.** crates/vibe-cli/tests/client_contract_project.rs: (1) library_project_verbs_conform_to_their_registered_formats asserts, in order: the init stdout conforms to vibe_wire::generated::init_report::InitReport. `install org.vibevm.world/wal@=1.0.0 --path P --offline --plan-only` prints exactly one document, conforming to vibe_wire::generated::install::e2::plan::InstallPlan. `install org.vibevm.world/wal@=1.0.0 --path P --offline` prints a document with command "install:plan" (conforms to install::e2::plan::InstallPlan) and a document with command "install" (conforms to install_report::InstallReport); both are present; documents with any other command are ignored. `tree --path P` validates against crates/vibe-cli/resources/package-tree.schema.v1.json after removing the keys ok, command, invoked_by and unattended (jsonschema::validator_for, zero errors). `extensions --path P` conforms to extensions_report::ExtensionsReport. `update --all --path P --offline` conforms to update_report::UpdateReport. `uninstall org.vibevm.world/wal --path P --offline` conforms to uninstall_report::UninstallReport. Every run exits 0 with empty stderr. (2) the_contract_oracle_refuses_an_undeclared_member takes the init document, inserts "undeclared": true and asserts that std::panic::catch_unwind(|| common::wire::conforms::<InitReport>(&doc)).is_err().

**Read first.**
- crates/vibe-cli/tests/common/wire.rs:1-75
- crates/vibe-cli/tests/common/mod.rs:1-80
- crates/vibe-cli/tests/common/mod.rs:200-260
- crates/vibe-cli/tests/offline_store_e2e.rs:1-40
- crates/vibe-cli/tests/tree_json.rs:15-80

**Steps.**
1. In crates/vibe-cli/tests/common/wire.rs, add `pub fn documents(stdout: &str) -> Vec<serde_json::Value>`. It collects serde_json::Deserializer::from_str(stdout).into_iter::<serde_json::Value>() and panics with the stdout on a parse error.
2. In the same file, add `pub fn conforms<T: DeserializeOwned + Serialize>(document: &serde_json::Value)`. Body: `let mut declared = document.clone();` and, if declared is an object, remove its top-level keys "invoked_by" and "unattended". Then `let typed: T = serde_json::from_value(declared.clone())` and on error panic with "the document does not read as its registered wire type: {error}\n{document}". Then `let back = serde_json::to_value(&typed).unwrap();` and `assert_eq!(back, declared, "the document and its registered wire type disagree");`. The comparison is serde_json::Value equality, which ignores member order.
3. Create crates/vibe-cli/tests/common/library.rs. Register it in crates/vibe-cli/tests/common/mod.rs by adding exactly two lines next to the existing `pub mod app_registry;` (line 35): `/// The VibeVM Library's invocation of the project verbs, as a fixture.` and `pub mod library;`. Change nothing else in mod.rs.
4. In library.rs define the constants LIBRARY (6 strings) and LIBRARY_RUN (8 strings), the fn file_url(dir: &Path) -> String copied from crates/vibe-cli/tests/offline_store_e2e.rs:27, and the struct LibraryProject with the fields and methods listed in the decisions.
5. LibraryProject::new(): `let root = tempfile::tempdir().unwrap(); let project = root.path().join("proj"); let user = UserScratch::new(); let mut this = LibraryProject { user, root, project, init: String::new() }; this.init = this.run(&["init", "--path", this.path(), "--name", "demo", "--offline"]);` (bind the path to a local String first if the borrow checker requires it). Then `let registry = super::make_wal_dir_registry(this.root.path());` and append to this.project.join("vibe.toml") the text "\n[[registry]]\nname = \"local\"\nurl = \"{file_url(&registry)}\"\n". Return this.
6. Create crates/vibe-cli/tests/client_contract_project.rs with `mod common;` and the two tests described in `test`, using common::library::LibraryProject and common::wire::{documents, conforms}. Run the check, then cargo fmt -p vibe-cli. Keep library.rs, wire.rs and client_contract_project.rs each under 600 lines.

**Decisions already made.**
- Library project flags, appended after the verb arguments exactly as the Library does: LIBRARY = ["--json","--unattended","--invoked-by","vibevm-app","--agent-mode","agent"]. Application run flags: LIBRARY_RUN = ["--json","--progress-format","json","--unattended","--agent-mode","cli","--invoked-by","vibevm-app"].
- LibraryProject fields: `pub user: UserScratch`, `pub root: tempfile::TempDir`, `pub project: PathBuf`, `pub init: String`. Methods: `pub fn path(&self) -> &str` (the project as &str); `pub fn output(&self, args: &[&str]) -> std::process::Output` (self.user.vibe().args(args).args(LIBRARY).output().unwrap()); `pub fn run(&self, args: &[&str]) -> String` (output, then assert success and empty stderr with both streams in the message, then return stdout); `pub fn install_wal(&self) -> String` (run ["install","org.vibevm.world/wal@=1.0.0","--path",path,"--offline"]).
- crates/vibe-cli/tests/common/mod.rs is already 610 lines. It gains exactly the two lines of step 3 and may stay over 600; do not move helpers out of it. The 600-line limit applies only to library.rs, wire.rs and client_contract_project.rs.
- The fixture registry is a file:// directory registry declared in the project's vibe.toml, so every Library verb runs with --offline and needs no warm-up and no git. This was verified with the built binary.
- The oracle compares JSON values, not text. An emitter's member order is not part of the contract, and the existing order-sensitive common::wire::typed stays as it is for its current callers.
- The red form of every later conformance check is the panic "the document and its registered wire type disagree" from conforms: the generated types do not deny unknown fields, so an undeclared member reads fine and is lost on the way back.
- The suite file has no platform cfg: everything in it is portable.
- Exit codes and stderr are asserted by the tests, never changed.

**Not in this entry.** Not in this entry: the closure-diff, list, check, cache and skill documents (later entries), application verbs, any change under crates/vibe-cli/src or schemas/, and splitting common/mod.rs to get it under 600 lines.

Entries that write the same file run one after another, never in parallel.
