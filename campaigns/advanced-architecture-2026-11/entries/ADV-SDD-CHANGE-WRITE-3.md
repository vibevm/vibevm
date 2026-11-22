# ADV-SDD-CHANGE-WRITE-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe change new --from spec://org.example/payments/FEAT-004#DELIVERY --title "EU payouts"` creates <spec-root>/changes/<id>/ holding a valid change.toml (origin captured from the installed package), intent.md and acceptance.md, and refuses an existing change directory, leaving it byte-identical.

**Write.** `crates/vibe-cli/src/cli/change.rs`, `crates/vibe-cli/src/cli/change/write.rs`, `crates/vibe-cli/src/commands/change/mod.rs`, `crates/vibe-cli/src/commands/change/home.rs`, `crates/vibe-cli/src/commands/change/new.rs`, `crates/vibe-cli/tests/change_support/mod.rs`, `crates/vibe-cli/tests/cli_change_new.rs`, `vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml`

**Check.** `cargo test -p vibe-cli --test cli_change_new 2>&1 | grep -F "test result: ok. 3 passed"`

**Test.** crates/vibe-cli/tests/cli_change_new.rs: `new_from_installed_origin_writes_a_valid_change`, `new_refuses_an_existing_change_directory`, `new_generates_a_chg_id_and_reports_json`

**Read first.**
- crates/vibe-cli/src/cli/change.rs and crates/vibe-cli/src/commands/change/mod.rs (ChangeArgs, ChangeSubcommand and the dispatcher `run`, created by the READ entry; see the contract in decisions)
- crates/vibe-cli/src/cli/cache.rs and crates/vibe-cli/src/commands/cache/mod.rs (the Args + Subcommand + match-dispatch pattern)
- crates/vibe-requirements/src/change/capture.rs (capture_origin, OriginCapture from ADV-SDD-CHANGE-WRITE-1)
- crates/vibe-core/src/manifest/change/mod.rs (ChangeId::parse, ChangeContext::for_node, ChangeContext::directory, ChangeContext::spec_root, CHANGES_DIR, ChangeManifest::parse_str)
- crates/vibe-safefs/src/project/pinned.rs (Pinned::create_child_exclusive), crates/vibe-safefs/src/project.rs (Project::open, Project::dir, ExclusiveChildError), crates/vibe-safefs/src/file/create_new.rs (Project::publish_new_in)
- crates/vibe-cli/tests/cli_facts_selected.rs (registry + `vibe install` fixture and the `snapshot` walk)

