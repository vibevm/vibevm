# ADV-SDD-DETECT-STACK-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe init --detect-stack --plan-only --json` in an existing repository prints the detected stacks with their evidence files, skipped paths and assumptions, and leaves every byte of the tree unchanged.

**Write.** `crates/vibe-cli/src/cli/init.rs`, `crates/vibe-cli/src/commands/init/mod.rs`, `crates/vibe-cli/src/commands/init/detect.rs`, `crates/vibe-cli/src/commands/init/proposal.rs`, `crates/vibe-cli/tests/cli_init_detect.rs`

**Check.** `cargo test -p vibe-cli --test cli_init_detect detect_scan_ 2>&1 | grep -F 'test result: ok. 2 passed'`

**Test.** crates/vibe-cli/tests/cli_init_detect.rs (new test target): `detect_scan_reports_root_marker_and_writes_nothing` and `detect_scan_reads_one_level_below_only_without_root_marker`. Today the target does not exist, so cargo reports no test target and grep fails.

**Read first.**
- crates/vibe-cli/src/cli/init.rs: struct InitArgs
- crates/vibe-cli/src/commands/init/mod.rs: fn run, enum InitMode
- crates/vibe-safefs/src/file/stable.rs: Project::stable_file_state and fn read_stable_with_identity_in (which results mean absent and which mean changed)
- crates/vibe-safefs/src/project/pinned.rs: Pinned::open_child_checked
- crates/vibe-safefs/src/project/enumerate.rs: Project::child_names
- crates/vibe-cli/tests/cli_init.rs: how UserScratch builds `vibe init` commands

