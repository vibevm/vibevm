# ADV-SDD-BOOT-DELTA-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe install <pkgs> --plan-only --boot-delta --json` emits an install:plan whose `boot_delta` rows equal the `lanes` of the install:closure-diff that the real `vibe install <pkgs>` then emits, while the project tree and the user settings outside the package store are byte-identical before and after the preview.

**Write.** `crates/vibe-cli/src/commands/install/boot_delta.rs`, `crates/vibe-cli/src/commands/install/mod.rs`, `crates/vibe-cli/src/commands/install/preview.rs`, `crates/vibe-cli/tests/cli_boot_delta_preview.rs`, `vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml`

**Check.** `cargo test -p vibe-cli --test cli_boot_delta_preview 2>&1 | grep -F "test result: ok. 1 passed"`

**Test.** crates/vibe-cli/tests/cli_boot_delta_preview.rs :: boot_delta_preview_matches_real_install_and_leaves_project_unchanged (new test target; it does not exist today, so the check fails before the work)

**Read first.**
- crates/vibe-cli/src/commands/install/preview.rs fn run_with_selected (the scratch close pattern) and fn prepare
- crates/vibe-cli/src/commands/install/direct.rs fn prepare_direct_install and fn execute_after_open (the composition this entry mirrors, without trace and with NoAfterDurableWorld)
- crates/vibe-cli/src/commands/install/closure_diff.rs fn lane_sizes and fn closure_diff (the lane_bytes filter rule)
- crates/vibe-cli/src/commands/install/trust.rs fn judgment, and crates/vibe-install/src/lifecycle/plan.rs fn admit_consent (why an empty trust policy with no question refuses every dependency contribution that would run)
- crates/vibe-cli/tests/cli_declared_boot.rs fn exercise (flow-package registry fixture with [boot_snippet])
- crates/vibe-cli/tests/cli_project_preview/support.rs fn invoke (JSON document stream parsing)

**Steps.**
1. Precondition: the owner has answered owner question 1 with option (a) and that answer is recorded in campaigns/advanced-architecture-2026-11/PLAN.md under Owner decisions. If it is not recorded there, stop with a failure note and change nothing.
2. Create crates/vibe-cli/src/commands/install/boot_delta.rs with the items in decisions: the scope lines, `pub(super) fn measure`, `fn copy_workspace`, `fn copy_dir`, `fn apply`, `fn rows`, and `struct BootDeltaGate` with its ConfirmGate impl.
3. In crates/vibe-cli/src/commands/install/mod.rs, add the line `mod boot_delta;` directly BEFORE the line `mod closure_diff;` (alphabetical order; rustfmt sorts the mod block).
4. In crates/vibe-cli/src/commands/install/preview.rs fn run_with_selected: as its first line, add `let boot_embedded = embedded_root.clone();`. Change `let report = match (result, cleanup) {` to `let mut report = match (result, cleanup) {`. Directly after that match statement (before `if ctx.is_json() {`), add the block given in decisions.
5. In vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml, apply the spec edit given in spec_edit.
6. Create crates/vibe-cli/tests/cli_boot_delta_preview.rs as described in decisions.
7. Run `cargo fmt --all`; reflowed lines are expected. Run `cargo clippy -p vibe-cli --all-targets -- -D warnings` and fix only warnings in this entry's files. Run `wc -l` on boot_delta.rs and preview.rs: each must stay at or under 600 lines.
8. Run the check.