**Steps.**
1. Before editing, confirm the READ contract from decisions: `grep -n 'pub enum ChangeSubcommand' crates/vibe-cli/src/cli/change.rs`, `grep -n 'pub(crate) fn run' crates/vibe-cli/src/commands/change/mod.rs` and `grep -n 'Command::Change(_)' crates/vibe-cli/src/command_progress.rs` must each print one line. If any prints nothing, stop and report the packet as unclear; do not create a second `vibe change` command.
2. Create crates/vibe-cli/src/cli/change/write.rs. Line 1: `//! Argument structs for the writing verbs of \`vibe change\`.` Line 2: `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD");`. Add `use std::path::PathBuf;` and `#[derive(Debug, clap::Args)] pub struct ChangeNewArgs` with fields, each with a one-line doc comment: `#[arg(long)] pub from: String` (`The spec:// fact of an installed package this change starts from.`), `#[arg(long)] pub title: String` (`The change title.`), `#[arg(long)] pub slug: Option<String>` (`Display slug; defaults to the title in lowercase with hyphens.`), `#[arg(long)] pub id: Option<String>` (`Immutable change id; defaults to a generated chg- id.`), `#[arg(long, default_value = ".")] pub path: PathBuf` (`Project root; defaults to the current directory.`). In cli/change.rs add `mod write;` and `pub use write::*;` near its other declarations, and add to `ChangeSubcommand` the variant `New(ChangeNewArgs)` with doc comment `/// Create a change from a cited fact of an installed package.`
3. Create crates/vibe-cli/src/commands/change/home.rs (line 1 `//! The selected node and world a writing change verb acts on.`, line 2 the CHANGE-SCAFFOLD scope! line). Add `pub(super) struct ChangeHome { pub selected_root: PathBuf, pub world_root: PathBuf, pub consumer: vibe_spec::SelfCoordinate, pub context: vibe_core::manifest::change::ChangeContext }` and `pub(super) fn change_home(path: &Path) -> anyhow::Result<ChangeHome>`: `let selected = vibe_workspace::Workspace::discover_selected(path)?;` `let text = std::fs::read_to_string(selected.selected_root.join("vibe.toml"))?;` `let manifest = vibe_core::manifest::Manifest::parse_str(&text)?;` `let node = manifest.consumer_node().ok_or_else(|| anyhow::anyhow!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD: the selected vibe.toml declares no project or package; fix surface: vibe.toml"))?;` `let consumer = vibe_spec::SelfCoordinate::new(node.group.map(|g| g.to_string()), node.name);` `let context = ChangeContext::for_node(&manifest, ".", None)?;` return `ChangeHome { selected_root: selected.selected_root, world_root: selected.workspace.root, consumer, context }`. In commands/change/mod.rs add `mod home;` and `mod new;` and the arm `ChangeSubcommand::New(args) => new::run(ctx, args),`.
4. Create crates/vibe-cli/src/commands/change/new.rs (line 1 `//! \`vibe change new\`: scaffold a change directory from an installed origin.`, line 2 the CHANGE-SCAFFOLD scope! line). `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD")] pub(super) fn run(ctx: &crate::output::Context, args: crate::cli::ChangeNewArgs) -> anyhow::Result<()>`: 1) `let home = super::home::change_home(&args.path)?;` 2) id text: `args.id.clone()` when given; otherwise `let seed = format!("{}:{}:{}:{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(), std::process::id(), args.from, args.title);` `let hex = format!("{:x}", sha2::Sha256::digest(seed.as_bytes()));` and the id text is `format!("chg-{}", &hex[..32])` (import `sha2::Digest`). Then `let change_id = ChangeId::parse(&id_text)?;` and `let directory = home.context.directory(&change_id);` 3) `let capture = vibe_requirements::change::capture_origin(&home.world_root, &home.consumer, &args.from)?;` 4) slug = args.slug, or else: lowercase the title, replace each maximal run of characters that are not ASCII alphanumeric with one '-', trim leading and trailing '-'; if the result is empty use "change". 5) Render change.toml, intent.md and acceptance.md exactly as in decisions; the title in change.toml is `toml::Value::String(args.title.clone()).to_string()`. 6) Validate before any write: `ChangeManifest::parse_str(&toml_text, &home.context, &directory)?;`
5. Continue new.rs: 7) `let project = vibe_safefs::Project::open(&home.selected_root)?;` `let parent_text = vibe_core::machine_json_path(&home.context.spec_root().join(vibe_core::manifest::change::CHANGES_DIR));` `let parent_parts: Vec<&str> = parent_text.split('/').collect();` `let parent = project.dir(&parent_parts, true)?;` (the spec root is validated portable by ChangeContext::for_node, so the lossy conversion never loses bytes). `let dir_text = vibe_core::machine_json_path(&directory);` 8) `let dir = match parent.create_child_exclusive(change_id.as_str()) { Ok(dir) => dir, Err(vibe_safefs::ExclusiveChildError::NotCreated(source)) => anyhow::bail!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD: change directory {dir_text} already exists or cannot be created: {source}; fix surface: choose another --id"), Err(vibe_safefs::ExclusiveChildError::CreatedNotReopened { path, source }) => anyhow::bail!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-SCAFFOLD: created {} but could not reopen it: {source}; fix surface: inspect and remove that directory", path.display()) };` Nothing is cleaned up in either refusal. 9) For (name, bytes) in [("intent.md", intent), ("acceptance.md", acceptance), ("change.toml", toml_text)] in that order: `project.publish_new_in(&dir, name, bytes.as_bytes()).map_err(vibe_safefs::PublishError::into_report)?;` 10) Output: `#[derive(serde::Serialize)] struct NewReport { change_id: String, directory: String, files: Vec<&'static str> }`. If `ctx.is_json()`, `ctx.emit_json(&NewReport { change_id: change_id.as_str().to_string(), directory: dir_text, files: vec!["intent.md", "acceptance.md", "change.toml"] })?`; otherwise `println!("created change {} at {}", change_id.as_str(), dir_text)`.
6. Create crates/vibe-cli/tests/change_support/mod.rs. First line `#![allow(dead_code)]` (each test binary uses a subset; same pattern as tests/common/mod.rs). Then `use crate::common::UserScratch;`, `pub const ADDRESS: &str = "spec://org.example/payments/FEAT-004#DELIVERY";`, `pub const ID: &str = "eu-payouts";` and `pub struct ChangeWorld { pub user: UserScratch, pub project: tempfile::TempDir, pub registry: tempfile::TempDir }` with: `pub fn installed() -> Self` writes, for v in ["1.0.0", "1.1.0"], `registry/org.example/payments/v<v>/vibe.toml` = `[package]\ngroup='org.example'\nname='payments'\nkind='feat'\nversion='<v>'\n` and `registry/org.example/payments/v<v>/<vibe_core::layout::current_specs_root()>/FEAT-004.md` = `# Payments\n\n@fact:DELIVERY Payouts version <v>. @status:spec/done\n`; writes project `vibe.toml` = `[project]\ngroup='org.consumer'\nname='demo'\nversion='1.0.0'\n`; then calls `self.install("1.0.0")`. `pub fn install(&self, version: &str)` runs `user.vibe().args(["install", &format!("org.example/payments@={version}"), "--path"]).arg(project).arg("--registry").arg(registry).arg("--assume-yes").output()` and asserts success. `pub fn upgrade(&self)` calls `self.install("1.1.0")`. `pub fn vibe(&self, args: &[&str]) -> std::process::Output` runs `self.user.vibe().current_dir(self.project.path()).args(args).output().unwrap()`. `pub fn new_change(&self)` runs `self.vibe(&["change", "new", "--from", ADDRESS, "--title", "EU payouts", "--id", ID])` and asserts success. `pub fn change_dir(&self, id: &str) -> PathBuf` = `project/current_specs_root()/changes/id`. `pub fn slot_sha(&self, version: &str, tail: &str) -> String` = `format!("sha256:{}", vibe_workspace::vibedeps::sha256_file(&self.project.path().join(crate::common::slot_rel("org.example.payments", version, tail))).unwrap())`. `pub fn snapshot(&self, id: &str) -> std::collections::BTreeMap<PathBuf, Vec<u8>>` is the recursive walk of `change_dir(id)` copied from `snapshot` in tests/cli_facts_selected.rs.
7. Create crates/vibe-cli/tests/cli_change_new.rs with `mod common;`, `mod change_support;`, `use change_support::{ADDRESS, ChangeWorld, ID};`. `new_from_installed_origin_writes_a_valid_change`: `let w = ChangeWorld::installed(); w.new_change();` assert intent.md, acceptance.md and change.toml exist in `w.change_dir(ID)`; change.toml text contains `package = "feat:org.example/payments@=1.0.0"`, `manifest_sha256 = "{w.slot_sha("1.0.0", "vibe.toml")}"`, `sha256 = "{w.slot_sha("1.0.0", &common::spec_rel("FEAT-004.md"))}"` and `slug = "eu-payouts"`; and `vibe_requirements::change::read_saved_change(w.project.path(), ".", None, ID).is_ok()`. `new_refuses_an_existing_change_directory`: installed(), new_change(), `let before = w.snapshot(ID);` run the same command again with `w.vibe(...)`; assert it fails, its stderr contains `already exists`, and `w.snapshot(ID) == before`. `new_generates_a_chg_id_and_reports_json`: run `w.vibe(&["--json", "change", "new", "--from", ADDRESS, "--title", "EU payouts"])`, assert success, parse stdout with serde_json; `change_id` is a string of 36 characters starting with `chg-` whose remaining 32 characters are ASCII digits or `a`..=`f`, and `w.change_dir(change_id)` is a directory.
8. Apply the spec_edit to PROP-071 (insert the CHANGE-SCAFFOLD line immediately before the final `</spec>` line), then run the check.

