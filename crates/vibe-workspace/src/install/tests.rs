//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use super::*;
use specmark::verifies;
use tempfile::TempDir;

#[cfg(test)]
fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

#[cfg(test)]
fn ver(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

/// A `ResolvedDep` with a content tree written into a fresh temp dir.
/// The `TempDir` is returned so the caller keeps it alive.
#[cfg(test)]
fn dep_with_boot(
    name: &str,
    version: &str,
    snippet_toml: &str,
    boot_rel: &str,
    boot_body: &str,
) -> (ResolvedDep, TempDir) {
    let pkg = TempDir::new().unwrap();
    write(
        pkg.path(),
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n\n{snippet_toml}"
        ),
    );
    write(pkg.path(), boot_rel, boot_body);
    let manifest = Manifest::read(pkg.path().join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.to_string(),
        version: ver(version),
        content_dir: pkg.path().to_path_buf(),
        manifest,
        requires: vec![],
    };
    (dep, pkg)
}

#[test]
fn apply_resolution_materialises_and_regenerates_a_standalone_project() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/10-flow-wal.md\"\ncategory = \"flow\"\n",
        "boot/10-flow-wal.md",
        "# wal boot",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    assert_eq!(outcome.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert_eq!(outcome.nodes_regenerated, vec!["."]);
    // The package tree is materialised verbatim into its slot.
    assert!(
        ws_dir
            .path()
            .join("vibedeps/flow-wal/0.3.0/boot/10-flow-wal.md")
            .is_file()
    );
    assert!(
        ws_dir
            .path()
            .join("vibedeps/flow-wal/0.3.0/vibe.toml")
            .is_file()
    );
    // INDEX.md names the node's own foundation boot and the dependency.
    let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(index.contains("spec/boot/00-core.md"), "{index}");
    assert!(
        index.contains("vibedeps/flow-wal/0.3.0/boot/10-flow-wal.md"),
        "{index}"
    );
    // The redirect lands at the node root.
    assert!(ws_dir.path().join("CLAUDE.md").is_file());
}

#[test]
fn apply_resolution_with_no_dependencies_still_writes_index() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"solo\"\nversion = \"0.1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(&ws, &[], SlotIntegrity::TrustPresence, None).unwrap();
    assert!(outcome.materialised.is_empty());
    assert_eq!(outcome.nodes_regenerated, vec!["."]);
    assert!(ws_dir.path().join("spec/boot/INDEX.md").is_file());
}

#[test]
fn apply_resolution_inline_dependency_produces_inline_md() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/crit\" = { version = \"^1.0\", link = \"inline\" }\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (dep, _pkg) = dep_with_boot(
        "crit",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/crit.md\"\n",
        "boot/crit.md",
        "# critical discipline",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    // The consumer declared `link = "inline"`, so the dependency's
    // boot is concatenated into INLINE.md.
    let inline = fs::read_to_string(ws_dir.path().join("spec/boot/INLINE.md")).unwrap();
    assert!(inline.contains("# critical discipline"), "{inline}");
}

#[test]
fn apply_resolution_renders_when_from_a_boot_snippet() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/win\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (dep, _pkg) = dep_with_boot(
        "win",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/win.md\"\nwhen = \"os:windows\"\n",
        "boot/win.md",
        "# windows-only guidance",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    // The `[boot_snippet].when` rides into INDEX.md, and the entry is
    // dynamic — a condition forces the dynamic INCLUDE form even with
    // no `link` declared anywhere.
    let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(
        index.contains("vibedeps/flow-win/1.0.0/boot/win.md"),
        "{index}"
    );
    assert!(index.contains("kind = \"dynamic\""), "{index}");
    assert!(index.contains("when = \"os:windows\""), "{index}");
}

#[test]
fn apply_resolution_skips_a_dependency_outside_the_node_requires() {
    // The resolution carries `flow:extra`, but the project does not
    // require it — it is materialised, but contributes no boot entry.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    let (wal, _w) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let (extra, _e) = dep_with_boot(
        "extra",
        "0.1.0",
        "[boot_snippet]\nsource = \"boot/extra.md\"\n",
        "boot/extra.md",
        "# extra",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(&ws, &[wal, extra], SlotIntegrity::TrustPresence, None).unwrap();

    let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(
        index.contains("vibedeps/flow-wal/0.3.0/boot/wal.md"),
        "{index}"
    );
    // `flow:extra` is materialised but not in the boot index.
    assert!(
        ws_dir
            .path()
            .join("vibedeps/flow-extra/0.1.0/boot/extra.md")
            .is_file()
    );
    assert!(!index.contains("flow-extra"), "{index}");
}

