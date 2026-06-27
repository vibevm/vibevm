//! Install-hook + in-place materialisation tests for the install cell,
//! out-of-line per the file-length budget. Shares scaffolding with
//! `tests` through `super::test_helpers`.

use super::test_helpers::*;
use super::*;
use specmark::verifies;
use tempfile::TempDir;

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
        source_mutable: false,
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
        source_mutable: false,
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
        source_mutable: false,
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
        source_mutable: false,
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

#[test]
fn materialise_subtree_does_not_prune_unrelated_slots() {
    // Scoped `vibe update` materialises only its subtree and must leave every
    // other installed slot in place — unlike `apply_resolution`, which prunes.
    let ws = TempDir::new().unwrap();
    write(
        ws.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    // An unrelated pre-existing slot a full prune pass WOULD remove.
    let stray = TempDir::new().unwrap();
    write(stray.path(), "vibe.toml", "x");
    vibedeps::materialise(
        ws.path(),
        PackageKind::Flow,
        "stray",
        &ver("0.1.0"),
        stray.path(),
    )
    .unwrap();

    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let out = materialise_subtree(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        None,
    )
    .unwrap();
    assert_eq!(out.materialised, vec!["vibedeps/flow-wal/0.3.0"]);
    // The unrelated slot survives — subtree materialisation never prunes.
    assert!(vibedeps::is_materialised(
        ws.path(),
        PackageKind::Flow,
        "stray",
        &ver("0.1.0")
    ));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-022#in-place", r = 1)]
fn already_placed_in_place_slot_runs_hook_without_moving() {
    // An incremental in-place update (PROP-022 §2.4) has the install layer
    // git-fetch the slot directly, then hand it to the materialise pass with
    // `content_dir` == the slot. The pass must NOT move/clear it — only run
    // the hook against the freshly-updated tree.
    let ws = TempDir::new().unwrap();
    let kind = PackageKind::Feat;
    let slot = vibedeps::in_place_slot_abs_path(ws.path(), kind, "giant");
    write(&slot, ".git/HEAD", "ref: refs/heads/main\n");
    write(
        &slot,
        "vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"giant\"\nkind = \"feat\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n\n[hooks]\npre-install = \"hooks/prepare\"\n",
    );
    write(
        &slot,
        "hooks/prepare.sh",
        "#!/usr/bin/env bash\necho prep\n",
    );
    write(&slot, "SENTINEL", "must survive");

    let manifest = Manifest::read(slot.join("vibe.toml")).unwrap();
    // `content_dir` IS the slot → the "already placed" signal.
    let dep = ResolvedDep {
        kind,
        group: Group::parse("org.vibevm").unwrap(),
        name: "giant".to_string(),
        version: ver("1.0.0"),
        content_dir: slot.clone(),
        manifest,
        requires: vec![],
        source_mutable: false,
    };

    let out = materialise_resolution(
        ws.path(),
        std::slice::from_ref(&dep),
        // `Verify` would normally re-materialise — the already-placed signal
        // overrides that for in-place.
        SlotIntegrity::Verify,
        Some(&allow_vibevm()),
        &FakeProbe(vec!["bash".to_string()]),
        &FakeRunner(0),
    )
    .unwrap();
    assert_eq!(out.materialised, vec!["vibedeps/feat-giant"]);
    assert_eq!(out.hook_reports[0].status, "ran");
    // The slot was NOT moved/cleared — its sentinel survives.
    assert!(
        slot.join("SENTINEL").is_file(),
        "an already-placed in-place slot must not be moved"
    );
}