**Decisions already made.**
- READ contract this entry builds on (delivered by the READ entry in depends_on): crates/vibe-cli/src/cli/change.rs holds `#[derive(Debug, clap::Args)] pub struct ChangeArgs { #[command(subcommand)] pub command: ChangeSubcommand }` and `#[derive(Debug, clap::Subcommand)] pub enum ChangeSubcommand`, re-exported from crates/vibe-cli/src/cli.rs with `mod change;` + `pub use change::*;`; `Command::Change(ChangeArgs)` exists in crates/vibe-cli/src/cli/command.rs; crates/vibe-cli/src/commands/change/mod.rs holds `pub(crate) fn run(ctx: &crate::output::Context, args: crate::cli::ChangeArgs) -> anyhow::Result<()>` whose body is `match args.command { ... }`; main.rs dispatches `Command::Change(args) => commands::change::run(&ctx, args)`; command_progress.rs maps `Command::Change(_) => FiniteFallback("Command change")` (a wildcard, so new variants need no arm there). With this contract main.rs, cli.rs, command.rs and command_progress.rs are not touched by this entry.
- change.toml text, values substituted, exact bytes:
```
schema = 1

[change]
id = "<id>"
selected_node = "."
slug = "<slug>"
title = <quoted title>

[[origins]]
package = "<capture.package>"
manifest_sha256 = "<capture.manifest_sha256>"

[[origins.documents]]
address = "<--from>"
sha256 = "<capture.document_sha256>"

[[documents]]
role = "intent"
path = "intent.md"
normative = true
input = true

[[documents]]
role = "acceptance"
path = "acceptance.md"
normative = true
input = true
```
(final newline after the last line).
- intent.md is `# <title>`, blank line, `Origin: <--from> (<capture.package>)`, blank line, `Describe the intended change here.`, final newline. acceptance.md is `# Acceptance: <title>`, blank line, `- [ ] Describe how a reviewer confirms the change.`, final newline. There is no closure document.
- --from is required because the change grammar requires at least one origin. The spec root is the layout default (configured_spec_root None), the same root `read_saved_change(root, ".", None, id)` reads. selected_node is always ".".
- Tests confirm validity with `vibe_requirements::change::read_saved_change` (exists today), not with `vibe change show`, so they do not depend on show's output or exit policy.
- Write order: intent.md, acceptance.md, then change.toml, each created exclusively in the exclusively created directory; a crash part-way leaves an undeclared directory, never a change.toml naming missing files.
- No new dependency: sha2, serde, toml, vibe-safefs, vibe-requirements, vibe-spec and vibe-workspace are already vibe-cli dependencies; tempfile, serde_json and vibe-core are dev-dependencies. No generated file, wire type or codegen step.
- Refusals are anyhow errors (exit 1) with the texts in the steps.
- All new files are small; commands/change/mod.rs grows by 3 lines. Run `wc -l` on every touched file before finishing; none may exceed 600.
- Clippy -D warnings applies. change_support uses `#![allow(dead_code)]` like tests/common/mod.rs; nowhere else gets #[allow].