#[test]
fn apply_resolution_prunes_a_stale_slot_on_version_bump() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let ws = Workspace::load(ws_dir.path()).unwrap();

    let (wal_v1, _v1) = dep_with_boot(
        "wal",
        "0.1.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# v1",
    );
    apply_resolution(
        &ws,
        std::slice::from_ref(&wal_v1),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(ws_dir.path().join("vibedeps/flow-wal/0.1.0").is_dir());

    // Re-apply with wal bumped to 0.2.0 — the 0.1.0 slot is now stale.
    let (wal_v2, _v2) = dep_with_boot(
        "wal",
        "0.2.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# v2",
    );
    let outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&wal_v2),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(ws_dir.path().join("vibedeps/flow-wal/0.2.0").is_dir());
    assert!(
        !ws_dir.path().join("vibedeps/flow-wal/0.1.0").exists(),
        "the stale 0.1.0 slot must be pruned"
    );
    assert_eq!(outcome.pruned, vec!["vibedeps/flow-wal/0.1.0"]);
}

// --- PROP-011 §2.3 — materialise only the diff -----------------------

#[test]
fn apply_resolution_skips_a_present_slot_under_trust_presence() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();

    // First apply — the slot is absent, so it is materialised.
    let first = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert_eq!(first.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert!(first.skipped.is_empty());

    // A sentinel inside the slot — a file the source never had. If
    // the second apply re-copies the slot, `materialise` clears it
    // first and the sentinel vanishes; if it skips, the sentinel
    // survives. It is the proof the skip actually skipped.
    let sentinel = ws_dir.path().join("vibedeps/flow-wal/0.3.0/SENTINEL");
    fs::write(&sentinel, "untouched").unwrap();

    let second = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(
        second.materialised.is_empty(),
        "a present slot must not be re-copied"
    );
    assert_eq!(second.skipped, vec!["vibedeps/flow-wal/0.3.0"]);
    assert!(
        sentinel.is_file(),
        "TrustPresence must leave the slot untouched"
    );
}

#[test]
fn apply_resolution_rematerialises_a_present_slot_under_verify() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();

    apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify, None).unwrap();
    let sentinel = ws_dir.path().join("vibedeps/flow-wal/0.3.0/SENTINEL");
    fs::write(&sentinel, "doomed").unwrap();

    // Second apply under Verify — the present slot is re-materialised,
    // so the sentinel is cleared along with it.
    let second =
        apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify, None).unwrap();
    assert_eq!(second.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert!(second.skipped.is_empty(), "Verify must re-copy, never skip");
    assert!(!sentinel.exists(), "Verify must re-materialise the slot");
}

// --- PROP-020 2.1 — pre-install hooks ride the materialise pass ---------

/// A `ResolvedDep` whose manifest declares a `pre-install` hook, with the
/// `.sh` script written into the content tree so it materialises into the
/// slot where `select_invocation` looks for it.
#[cfg(test)]
fn dep_with_pre_hook(name: &str, version: &str) -> (ResolvedDep, TempDir) {
    let pkg = TempDir::new().unwrap();
    write(
        pkg.path(),
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n\n\
             [hooks]\npre-install = \"hooks/prepare\"\n"
        ),
    );
    write(
        pkg.path(),
        "hooks/prepare.sh",
        "#!/usr/bin/env bash\necho prep\n",
    );
    let manifest = Manifest::read(pkg.path().join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.to_string(),
        version: ver(version),
        content_dir: pkg.path().to_path_buf(),
        manifest,
        requires: vec![],
    };
    (dep, pkg)
}

/// Probe reporting a fixed set of interpreters present. The hook cell has an
/// identical fake, but a test-only type cannot cross module boundaries.
#[cfg(test)]
struct FakeProbe(Vec<String>);
#[cfg(test)]
impl InterpreterProbe for FakeProbe {
    fn has(&self, program: &str) -> bool {
        self.0.iter().any(|p| p == program)
    }
}

/// Runner returning a fixed exit code without spawning a process.
#[cfg(test)]
struct FakeRunner(i32);
#[cfg(test)]
impl HookRunner for FakeRunner {
    fn run(
        &self,
        _inv: &crate::hooks::HookInvocation,
        _cwd: &Path,
        _env: &[(String, String)],
    ) -> std::result::Result<i32, String> {
        Ok(self.0)
    }
}