**Decisions already made.**
- Order: this entry runs after ADV-SDD-BOOT-DELTA-1 (both write preview.rs) and before ADV-SDD-BOOT-DELTA-4 (both write boot_delta.rs and preview.rs).
- preview.rs block to add after the match:
```
    if args.boot_delta {
        report.boot_delta = Some(super::boot_delta::measure(
            ctx,
            &args,
            boot_embedded,
            root_offline,
        )?);
    }
```
- boot_delta.rs header: the doc line `//! vibe install --plan-only --boot-delta: the candidate closure installed into a throwaway copy of the workspace, and the watched boot lanes compared.` then `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#BOOT-DELTA-PREVIEW");`. Put `#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#BOOT-DELTA-PREVIEW")]` on fn measure (import `specmark::spec`). The anchor is created by this entry's spec_edit; cite no other new anchor.
- fn measure(ctx: &output::Context, args: &InstallArgs, embedded_root: Option<PathBuf>, root_offline: bool) -> Result<Vec<BootDeltaRow>> does this, in order:
1. `let root = super::resolve_project_root(&args.path)?;`
2. `let workspace_root = vibe_workspace::strip_unc_prefix(vibe_workspace::Workspace::discover(&root).context("discovering the workspace for the boot delta")?.root);`
3. `let member = root.strip_prefix(&workspace_root)` with the error context ``the project `{root}` is not inside its workspace root `{workspace_root}` `` (paths via .display()).
4. `let before = super::lane_sizes(&workspace_root);`
5. Create the scratch with `tempfile::Builder::new().prefix("vibe-boot-delta-").tempdir()`, context "creating the boot delta scratch directory". Set `copy = scratch.path().join("workspace")`.
6. Compute `result = copy_workspace(&workspace_root, &copy).and_then(|()| apply(ctx, args, &copy.join(member), embedded_root, root_offline)).map(|()| super::lane_sizes(&copy))`.
7. Close the scratch and combine result and cleanup exactly as preview.rs run_with_selected does with its four match arms. Use the context ``cleaning boot delta scratch `{path}` `` and, on a double failure, ``boot delta scratch cleanup also failed at `{path}`: {cleanup}``.
8. Return `Ok(rows(&before, &after))`.
- Copy skip rule (decided; it bounds the copy cost): copy_workspace(from, dest) -> Result<()> calls copy_dir(from, dest, true). Then, if `vibe_core::project_local_registry_path(from).is_file()`, it creates the parent of `vibe_core::project_local_registry_path(dest)` and copies that one file there. copy_dir(from, to, top: bool) -> Result<()>: create_dir_all(to); then for each std::fs::read_dir entry, using entry.file_type() (which does not follow symlinks):
- skip a symlink and anything that is neither a file nor a directory;
- skip a directory named `.git`, `target` or `node_modules` at any depth;
- when top is true, also skip a directory named `.vibe`;
- when the entry is a directory that itself contains an entry named `.git` (a nested checkout or worktree; check with `symlink_metadata(dir.join(".git")).is_ok()`), skip it;
- otherwise recurse into directories with top = false and std::fs::copy files.
Give every io error a context naming the path. Use std only: walkdir is a dev-dependency of vibe-cli and must not be used here. With this rule the copy of this repository is its authored tree (about 290 MB measured on 2026-11-22: campaigns, vibevm, crates); the cost is accepted for this entry.
- fn apply(ctx: &output::Context, args: &InstallArgs, project: &Path, embedded_root: Option<PathBuf>, root_offline: bool) -> Result<()>:
- `let quiet = ctx.quiet_child();`
- `let mut scratch_args = args.clone();` then set `path = project.to_path_buf()`, `plan_only = false`, `boot_delta = false`, `trace_compile = false`.
- `let root = super::resolve_project_root(&scratch_args.path)?;` and `let lease = super::acquire_lease(&root)?;`
- `let user_config = vibe_core::user_config::UserConfig::load().context("loading the user config")?;`
- `let policy = scratch_args.policy(root_offline, ctx.no_recommends(), &user_config);`
- `let selection = super::SelectedManifest::read(&root).prepare();`
- `let chain = vec!["validate".to_string(), "install".to_string()];`
- `let prelude = crate::commands::lifecycle::run_prelude(&quiet, selection, lease, "install", &chain, false, false).context("selecting the boot delta run identity")?;`
- Build `vibe_lifecycle::RunMetadata { requested: "install".to_string(), chain, offline: policy.offline, assume_yes: true, agent_mode: quiet.agent_mode(), force: false, trace_compile: prelude.identity.compile_trace, run_id: prelude.identity.run_id.clone(), started: prelude.identity.started.clone(), selected: prelude.selected.clone() }`.
- Call `vibe_orchestrator::execute_prepared(super::InstallExecution { args: scratch_args.inputs(), environment: &super::CliRegistryEnvironment::new(move || embedded_root), policy, lease: prelude.lease, manifest_mutation: &super::CliGitSourceMutation { args: &scratch_args }, selection: prelude.selection, metadata, sources: &super::CliPackageSourceFactory { args: &scratch_args, update_locked: false, reselect_sources: false }, confirm_gate: &BootDeltaGate { ctx }, observer: &super::CliInstallObserver::new(&quiet, None), agent: std::sync::Arc::new(vibe_lifecycle::NoAgentBackend), trace: None }, &mut super::NoAfterDurableWorld)` with the context "boot delta unavailable: installing the candidate closure into the scratch copy failed".
- If `run.parked.is_some() || matches!(run.disposition, super::InstallDisposition::Parked)`, `bail!("boot delta unavailable: the candidate install stops at a step that needs a hosting agent")`. Otherwise return Ok(()).
- struct BootDeltaGate<'a> { ctx: &'a output::Context }. impl vibe_orchestrator::ports::ConfirmGate for BootDeltaGate<'_>:
```
fn confirm_install(&self, _packages: usize) -> Result<()> {
    Ok(())
}

fn slot_judgment(&self) -> Result<vibe_install::SlotJudgmentContext> {
    Ok(vibe_install::SlotJudgmentContext {
        trust: vibe_core::trust::TrustPolicy::EMPTY,
        assume_yes: false,
        confirm: None,
        ..super::trust::judgment(self.ctx, false)?
    })
}
```
The host facts (PATH, environment, agent) stay the real ones, but no group is trusted, no question is asked and --assume-yes is not honoured, so SlotJudgment's admit_consent refuses every dependency `slot:` contribution that would run (Error::SlotConsentRequired), before any of them starts. The project's own manifest contributions (declarant_group None) are never asked about and run in the copy, as on every real install. confirm_install consents because nothing is installed into the project.
- fn rows(before: &[(String, Option<u64>)], after: &[(String, Option<u64>)]) -> Vec<BootDeltaRow> zips the two slices. It keeps a pair only when the lane names are equal and the sizes differ, which is the same rule as closure_diff::closure_diff lane_bytes. It maps each kept pair to `BootDeltaRow { lane, before, after }`. Import BootDeltaRow from vibe_wire::generated::install::e2::plan.
- No unwrap, expect or panic outside #[cfg(test)]. Every fallible call gets an anyhow context.
- Test file crates/vibe-cli/tests/cli_boot_delta_preview.rs. Start with `mod common; use common::UserScratch;`. Helper flow(registry, name, requires) writes `org.example/<name>/v1.0.0/vibe.toml` = `[package]\ngroup='org.example'\nname='<name>'\nkind='flow'\nversion='1.0.0'\n<requires>[boot_snippet]\nsource='<src>'\n`, where src = common::spec_rel("boot/<name>.md"), and the snippet file at that path with the text `# <name> {#<name>}\n\n<name>_marker ` followed by 300 'x' characters. Packages: shared (requires ""), then a and b, each with requires `[requires.packages]\n'org.example/shared'={version='=1.0.0',link='static'}\n`. The project vibe.toml is `[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\nspec_format='xml'\n`. Helper install(user, project, registry, preview: bool, package) runs `user.vibe()` with args install --json --path <project> --registry <registry> <package>, plus `--plan-only --boot-delta` when preview and `--assume-yes` otherwise. It asserts success (print stdout and stderr on failure) and parses stdout as a stream of JSON values with `serde_json::Deserializer::from_slice(..).into_iter::<serde_json::Value>()`. Helper snapshot(root, skip_top: Option<&str>) -> BTreeMap<String, Vec<u8>> records every file (path -> bytes) and every directory (path -> empty) under root, `.vibe` included, skipping only the top-level entry named skip_top.
- Test body: put `#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#BOOT-DELTA-PREVIEW")]` under #[test] on fn boot_delta_preview_matches_real_install_and_leaves_project_unchanged. Install org.example/a@=1.0.0 for real. Take P = snapshot(project, None) and U = snapshot(&user.settings, Some("cache")) (the settings-home `cache` is the package store, which the preview may fill). Run the preview for org.example/b@=1.0.0 and assert both snapshots are unchanged; on a mismatch print the differing paths. Take the doc whose command is "install:plan" and assert its "boot_delta" is a non-empty array. Install org.example/b@=1.0.0 for real, take the doc whose command is "install:closure-diff", and assert_eq!(plan["boot_delta"], diff["lanes"]). A manual run of the same shape on today's binary (a then b with a shared dependency) gave closure-diff lanes INDEX.md 814->963 and STATIC.xml null->1976, and a copy of the project without .vibe produced byte-identical lanes. If the settings snapshot differs, stop with a failure note naming the changed paths: the spec text below would then be false.

