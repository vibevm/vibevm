//! In-place materialisation tests for the install cell (PROP-022 §2.4),
//! out-of-line per the file-length budget. Shares scaffolding with `tests`
//! through `super::test_helpers`.

use std::cell::RefCell;

use super::test_helpers::*;
use super::*;
use specmark::verifies;
use tempfile::TempDir;
use vibe_core::{Group, PackageKind};

/// Records `<point>` for every slot event; `fail_pre` refuses pre-install.
#[cfg(test)]
#[derive(Default)]
struct EventLog {
    events: RefCell<Vec<&'static str>>,
    fail_pre: bool,
}

#[cfg(test)]
impl SlotLifecycle for EventLog {
    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        assert!(context.slot.is_dir(), "pre runs against a placed slot");
        self.events.borrow_mut().push("pre-install");
        if self.fail_pre {
            Err("fixture pre failure".to_string())
        } else {
            Ok(())
        }
    }

    fn post_install(&self, _context: SlotLifecycleContext<'_>) -> Result<(), String> {
        self.events.borrow_mut().push("post-install");
        Ok(())
    }
}

#[cfg(test)]
fn materialise_with(
    workspace: &Path,
    dep: &ResolvedDep,
    slot_integrity: SlotIntegrity,
    lifecycle: &dyn SlotLifecycle,
) -> Result<Materialised, WorkspaceError> {
    materialise_resolution_with_spec_format(
        workspace,
        std::slice::from_ref(dep),
        MaterialiseOptions {
            slot_integrity,
            spec_format: SpecFormat::Mixed,
            slot_verifier: None,
            lifecycle: SlotLifecycleMode::Callback(lifecycle),
            progress: vibe_core::progress::Progress::default(),
            rollback: None,
        },
    )
}

#[cfg(test)]
fn in_place_dep(name: &str, content_dir: &Path, in_place_changed: Option<bool>) -> ResolvedDep {
    ResolvedDep {
        kind: PackageKind::Feat,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.to_string(),
        version: ver("1.0.0"),
        content_dir: content_dir.to_path_buf(),
        source_hash: Some(source_hash()),
        embedded_sources: Vec::new(),
        manifest: Manifest::read(content_dir.join("vibe.toml")).unwrap(),
        requires: vec![],
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed,
        configuration: None,
    }
}

/// A fetched in-place clone: the manifest declares in-place and the tree
/// carries its `.git` (the live working tree).
#[cfg(test)]
fn in_place_clone(name: &str, extra_toml: &str) -> TempDir {
    let clone = TempDir::new().unwrap();
    write_in_place_tree(clone.path(), name, extra_toml);
    clone
}

