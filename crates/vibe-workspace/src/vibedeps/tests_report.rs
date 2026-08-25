//! Reconciliation-report oracles for lifecycle hook policy.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::{ContentHash, Group};

use super::*;

fn group() -> Group {
    Group::parse("org.example").unwrap()
}

fn version() -> semver::Version {
    semver::Version::parse("1.0.0").unwrap()
}

fn source_hash(digit: char) -> ContentHash {
    ContentHash::parse(&format!("sha256:{}", digit.to_string().repeat(64))).unwrap()
}

fn write(root: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn slot(workspace: &Path, name: &str) -> PathBuf {
    slot_abs_path(workspace, &group(), name, &version())
}

fn report(workspace: &Path, name: &str, source: &Path, hash: &ContentHash) -> MaterialiseReport {
    materialise_with_report(
        workspace,
        &group(),
        name,
        &version(),
        source,
        CopyMode::Copy,
        hash,
    )
    .unwrap()
}

fn payload_changed(report: &MaterialiseReport) -> bool {
    report.migrated || !report.written.is_empty() || !report.removed.is_empty()
}

fn changed(report: &MaterialiseReport) -> bool {
    report.identity_changed || payload_changed(report)
}

fn repair_only(report: &MaterialiseReport) -> bool {
    payload_changed(report) && !report.identity_changed
}

#[test]
fn unchanged_forced_reconcile_reports_no_mutations() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "stable.txt", "stable\n");
    report(
        workspace.path(),
        "unchanged-report",
        source.path(),
        &source_hash('1'),
    );

    let report = report(
        workspace.path(),
        "unchanged-report",
        source.path(),
        &source_hash('1'),
    );

    assert_eq!(report.footprint, [PathBuf::from("stable.txt")]);
    assert!(report.written.is_empty());
    assert!(report.removed.is_empty());
    assert!(!report.migrated);
    assert!(!report.identity_changed);
    assert!(!payload_changed(&report));
    assert!(!changed(&report));
    assert!(!repair_only(&report));
}

#[test]
fn source_diff_reports_one_write_and_changed_identity() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "payload.txt", "before\n");
    report(
        workspace.path(),
        "source-diff-report",
        source.path(),
        &source_hash('2'),
    );
    write(source.path(), "payload.txt", "after\n");

    let report = report(
        workspace.path(),
        "source-diff-report",
        source.path(),
        &source_hash('3'),
    );

    assert_eq!(report.written, [PathBuf::from("payload.txt")]);
    assert!(report.removed.is_empty());
    assert!(report.identity_changed);
    assert!(payload_changed(&report));
    assert!(changed(&report));
    assert!(!repair_only(&report));
}

#[test]
fn same_identity_payload_repair_is_reported_as_repair_only() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "payload.txt", "canonical\n");
    let hash = source_hash('4');
    report(workspace.path(), "repair-report", source.path(), &hash);
    write(
        &slot(workspace.path(), "repair-report"),
        "payload.txt",
        "drifted\n",
    );

    let report = report(workspace.path(), "repair-report", source.path(), &hash);

    assert_eq!(report.written, [PathBuf::from("payload.txt")]);
    assert!(report.removed.is_empty());
    assert!(!report.identity_changed);
    assert!(payload_changed(&report));
    assert!(changed(&report));
    assert!(repair_only(&report));
}

#[test]
fn only_stale_paths_actually_deleted_are_reported_as_removed() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "keep.txt", "keep\n");
    write(source.path(), "missing.txt", "stale and already missing\n");
    write(source.path(), "present.txt", "stale and present\n");
    report(
        workspace.path(),
        "removed-report",
        source.path(),
        &source_hash('5'),
    );
    fs::remove_file(source.path().join("missing.txt")).unwrap();
    fs::remove_file(source.path().join("present.txt")).unwrap();
    fs::remove_file(slot(workspace.path(), "removed-report").join("missing.txt")).unwrap();

    let report = report(
        workspace.path(),
        "removed-report",
        source.path(),
        &source_hash('6'),
    );

    assert_eq!(report.footprint, [PathBuf::from("keep.txt")]);
    assert!(report.written.is_empty());
    assert_eq!(report.removed, [PathBuf::from("present.txt")]);
    assert!(payload_changed(&report));
}

#[test]
fn legacy_wipe_reports_migration_and_every_incoming_write() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "a.txt", "a\n");
    write(source.path(), "nested/b.txt", "b\n");
    let slot = slot(workspace.path(), "migration-report");
    write(&slot, "legacy.txt", "legacy\n");

    let report = report(
        workspace.path(),
        "migration-report",
        source.path(),
        &source_hash('7'),
    );

    let incoming = [PathBuf::from("a.txt"), PathBuf::from("nested/b.txt")];
    assert_eq!(report.footprint, incoming);
    assert_eq!(report.written, incoming);
    assert!(report.migrated);
    assert!(report.identity_changed);
    assert!(payload_changed(&report));
    assert!(changed(&report));
    assert!(!repair_only(&report));
    assert!(!slot.join("legacy.txt").exists());
}

#[test]
fn exact_hash_crash_adoption_is_not_reported_as_a_write() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "owned.txt", "owned\n");
    report(
        workspace.path(),
        "adoption-report",
        source.path(),
        &source_hash('8'),
    );
    write(source.path(), "adopted.txt", "already placed\n");
    write(
        &slot(workspace.path(), "adoption-report"),
        "adopted.txt",
        "already placed\n",
    );

    let report = report(
        workspace.path(),
        "adoption-report",
        source.path(),
        &source_hash('9'),
    );

    assert_eq!(
        report.footprint,
        [PathBuf::from("adopted.txt"), PathBuf::from("owned.txt")]
    );
    assert!(report.written.is_empty());
    assert!(report.removed.is_empty());
    assert!(report.identity_changed);
    assert!(!payload_changed(&report));
    assert!(changed(&report));
}