**Not in this entry.** Not in this entry: human-mode boot lines (ADV-SDD-BOOT-DELTA-4); isolating the package store or registry cache from the user's (vibe_install plan/acquire.rs and submission.rs write to vibe_registry::store::store_root() directly, so that is a separate change); reducing the copy below the decided skip rule; --boot-delta on update or uninstall; refactoring direct.rs to share code with this module; a test for a dependency with a slot contribution; regenerating specmap.json.

**Depends on.** ADV-SDD-BOOT-DELTA-1, ADV-SDD-BOOT-DELTA-2

**Spec edit (apply as a step of this entry).**

In vibevm/vibespecs/common/PROP-073-source-decisions-and-operation-effects.xml, inside <OPERATION-EFFECTS>, insert this line directly before the line that begins `    <p><NATIVE-STOP-LAW fact="true" status="impl/done">`:
```
    <p><BOOT-DELTA-PREVIEW fact="true" status="impl/done">`vibe install --plan-only --boot-delta` is the one opt-in exception, approved by the owner, to the preview law above. It copies the workspace into an owned scratch directory, leaving out every `.git`, `target` and `node_modules` directory, every nested checkout and the top-level `.vibe` directory but keeping the project-local registry file, installs the candidate closure there through the ordinary install path, and reports every watched boot lane whose size differs between the project and the copy. In the copy no dependency is trusted and no question is asked, so a candidate install in which a dependency's `slot:` contribution would run reports the boot delta as unavailable; the project's own manifest contributions run as on every install. Post-install lifecycle phases do not run, and a candidate install that needs a hosting agent reports the boot delta as unavailable. The run may fill the ordinary package store and registry cache with the candidate packages; the project, its lifecycle state and the user's settings outside the package store stay unchanged, and the scratch directory is removed after success and after failure.</BOOT-DELTA-PREVIEW></p>
```

Entries that write the same file run one after another, never in parallel.