#[cfg(test)]
fn write_in_place_tree(root: &Path, name: &str, extra_toml: &str) {
    write(
        root,
        "vibe.toml",
        &format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"feat\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n{extra_toml}"
        ),
    );
    write(root, ".git/HEAD", "ref: refs/heads/main\n");
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn apply_resolution_places_an_in_place_package_in_an_unversioned_slot() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/giant\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

    let clone = in_place_clone("giant", "\n[boot_snippet]\nsource = \"boot/giant.md\"\n");
    write(clone.path(), "boot/giant.md", "# giant boot");
    let dep = in_place_dep("giant", clone.path(), None);

    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
    )
    .unwrap();

    // Placed in the UNVERSIONED slot, with `.git` preserved; no versioned slot.
    let slot = ws_dir.path().join(deps_rel("org.vibevm.giant"));
    assert!(slot.join(".git/HEAD").is_file());
    assert!(slot.join("boot/giant.md").is_file());
    assert!(
        !ws_dir
            .path()
            .join(deps_rel("org.vibevm.giant/1.0.0"))
            .exists()
    );
    assert_eq!(outcome.materialised, vec![deps_rel("org.vibevm.giant")]);

    // The clone source was moved, not copied.
    assert!(!clone.path().join("vibe.toml").exists());

    // `.gitignore` lists the slot — in-place is not vendored (§2.7).
    let gi = fs::read_to_string(ws_dir.path().join(".gitignore")).unwrap();
    assert!(
        gi.contains(&format!("{}/", deps_rel("org.vibevm.giant"))),
        "{gi}"
    );

    // INDEX.md references the UNVERSIONED boot path.
    let index = fs::read_to_string(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap();
    assert!(
        index.contains(&deps_rel("org.vibevm.giant/boot/giant.md")),
        "{index}"
    );
    assert!(!index.contains("feat-giant/1.0.0"), "{index}");
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
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
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    // Pre-place an in-place slot by hand.
    let clone = TempDir::new().unwrap();
    write(clone.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(clone.path(), "src/x", "y");
    vibedeps::materialise_in_place(
        ws_dir.path(),
        &Group::parse("org.vibevm").unwrap(),
        "giant",
        clone.path(),
    )
    .unwrap();

    let ws = Workspace::load(ws_dir.path()).unwrap();
    let outcome = apply_resolution(&ws, &[], SlotIntegrity::TrustPresence).unwrap();

    // The empty resolution prunes nothing in-place; the slot survives.
    assert!(outcome.pruned.is_empty(), "{:?}", outcome.pruned);
    assert!(
        vibedeps::is_in_place_slot(ws_dir.path(), &Group::parse("org.vibevm").unwrap(), "giant"),
        "the in-place slot must survive a prune pass"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION")]
fn pre_install_event_runs_against_an_in_place_slot() {
    let ws = TempDir::new().unwrap();
    let clone = in_place_clone("giant", "");
    let dep = in_place_dep("giant", clone.path(), None);
    let log = EventLog::default();

    let out = materialise_with(ws.path(), &dep, SlotIntegrity::TrustPresence, &log).unwrap();

    assert_eq!(out.materialised, vec![deps_rel("org.vibevm.giant")]);
    assert_eq!(log.events.borrow().as_slice(), ["pre-install"]);
    // The event ran against the unversioned in-place slot, which exists.
    assert!(vibedeps::is_in_place_slot(
        ws.path(),
        &Group::parse("org.vibevm").unwrap(),
        "giant"
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION")]
fn pre_install_failure_rolls_back_an_in_place_slot() {
    let ws = TempDir::new().unwrap();
    let clone = in_place_clone("giant", "");
    let dep = in_place_dep("giant", clone.path(), None);
    let log = EventLog {
        fail_pre: true,
        ..EventLog::default()
    };

    let err = materialise_with(ws.path(), &dep, SlotIntegrity::TrustPresence, &log).unwrap_err();

    assert!(matches!(err, WorkspaceError::SlotLifecycle { .. }), "{err}");
    // The in-place slot is rolled back, not left half-prepared.
    assert!(!vibedeps::is_in_place_slot(
        ws.path(),
        &Group::parse("org.vibevm").unwrap(),
        "giant"
    ));
}

#[test]
fn scoped_subtree_materialisation_does_not_prune_unrelated_slots() {
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
    write(
        stray.path(),
        "vibe.toml",
        "[package]\ngroup = \"org.vibevm\"\nname = \"stray\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    );
    vibedeps::materialise(
        ws.path(),
        &Group::parse("org.vibevm").unwrap(),
        "stray",
        &ver("0.1.0"),
        stray.path(),
        &source_hash(),
    )
    .unwrap();

    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let log = EventLog::default();
    let mut rollback = InstallRollback::new(ws.path());
    let mut out = materialise_subtree_with_spec_format_and_slot_lifecycle(
        ws.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        SpecFormat::Mixed,
        None,
        &log,
        &mut rollback,
    )
    .unwrap();
    assert_eq!(out.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    // The unrelated slot survives — subtree materialisation never prunes.
    assert!(vibedeps::is_materialised(
        ws.path(),
        &Group::parse("org.vibevm").unwrap(),
        "stray",
        &ver("0.1.0")
    ));
    assert!(out.take_post_install_plan().is_some());
    assert!(out.take_post_install_plan().is_none());
}

/// An incremental in-place update (PROP-022 §2.4) has the install layer
/// git-fetch the slot directly, then hand it to the materialise pass with
/// `content_dir` == the slot — the "already placed" signal.
#[cfg(test)]
fn already_placed(changed: bool) -> (TempDir, ResolvedDep) {
    let ws = TempDir::new().unwrap();
    let group = Group::parse("org.vibevm").unwrap();
    let slot = vibedeps::in_place_slot_abs_path(ws.path(), &group, "giant");
    write_in_place_tree(&slot, "giant", "");
    write(&slot, "SENTINEL", "must survive");
    let dep = in_place_dep("giant", &slot, Some(changed));
    (ws, dep)
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#in-place",
    r = 1
)]
fn changed_already_placed_in_place_slot_raises_each_event_once_without_moving() {
    let (ws, dep) = already_placed(true);
    let log = EventLog::default();

    // `Verify` would normally re-materialise — the already-placed signal
    // overrides that for in-place.
    let mut out = materialise_with(ws.path(), &dep, SlotIntegrity::Verify, &log).unwrap();

    assert_eq!(out.materialised, vec![deps_rel("org.vibevm.giant")]);
    assert_eq!(log.events.borrow().as_slice(), ["pre-install"]);
    // The slot was NOT moved/cleared — its sentinel survives.
    assert!(
        dep.content_dir.join("SENTINEL").is_file(),
        "an already-placed in-place slot must not be moved"
    );

    let plan = PostInstallPlan::new(ws.path(), std::mem::take(&mut out.post_install_deps)).unwrap();
    run_post_install_slot_lifecycle(plan, SlotLifecycleMode::Callback(&log)).unwrap();
    assert_eq!(
        log.events.borrow().as_slice(),
        ["pre-install", "post-install"]
    );
}

#[test]
fn unchanged_already_placed_in_place_slot_raises_no_event() {
    let (ws, dep) = already_placed(false);
    let log = EventLog::default();

    let out = materialise_with(ws.path(), &dep, SlotIntegrity::Verify, &log).unwrap();

    assert!(out.materialised.is_empty());
    assert_eq!(out.skipped, [deps_rel("org.vibevm.giant")]);
    assert!(out.post_install_deps.is_empty());
    assert!(log.events.borrow().is_empty());
}
