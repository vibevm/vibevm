//! Payload-diff scheduling oracles for slot events: which materialisation
//! outcomes raise `pre-install` / `post-install`, exactly once, and against a
//! slot that is already isolated from the shared store.

use std::cell::RefCell;
use std::time::{Duration, UNIX_EPOCH};

use specmark::verifies;
use tempfile::TempDir;
use vibe_core::{ContentHash, Group, PackageKind};

use super::test_helpers::*;
use super::*;

/// Records `<package>:<point>` for every slot event. With `mutate` it also
/// writes into the slot at each point, the way a contributed script may.
#[cfg(test)]
#[derive(Default)]
struct EventLog {
    events: RefCell<Vec<String>>,
    mutate: bool,
}

#[cfg(test)]
impl EventLog {
    fn mutating() -> Self {
        Self {
            mutate: true,
            ..Self::default()
        }
    }

    fn events(&self) -> Vec<String> {
        self.events.borrow().clone()
    }

    fn record(
        &self,
        context: SlotLifecycleContext<'_>,
        point: &str,
        relative: &str,
    ) -> Result<(), String> {
        if self.mutate {
            fs::write(context.slot.join(relative), format!("{point} wrote\n"))
                .map_err(|error| error.to_string())?;
        }
        self.events
            .borrow_mut()
            .push(format!("{}:{point}", context.name));
        Ok(())
    }
}

#[cfg(test)]
impl SlotLifecycle for EventLog {
    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        self.record(context, "pre-install", "payload/changed.txt")
    }

    fn post_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String> {
        self.record(context, "post-install", "payload/stable.txt")
    }
}

#[cfg(test)]
struct Verified;

#[cfg(test)]
impl SlotVerifier for Verified {
    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::Verified
    }
}

#[cfg(test)]
struct Diverged;

#[cfg(test)]
impl SlotVerifier for Diverged {
    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::DivergedDetail {
            reason: "fixture payload drift".to_string(),
        }
    }
}

#[cfg(test)]
fn hash(digit: char) -> ContentHash {
    ContentHash::parse(&format!("sha256:{}", digit.to_string().repeat(64))).unwrap()
}

#[cfg(test)]
fn payload_dep(name: &str) -> (ResolvedDep, TempDir) {
    payload_dep_with_mode(name, false)
}

#[cfg(test)]
fn payload_dep_with_mode(name: &str, hardlink: bool) -> (ResolvedDep, TempDir) {
    let source = TempDir::new().unwrap();
    let materialization = if hardlink {
        "materialization = \"hardlink\"\n"
    } else {
        ""
    };
    write(
        source.path(),
        "vibe.toml",
        &format!(
            "[package]\n\
             group = \"org.vibevm\"\n\
             name = \"{name}\"\n\
             kind = \"flow\"\n\
             version = \"1.0.0\"\n\
             {materialization}"
        ),
    );
    write(source.path(), "scripts/finalise.sh", "echo finalise\n");
    write(source.path(), "payload/changed.txt", "canonical\n");
    write(source.path(), "payload/stable.txt", "stable\n");
    let manifest = Manifest::read(source.path().join(Manifest::FILENAME)).unwrap();
    (
        ResolvedDep {
            kind: PackageKind::Flow,
            group: Group::parse("org.vibevm").unwrap(),
            name: name.to_string(),
            version: ver("1.0.0"),
            content_dir: source.path().to_path_buf(),
            source_hash: Some(hash('1')),
            embedded_sources: Vec::new(),
            manifest,
            requires: Vec::new(),
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
            configuration: None,
        },
        source,
    )
}

#[cfg(test)]
fn slot_label(name: &str) -> String {
    deps_rel(format!("org.vibevm.{name}/1.0.0"))
}

#[cfg(test)]
fn slot_path(workspace: &Path, name: &str) -> std::path::PathBuf {
    vibedeps::slot_abs_path(
        workspace,
        &Group::parse("org.vibevm").unwrap(),
        name,
        &ver("1.0.0"),
    )
}