**Steps.**
1. In crates/vibe-cli/src/cli/init.rs, add two fields to `pub struct InitArgs` directly after the `stack` field, exactly:
```
    /// Detect the project's stack from its marker files (Cargo.toml,
    /// package.json, tsconfig.json, go.mod) instead of naming it with --stack.
    #[arg(long = "detect-stack", conflicts_with = "stack", requires = "plan_only")]
    pub detect_stack: bool,

    /// With --detect-stack: print what init would detect and write, and write nothing.
    #[arg(long = "plan-only", requires = "detect_stack")]
    pub plan_only: bool,
```
Every other `InitArgs { .. }` literal in the crate ends with `..args.clone()` (mod.rs fn ensure_project_exists, prompts.rs, root_role.rs); confirm with `grep -rn 'InitArgs {' crates/vibe-cli/src` and change none of them.
2. Create crates/vibe-cli/src/commands/init/detect.rs. First line after the module doc: `specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#project-initialization");`. Define `pub(super) struct StackRow { pub name: &'static str, pub coordinate: &'static str, pub markers: &'static [&'static str] }` and `pub(super) const STACKS: &[StackRow]` with exactly three rows in this order: (`rust-ai-native`, `org.vibevm.ai-native/rust-ai-native`, [`Cargo.toml`]), (`typescript-ai-native`, `org.vibevm.ai-native/typescript-ai-native`, [`package.json`, `tsconfig.json`]), (`go-ai-native`, `org.vibevm.ai-native/go-ai-native`, [`go.mod`]). Define `const EXCLUDED: &[&str] = &[".git", "vibevm", "target", "node_modules", "dist"];` and `const JS_ASSUMPTION: &str` with exactly this text:
```
package.json without tsconfig.json is read as TypeScript; a plain JavaScript project also gets typescript-ai-native
```
Define `#[derive(Debug, Clone, serde::Serialize)] pub(super) struct DetectedStack { pub coordinate: String, pub name: String, pub evidence: Vec<String> }`, `#[derive(Debug, Clone, serde::Serialize)] pub(super) struct SkippedPath { pub path: String, pub reason: &'static str }` and `pub(super) struct Detection { pub stacks: Vec<DetectedStack>, pub skipped: Vec<SkippedPath>, pub assumptions: Vec<String> }`.
3. In detect.rs write `pub(super) fn detect(root: &std::path::Path) -> anyhow::Result<Detection>` (root is absolute and exists). Algorithm: (a) `let project = vibe_safefs::Project::open(root)?; let dir = project.root_dir()?;` then `project.child_names(&dir)?` sorted bytewise; errors here are returned. (b) For each root name that is a marker of any row, call a helper `fn check_marker(project: &vibe_safefs::Project, rel: &str, evidence: &mut Vec<String>, skipped: &mut Vec<SkippedPath>)` that maps `project.stable_file_state(rel)` exactly so: `Ok(Some(_))` -> push rel to evidence; `Ok(None)` -> skipped {rel, `unstable`} (the file vanished after it was listed); `Err(e)` where `format!("{e:#}")` contains `changed between` or `no longer names the held file` -> skipped {rel, `unstable`} (content or name changed during the two hash passes); any other `Err(_)` -> skipped {rel, `unreadable`} (a symlink, a non-regular file, a hard-linked file or a permission error lands here, because safefs opens no-follow and refuses nlink > 1). (c) List the root again, sort; when it differs from the first listing push skipped {path: `.`, reason: `unstable`}. (d) Only when evidence is still empty: for each root name that is neither in EXCLUDED nor a marker, `dir.open_child_checked(name)`: `Ok(Some(child))` -> `project.child_names(&child)` (Err -> push skipped {name, `unreadable`} and continue), sort, run check_marker on `format!("{name}/{marker}")` for each marker name present, then list again and push skipped {name, `unstable`} when the listing differs; `Ok(None)` -> skipped {name, `unstable`}; `Err(e)` -> push skipped {name, `unreadable`} only when `e.chain().find_map(|c| c.downcast_ref::<std::io::Error>())` has kind `std::io::ErrorKind::PermissionDenied`, otherwise pass over it (regular files and links are not scanned). (e) Build `stacks` in STACKS order: a row's evidence is every evidence path whose last `/`-segment is one of its markers, sorted bytewise; rows with no evidence are left out. (f) When the typescript row is present and none of its evidence ends in `tsconfig.json`, push `JS_ASSUMPTION.to_string()` to assumptions. (g) Sort skipped by path bytewise. No unwrap/expect/panic outside #[cfg(test)].
4. Create crates/vibe-cli/src/commands/init/proposal.rs (same scope line as detect.rs). Define `#[derive(serde::Serialize)] pub(super) struct Proposal { ok: bool, command: &'static str, plan_only: bool, stacks: Vec<super::detect::DetectedStack>, skipped: Vec<super::detect::SkippedPath>, assumptions: Vec<String> }` with `pub(super) fn new(detection: super::detect::Detection) -> Self` (ok true, command `init`, plan_only true) and `pub(super) fn report(&self, ctx: &crate::output::Context, display_root: &str) -> anyhow::Result<()>`: when `ctx.is_json()` return `ctx.emit_json(self)`; otherwise `ctx.rule(&format!("stack detection in {display_root}"))`, then one `ctx.hint` per stack with `format!("stack: {name} ({coordinate}) · evidence: {}", evidence.join(", "))`, or `ctx.hint("stack: none detected")` when there are none, one `ctx.hint(&format!("skipped: {path} · {reason}"))` per skipped row, one `ctx.hint(&format!("assumption: {text}"))` per assumption, then `ctx.trailer(&format!("plan only · {n} stack{} detected · nothing written", if n == 1 { "" } else { "s" }))` and `Ok(())`.
5. In crates/vibe-cli/src/commands/init/mod.rs add `mod detect;` and `mod proposal;` to the module list. In `fn run`, directly after the `let project_path = ...;` statement and before `let mut user_config`, insert: when `args.detect_stack` is true, first refuse with exactly `bail!("`--detect-stack` starts a project root; it cannot be combined with `init package`, `init group` or `--type package`")` when `!matches!(mode, InitMode::Project) || args.init_type == Some(crate::cli::InitType::Package)`; then, when `args.plan_only`, compute `let root = helpers::canonical_no_unc(&project_path)?;`, `let detection = detect::detect(&root)?;`, call `proposal::Proposal::new(detection).report(ctx, &helpers::normalize_display(&project_path, &root))?;` and `return Ok(());`. This branch never loads or saves the user config.
6. Create crates/vibe-cli/tests/cli_init_detect.rs using `use vibe_test_support::UserScratch;` (no `mod common;`). Helpers: `fixture(files: &[(&str, &str)]) -> tempfile::TempDir` (creates parent dirs), `snapshot(root: &Path) -> BTreeMap<String, Vec<u8>>` (recursive std::fs walk, files with their bytes, directories as empty entries keyed with a trailing `/`), `plan(user: &UserScratch, dir: &Path) -> serde_json::Value` running `user.vibe().args(["init", "--detect-stack", "--plan-only", "--json", "--path"]).arg(dir)`, asserting success and parsing stdout. Test `detect_scan_reports_root_marker_and_writes_nothing`: fixture `Cargo.toml` = `[package]\nname = "demo"\nversion = "0.1.0"\n`, `CLAUDE.md` = `# Notes\nmine\n`, `src/main.rs` = `fn main() {}\n`; snapshot before == after; json `ok` true, `command` `init`, `plan_only` true, `stacks` == `[{"coordinate":"org.vibevm.ai-native/rust-ai-native","name":"rust-ai-native","evidence":["Cargo.toml"]}]`, `skipped` == `[]`, `assumptions` == `[]`. Test `detect_scan_reads_one_level_below_only_without_root_marker`: fixture A `web/package.json`=`{}\n`, `node_modules/package.json`=`{}\n`, `target/Cargo.toml`=`[package]\n`, `docs/readme.txt`=`x\n` -> stacks == one typescript-ai-native row with evidence `["web/package.json"]`, assumptions == [the exact JS_ASSUMPTION text]; fixture B `Cargo.toml` plus `site/package.json`=`{}\n` -> stacks == only the rust-ai-native row with evidence `["Cargo.toml"]`.