#[cfg(test)]
fn allow_vibevm() -> HookPolicy {
    HookPolicy {
        allowed_groups: vec!["org.vibevm".to_string()],
        allow_hooks: false,
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#phases", r = 1)]
fn pre_install_hook_runs_for_an_allowed_group() {
    let ws = TempDir::new().unwrap();
    let (dep, _pkg) = dep_with_pre_hook("wal", "0.3.0");
    let out = materialise_resolution(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        Some(&allow_vibevm()),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(0),
    )
    .unwrap();
    assert_eq!(out.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    assert_eq!(out.hook_reports.len(), 1);
    assert_eq!(out.hook_reports[0].status, "ran");
    assert!(ws.path().join("vibedeps/flow-wal/0.3.0").is_dir());
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#failure", r = 1)]
fn pre_install_failure_rolls_back_the_slot() {
    let ws = TempDir::new().unwrap();
    let (dep, _pkg) = dep_with_pre_hook("wal", "0.3.0");
    // A non-zero pre-install exit is a hard failure (PROP-020 2.5).
    let err = materialise_resolution(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        Some(&allow_vibevm()),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(1),
    )
    .unwrap_err();
    assert!(matches!(err, WorkspaceError::Hook(_)), "{err}");
    assert!(
        !ws.path().join("vibedeps/flow-wal/0.3.0").exists(),
        "a failed pre-install must roll the slot back"
    );
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#trust-gate", r = 1)]
fn untrusted_group_skips_the_hook_without_running_it() {
    let ws = TempDir::new().unwrap();
    let (dep, _pkg) = dep_with_pre_hook("wal", "0.3.0");
    // The default policy trusts no group, so trust resolves to NeedsConsent:
    // the hook is skipped-and-reported and the runner is never reached. The
    // exit-1 runner would fail this test if it ran.
    let out = materialise_resolution(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        Some(&HookPolicy::default()),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(1),
    )
    .unwrap();
    assert_eq!(out.hook_reports[0].status, "skipped-needs-consent");
    assert!(
        ws.path().join("vibedeps/flow-wal/0.3.0").is_dir(),
        "a skipped hook is not a failure — the slot survives"
    );
}

/// A `ResolvedDep` declaring a `post-install` hook, its `.sh` script written
/// into the content tree.
#[cfg(test)]
fn dep_with_post_hook(name: &str, version: &str) -> (ResolvedDep, TempDir) {
    let pkg = TempDir::new().unwrap();
    write(
        pkg.path(),
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{version}\"\n\n\
             [hooks]\npost-install = \"hooks/finalise\"\n"
        ),
    );
    write(
        pkg.path(),
        "hooks/finalise.sh",
        "#!/usr/bin/env bash\necho done\n",
    );
    let manifest = Manifest::read(pkg.path().join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Flow,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.to_string(),
        version: ver(version),
        content_dir: pkg.path().to_path_buf(),
        manifest,
        requires: vec![],
    };
    (dep, pkg)
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#phases", r = 1)]
fn post_install_runs_for_materialised_slots_and_flags_failure() {
    let ws = TempDir::new().unwrap();
    let (dep, _pkg) = dep_with_post_hook("wal", "0.3.0");
    // Materialise the slot first (no pre-install) so the post-install script
    // is on disk for selection.
    let mat = materialise_resolution(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        None,
        &FakeProbe(vec![]),
        &FakeRunner(0),
    )
    .unwrap();

    // Runs for the freshly-materialised slot.
    let ran = run_post_install_with(
        ws.path(),
        std::slice::from_ref(&dep),
        &mat.materialised,
        &allow_vibevm(),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(0),
    )
    .unwrap();
    assert_eq!(ran.len(), 1);
    assert_eq!(ran[0].phase, "post-install");
    assert_eq!(ran[0].status, "ran");

    // A slot absent from the materialised set is skipped entirely.
    let skipped = run_post_install_with(
        ws.path(),
        std::slice::from_ref(&dep),
        &[],
        &allow_vibevm(),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(0),
    )
    .unwrap();
    assert!(skipped.is_empty());

    // A non-zero post-install exit is reported, not fatal (PROP-020 §2.5).
    let flagged = run_post_install_with(
        ws.path(),
        std::slice::from_ref(&dep),
        &mat.materialised,
        &allow_vibevm(),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(1),
    )
    .unwrap();
    assert_eq!(flagged[0].status, "post-install-failed");
}

// --- PROP-022 §2.4 — in-place materialization --------------------------

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-022#in-place", r = 1)]
fn apply_resolution_places_an_in_place_package_in_an_unversioned_slot() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/giant\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");

    // A fetched in-place clone: the manifest declares in-place, plus a `.git`
    // (the live working tree) and a boot snippet.
    let clone = TempDir::new().unwrap();
    write(
        clone.path(),
        "vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"giant\"\nkind = \"feat\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n\n[boot_snippet]\nsource = \"boot/giant.md\"\n",
    );
    write(clone.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(clone.path(), "boot/giant.md", "# giant boot");
    let manifest = Manifest::read(clone.path().join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Feat,
        group: Group::parse("org.vibevm").unwrap(),
        name: "giant".to_string(),
        version: ver("1.0.0"),
        content_dir: clone.path().to_path_buf(),
        manifest,
        requires: vec![],
    };

    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    // Placed in the UNVERSIONED slot, with `.git` preserved; no versioned slot.
    let slot = ws_dir.path().join("vibedeps/feat-giant");
    assert!(slot.join(".git/HEAD").is_file());
    assert!(slot.join("boot/giant.md").is_file());
    assert!(!ws_dir.path().join("vibedeps/feat-giant/1.0.0").exists());
    assert_eq!(outcome.materialised, vec!["vibedeps/feat-giant"]);

    // The clone source was moved, not copied.
    assert!(!clone.path().join("vibe.toml").exists());

    // `.gitignore` lists the slot — in-place is not vendored (§2.7).
    let gi = fs::read_to_string(ws_dir.path().join(".gitignore")).unwrap();
    assert!(gi.contains("vibedeps/feat-giant/"), "{gi}");

    // INDEX.md references the UNVERSIONED boot path.
    let index = fs::read_to_string(ws_dir.path().join("spec/boot/INDEX.md")).unwrap();
    assert!(
        index.contains("vibedeps/feat-giant/boot/giant.md"),
        "{index}"
    );
    assert!(!index.contains("feat-giant/1.0.0"), "{index}");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-022#in-place", r = 1)]
fn prune_leaves_an_in_place_slot_untouched() {
    // A standalone project whose resolution carries no packages must not
    // prune a pre-existing in-place slot (it is a git working tree, not a
    // stale versioned slot).
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    write(ws_dir.path(), "spec/boot/00-core.md", "# core");
    // Pre-place an in-place slot by hand.
    let clone = TempDir::new().unwrap();
    write(clone.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(clone.path(), "src/x", "y");
    vibedeps::materialise_in_place(ws_dir.path(), PackageKind::Feat, "giant", clone.path())
        .unwrap();

    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(&ws, &[], SlotIntegrity::TrustPresence, None).unwrap();

    // The empty resolution prunes nothing in-place; the slot survives.
    assert!(outcome.pruned.is_empty(), "{:?}", outcome.pruned);
    assert!(
        vibedeps::is_in_place_slot(ws_dir.path(), PackageKind::Feat, "giant"),
        "the in-place slot must survive a prune pass"
    );
}

/// An in-place `ResolvedDep` that also declares a `pre-install` hook — the
/// canonical bridge composition (PROP-023 §2.3): a git working tree shaped by
/// a hook. The clone carries `.git`, the in-place manifest, and the script.
#[cfg(test)]
fn dep_in_place_with_pre_hook(name: &str, version: &str) -> (ResolvedDep, TempDir) {
    let pkg = TempDir::new().unwrap();
    write(
        pkg.path(),
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"feat\"\nversion = \"{version}\"\nmaterialization = \"in-place\"\n\n[hooks]\npre-install = \"hooks/prepare\"\n"
        ),
    );
    write(pkg.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(
        pkg.path(),
        "hooks/prepare.sh",
        "#!/usr/bin/env bash\necho prep\n",
    );
    let manifest = Manifest::read(pkg.path().join("vibe.toml")).unwrap();
    let dep = ResolvedDep {
        kind: PackageKind::Feat,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.to_string(),
        version: ver(version),
        content_dir: pkg.path().to_path_buf(),
        manifest,
        requires: vec![],
    };
    (dep, pkg)
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#phases", r = 1)]
fn pre_install_hook_runs_in_an_in_place_slot() {
    let ws = TempDir::new().unwrap();
    let (dep, _pkg) = dep_in_place_with_pre_hook("giant", "1.0.0");
    let out = materialise_resolution(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        Some(&allow_vibevm()),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(0),
    )
    .unwrap();
    assert_eq!(out.materialised, vec!["vibedeps/feat-giant"]);
    assert_eq!(out.hook_reports.len(), 1);
    assert_eq!(out.hook_reports[0].status, "ran");
    // The hook ran against the unversioned in-place slot, which exists.
    assert!(vibedeps::is_in_place_slot(
        ws.path(),
        PackageKind::Feat,
        "giant"
    ));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#failure", r = 1)]
fn pre_install_failure_rolls_back_an_in_place_slot() {
    let ws = TempDir::new().unwrap();
    let (dep, _pkg) = dep_in_place_with_pre_hook("giant", "1.0.0");
    let err = materialise_resolution(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        Some(&allow_vibevm()),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(1),
    )
    .unwrap_err();
    assert!(matches!(err, WorkspaceError::Hook(_)), "{err}");
    // PROP-020 §2.5 — the in-place slot is rolled back, not left half-prepared.
    assert!(!vibedeps::is_in_place_slot(
        ws.path(),
        PackageKind::Feat,
        "giant"
    ));
}