#[cfg(test)]
fn materialise(
    workspace: &Path,
    resolution: &[ResolvedDep],
    slot_integrity: SlotIntegrity,
    slot_verifier: Option<&dyn SlotVerifier>,
    lifecycle: SlotLifecycleMode<'_>,
) -> Materialised {
    materialise_resolution_with_spec_format(
        workspace,
        resolution,
        MaterialiseOptions {
            slot_integrity,
            spec_format: SpecFormat::Mixed,
            slot_verifier,
            lifecycle,
            progress: vibe_core::progress::Progress::default(),
            rollback: None,
        },
    )
    .unwrap()
}

/// First install of `dep`, with no slot lifecycle listening.
#[cfg(test)]
fn seed_slot(workspace: &Path, dep: &ResolvedDep) -> Materialised {
    materialise(
        workspace,
        std::slice::from_ref(dep),
        SlotIntegrity::Verify,
        None,
        SlotLifecycleMode::None,
    )
}

#[cfg(test)]
fn take_plan(workspace: &Path, outcome: &mut Materialised) -> Option<PostInstallPlan> {
    PostInstallPlan::new(workspace, std::mem::take(&mut outcome.post_install_deps))
}

#[cfg(test)]
fn run_post(workspace: &Path, outcome: &mut Materialised, log: &EventLog) {
    run_post_install_slot_lifecycle(
        take_plan(workspace, outcome).expect("a payload change produces a post plan"),
        SlotLifecycleMode::Callback(log),
    )
    .unwrap();
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION")]
fn unchanged_verified_slot_raises_no_event() {
    let workspace = TempDir::new().unwrap();
    let (dep, _source) = payload_dep("steady");
    seed_slot(workspace.path(), &dep);
    let log = EventLog::default();

    let mut outcome = materialise(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&Verified),
        SlotLifecycleMode::Callback(&log),
    );

    assert!(outcome.materialised.is_empty());
    assert_eq!(outcome.skipped, [slot_label("steady")]);
    assert!(outcome.post_install_deps.is_empty());
    assert!(take_plan(workspace.path(), &mut outcome).is_none());
    assert!(log.events().is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION")]
fn verify_heals_one_file_and_raises_each_event_once() {
    let workspace = TempDir::new().unwrap();
    let (dep, _source) = payload_dep("healed");
    seed_slot(workspace.path(), &dep);
    let slot = slot_path(workspace.path(), "healed");
    let stable = slot.join("payload/stable.txt");
    let stable_mtime = fs::metadata(&stable).unwrap().modified().unwrap();
    fs::write(slot.join("payload/changed.txt"), "drifted\n").unwrap();
    let log = EventLog::default();

    let mut outcome = materialise(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&Diverged),
        SlotLifecycleMode::Callback(&log),
    );

    assert_eq!(outcome.materialised, [slot_label("healed")]);
    assert_eq!(outcome.integrity_warnings.len(), 1);
    assert_eq!(
        fs::read_to_string(slot.join("payload/changed.txt")).unwrap(),
        "canonical\n"
    );
    assert_eq!(
        fs::metadata(stable).unwrap().modified().unwrap(),
        stable_mtime
    );
    assert_eq!(log.events(), ["healed:pre-install"]);

    run_post(workspace.path(), &mut outcome, &log);
    assert_eq!(log.events(), ["healed:pre-install", "healed:post-install"]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION")]
fn source_payload_change_raises_each_event_once() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, source) = payload_dep("edited");
    seed_slot(workspace.path(), &dep);
    write(source.path(), "payload/changed.txt", "source changed\n");
    dep.source_hash = Some(hash('2'));
    dep.source_mutable = true;
    let log = EventLog::default();

    let mut outcome = materialise(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
        SlotLifecycleMode::Callback(&log),
    );

    assert_eq!(outcome.materialised, [slot_label("edited")]);
    assert_eq!(log.events(), ["edited:pre-install"]);
    run_post(workspace.path(), &mut outcome, &log);
    assert_eq!(log.events(), ["edited:pre-install", "edited:post-install"]);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION")]
fn two_dependency_pass_raises_events_only_for_the_changed_subset() {
    let workspace = TempDir::new().unwrap();
    let (stable, _stable_source) = payload_dep("stable");
    let (mut changed, changed_source) = payload_dep("changed");
    seed_slot(workspace.path(), &stable);
    seed_slot(workspace.path(), &changed);
    write(
        changed_source.path(),
        "payload/changed.txt",
        "source changed\n",
    );
    changed.source_hash = Some(hash('2'));
    changed.source_mutable = true;
    let log = EventLog::default();

    let mut outcome = materialise(
        workspace.path(),
        &[stable, changed],
        SlotIntegrity::TrustPresence,
        None,
        SlotLifecycleMode::Callback(&log),
    );

    assert_eq!(log.events(), ["changed:pre-install"]);
    run_post(workspace.path(), &mut outcome, &log);
    assert_eq!(
        log.events(),
        ["changed:pre-install", "changed:post-install"]
    );
    assert!(take_plan(workspace.path(), &mut outcome).is_none());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink")]
fn hardlink_to_copy_transition_is_copy_on_write_before_both_events() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, source) = payload_dep_with_mode("hardlinked", true);
    seed_slot(workspace.path(), &dep);
    let preserved_mtime = UNIX_EPOCH + Duration::from_secs(3_000_000);
    fs::File::options()
        .write(true)
        .open(source.path().join("scripts/finalise.sh"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(preserved_mtime))
        .unwrap();
    let manifest_path = source.path().join(Manifest::FILENAME);
    let manifest = fs::read_to_string(&manifest_path).unwrap().replace(
        "materialization = \"hardlink\"",
        "materialization = \"copy\"",
    );
    fs::write(&manifest_path, manifest).unwrap();
    dep.manifest = Manifest::read(&manifest_path).unwrap();
    dep.source_hash = Some(hash('5'));
    dep.source_mutable = true;
    let log = EventLog::mutating();

    let mut outcome = materialise(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        None,
        SlotLifecycleMode::Callback(&log),
    );

    assert_eq!(log.events(), ["hardlinked:pre-install"]);
    let slot = slot_path(workspace.path(), "hardlinked");
    assert_eq!(
        fs::metadata(slot.join("scripts/finalise.sh"))
            .unwrap()
            .modified()
            .unwrap(),
        preserved_mtime,
        "copy-on-write detachment must preserve payload mtime"
    );
    assert_eq!(
        fs::read_to_string(source.path().join("payload/changed.txt")).unwrap(),
        "canonical\n",
        "a pre-install write must not reach the source/cache inode"
    );
    assert_eq!(
        fs::read_to_string(source.path().join("payload/stable.txt")).unwrap(),
        "stable\n"
    );

    run_post(workspace.path(), &mut outcome, &log);
    assert_eq!(
        log.events(),
        ["hardlinked:pre-install", "hardlinked:post-install"]
    );
    assert_eq!(
        fs::read_to_string(source.path().join("payload/changed.txt")).unwrap(),
        "canonical\n"
    );
    assert_eq!(
        fs::read_to_string(source.path().join("payload/stable.txt")).unwrap(),
        "stable\n",
        "a post-install write must not reach the source/cache inode"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#hardlink")]
fn post_event_detaches_hardlinks_even_when_no_pre_event_ran() {
    let workspace = TempDir::new().unwrap();
    let (dep, source) = payload_dep_with_mode("post-only-run", true);
    let mut outcome = seed_slot(workspace.path(), &dep);
    let log = EventLog::mutating();

    run_post(workspace.path(), &mut outcome, &log);

    assert_eq!(log.events(), ["post-only-run:post-install"]);
    assert_eq!(
        fs::read_to_string(source.path().join("payload/stable.txt")).unwrap(),
        "stable\n",
        "post-install must establish its own copy-on-write boundary"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION")]
fn source_payload_removal_raises_each_event_once() {
    let workspace = TempDir::new().unwrap();
    let (mut dep, source) = payload_dep("trimmed");
    seed_slot(workspace.path(), &dep);
    fs::remove_file(source.path().join("payload/changed.txt")).unwrap();
    dep.source_hash = Some(hash('4'));
    dep.source_mutable = true;
    let log = EventLog::default();

    let mut outcome = materialise(
        workspace.path(),
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
        SlotLifecycleMode::Callback(&log),
    );

    assert_eq!(outcome.materialised, [slot_label("trimmed")]);
    assert!(
        !slot_path(workspace.path(), "trimmed")
            .join("payload/changed.txt")
            .exists()
    );
    assert_eq!(log.events(), ["trimmed:pre-install"]);
    run_post(workspace.path(), &mut outcome, &log);
    assert_eq!(
        log.events(),
        ["trimmed:pre-install", "trimmed:post-install"]
    );
}
