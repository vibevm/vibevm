//! Mutable `file://` source hash-gate oracles (PROP-054 §9.3).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use tempfile::TempDir;
use vibe_core::ContentHash;

use super::test_helpers::*;
use super::*;

fn set_old_mtime(path: &Path, seconds: u64) -> std::time::SystemTime {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(seconds)))
        .unwrap();
    fs::metadata(path).unwrap().modified().unwrap()
}

fn mutable_fixture() -> (Workspace, ResolvedDep, TempDir, TempDir) {
    let workspace_dir = TempDir::new().unwrap();
    write(
        workspace_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(workspace_dir.path(), boot_rel("00-core.md"), "# core");
    let (mut dep, package) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    dep.source_mutable = true;
    let workspace = Workspace::load(workspace_dir.path()).unwrap();
    (workspace, dep, workspace_dir, package)
}

fn slot(workspace: &Workspace, dep: &ResolvedDep) -> PathBuf {
    vibedeps::slot_abs_path(&workspace.root, &dep.group, &dep.name, &dep.version)
}

fn changed_source_hash() -> ContentHash {
    ContentHash::parse("sha256:2222222222222222222222222222222222222222222222222222222222222222")
        .unwrap()
}

struct StubVerifier(SlotCheck);

impl SlotVerifier for StubVerifier {
    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        self.0.clone()
    }
}

#[test]
fn unchanged_mutable_source_skips_when_the_record_hash_matches() {
    let (workspace, dep, _workspace_dir, _package) = mutable_fixture();

    apply_resolution(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    let slot = slot(&workspace, &dep);
    let payload_mtime = set_old_mtime(&slot.join("boot/wal.md"), 1_000_000);
    let record_mtime = set_old_mtime(&slot.join(vibedeps::SLOT_RECORD_FILENAME), 2_000_000);

    let second = apply_resolution(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    assert!(second.materialised.is_empty());
    assert_eq!(second.skipped, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert_eq!(
        fs::metadata(slot.join("boot/wal.md"))
            .unwrap()
            .modified()
            .unwrap(),
        payload_mtime
    );
    assert_eq!(
        fs::metadata(slot.join(vibedeps::SLOT_RECORD_FILENAME))
            .unwrap()
            .modified()
            .unwrap(),
        record_mtime,
        "a hash-earned skip writes neither payload nor record"
    );
}

#[test]
fn changed_mutable_source_reconciles_only_changed_payload() {
    let (workspace, mut dep, _workspace_dir, _package) = mutable_fixture();
    apply_resolution(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    let slot = slot(&workspace, &dep);
    let stable = slot.join("vibe.toml");
    let changed = slot.join("boot/wal.md");
    let record = slot.join(vibedeps::SLOT_RECORD_FILENAME);
    let stable_mtime = set_old_mtime(&stable, 1_000_000);
    let changed_mtime = set_old_mtime(&changed, 2_000_000);
    let record_mtime = set_old_mtime(&record, 3_000_000);

    fs::write(dep.content_dir.join("boot/wal.md"), "# wal edited").unwrap();
    let next_hash = changed_source_hash();
    dep.source_hash = Some(next_hash.clone());

    let outcome = apply_resolution(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    assert_eq!(outcome.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(outcome.skipped.is_empty());
    assert_eq!(
        fs::metadata(&stable).unwrap().modified().unwrap(),
        stable_mtime
    );
    assert_ne!(
        fs::metadata(&changed).unwrap().modified().unwrap(),
        changed_mtime,
        "the changed payload is atomically replaced"
    );
    assert_ne!(
        fs::metadata(&record).unwrap().modified().unwrap(),
        record_mtime,
        "the changed source identity is recorded last"
    );
    assert_eq!(fs::read_to_string(changed).unwrap(), "# wal edited");
    assert_eq!(
        vibedeps::read_slot_record(&slot).unwrap().source_hash,
        next_hash
    );
}

#[test]
fn verify_accepts_an_unchanged_mutable_source_only_after_payload_verification() {
    let (workspace, dep, _workspace_dir, _package) = mutable_fixture();
    apply_resolution(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    let outcome = apply_resolution_with(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&StubVerifier(SlotCheck::Verified)),
        None,
    )
    .unwrap();

    assert!(outcome.materialised.is_empty());
    assert_eq!(outcome.skipped, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(outcome.integrity_warnings.is_empty());
}

#[test]
fn verify_heals_payload_drift_even_when_mutable_source_hash_is_unchanged() {
    let (workspace, dep, _workspace_dir, _package) = mutable_fixture();
    apply_resolution(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    let payload = slot(&workspace, &dep).join("boot/wal.md");
    fs::write(&payload, "corrupt local payload").unwrap();

    let outcome = apply_resolution_with(
        &workspace,
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&StubVerifier(SlotCheck::Diverged {
            expected: "sha256:recorded".to_string(),
            actual: "sha256:drifted".to_string(),
        })),
        None,
    )
    .unwrap();

    assert_eq!(outcome.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(outcome.skipped.is_empty());
    assert_eq!(fs::read_to_string(payload).unwrap(), "# wal");
    assert_eq!(outcome.integrity_warnings.len(), 1);
}