**Decisions already made.**
- Flags `--detect-stack` and `--plan-only` require each other in this entry; ADV-SDD-DETECT-STACK-3 lifts the requirement on `--detect-stack`.
- The detection table is built in (three published stacks only); no stack package declares markers.
- Root markers decide. Direct child directories are scanned only when the root has no marker, so a Rust repo with `site/package.json` stays unambiguous.
- Excluded names: .git, vibevm, target, node_modules, dist. Symlinks and reparse points are never followed.
- `skipped[]` reasons are exactly `unstable` (the path vanished, or its content, name or directory listing changed during the scan) and `unreadable` (a link, a non-regular file, a hard-linked file, or a permission error); neither is fatal. Failure to open or list the root itself is fatal, and a --path that does not exist fails in canonical_no_unc.
- The proposal JSON is a hand-written serde struct emitted through output::Context::emit_json, not a registered wire format.
- Keys: ok, command, plan_only, stacks[{coordinate,name,evidence}], skipped[{path,reason}], assumptions[]. ADV-SDD-DETECT-STACK-2 adds agent_files and writes; ADV-SDD-DETECT-STACK-3 adds next_commands.
- detect.rs and proposal.rs carry the existing anchor spec://org.vibevm.core/vibevm/VIBEVM-SPEC#project-initialization through specmark::scope!; no #[spec] marker is added, so specmap.json needs no regeneration.

**Not in this entry.** Not in this entry: initializing the project, agent-file or write listings, a generic analyzer or inventory crate, reading stack markers from package manifests, touching vibe-scrape or vibe-safefs, and registering a JTD/JSON schema for the proposal.

**Spec edit (apply as a step of this entry).**

None. detect.rs and proposal.rs use the existing anchor spec://org.vibevm.core/vibevm/VIBEVM-SPEC#project-initialization (VIBEVM-SPEC.md §14.1) through specmark::scope!.

Entries that write the same file run one after another, never in parallel.