**Not in this entry.** Not in this entry: quoting the cited unit's text into intent.md, multiple --from values, a closure.md, an MCP tool, reading a configured spec root, and any change to `vibe change show`.

**Depends on.** ADV-SDD-CHANGE-WRITE-1, ADV-SDD-CHANGE-READ-1

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-071-change-bindings-and-closure.xml insert this line immediately before the final line `</spec>`:
```
  <p><CHANGE-SCAFFOLD fact="true" status="impl/done">vibe change new --from address --title text creates configured-spec-root/changes/id/ holding change.toml, intent.md and acceptance.md. The id is chg- followed by 32 lowercase hexadecimal digits unless --id names a valid retained id. The slug defaults to the lowercased title with each run of other characters replaced by one hyphen. The origin is captured from the installed world and never fetched: the pin is the locked kind:group/name@=version, manifest_sha256 is the slot record digest of the slot's vibe.toml, and the document sha256 is the slot record digest of the file the address resolves to. An address naming the consuming project, carrying a version, naming a package that is not locked, or naming an in-place package refuses. The manifest is validated by the shared grammar before anything is written, the directory is created exclusively, and an existing change directory refuses and stays byte-identical.</CHANGE-SCAFFOLD></p>
```

Entries that write the same file run one after another, never in parallel.
