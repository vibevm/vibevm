//! Unit tests for [`super`]: a slot that agrees with its lock row is
//! silent, one that disagrees is one error that names both hashes and the
//! fix, and a project with nothing to compare stays silent.

use std::fs;
use std::path::Path;

use tempfile::tempdir;
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, SpecFormat};
use vibe_core::{ContentHash, Group};
use vibe_workspace::vibedeps::{SLOT_RECORD_SCHEMA, SlotRecord, slot_abs_path, write_slot_record};

use super::LockSlotHashCheck;
use crate::test_support::{opts, write_minimal_project};
use crate::{Check, CheckId, CheckReport, Severity};

const LOCKED: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A lock with two rows, so the finding's field must name the second.
fn write_lock(project: &Path, second_hash: &str) {
    let row = |name: &str, hash: &str| {
        format!(
            "\n[[package]]\nkind = \"flow\"\ngroup = \"org.example\"\nname = \"{name}\"\n\
             version = \"0.1.0\"\nsource_url = \"file:///registry\"\n\
             content_hash = \"{hash}\"\nfiles_written = []\n"
        )
    };
    let text = format!(
        "[meta]\ngenerated_by = \"vibe-test\"\ngenerated_at = \"2026-11-02T00:00:00Z\"\n\
         schema_version = {CURRENT_SCHEMA_VERSION}\n{}{}",
        row("first", LOCKED),
        row("second", second_hash),
    );
    fs::write(project.join("vibe.lock"), text).unwrap();
}

/// A materialised slot whose record says it holds `source_hash`.
fn write_slot(project: &Path, name: &str, source_hash: &str) {
    let group = Group::parse("org.example").unwrap();
    let slot = slot_abs_path(project, &group, name, &semver::Version::new(0, 1, 0));
    fs::create_dir_all(&slot).unwrap();
    let record = SlotRecord {
        schema: SLOT_RECORD_SCHEMA,
        source_hash: ContentHash::parse(source_hash).unwrap(),
        spec_format: SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files: Vec::new(),
    };
    write_slot_record(&slot, &record).unwrap();
}

fn run(project: &Path) -> CheckReport {
    let mut report = CheckReport::default();
    LockSlotHashCheck.run(project, &opts(), &mut report);
    report
}

#[test]
fn slots_that_hold_what_the_lock_pins_are_silent() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    write_lock(project.path(), LOCKED);
    write_slot(project.path(), "first", LOCKED);
    write_slot(project.path(), "second", LOCKED);
    assert!(run(project.path()).findings.is_empty());
}

#[test]
fn a_slot_that_disagrees_with_its_lock_row_is_one_error() {
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    write_lock(project.path(), LOCKED);
    write_slot(project.path(), "first", LOCKED);
    write_slot(project.path(), "second", OTHER);
    let report = run(project.path());
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let finding = &report.findings[0];
    assert_eq!(finding.check, CheckId::LockSlotHash);
    assert_eq!(finding.severity, Severity::Error);
    let path = finding
        .path
        .as_ref()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    assert!(path.ends_with("org.example.second/0.1.0"), "{path}");
    assert_eq!(finding.field.as_deref(), Some("package[1].content_hash"));
    assert_eq!(finding.package.as_deref(), Some("org.example/second@0.1.0"));
    assert!(finding.message.contains(LOCKED) && finding.message.contains(OTHER));
    assert!(finding.message.contains("`vibe update org.example/second`"));
}

#[test]
fn nothing_to_compare_is_nothing_to_report() {
    // No lock at all.
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    assert!(run(project.path()).findings.is_empty());
    // A lock whose packages have no slot record (in-place, legacy, missing).
    write_lock(project.path(), OTHER);
    assert!(run(project.path()).findings.is_empty());
}

#[test]
fn an_in_tree_source_that_moved_on_is_a_warning() {
    // A `local` row: the source lives in the working tree and changes as
    // people edit it, so the drift is reported but does not fail the check.
    let project = tempdir().unwrap();
    write_minimal_project(project.path());
    write_lock(project.path(), LOCKED);
    let lock = project.path().join("vibe.lock");
    let text = fs::read_to_string(&lock).unwrap().replace(
        "name = \"second\"\n",
        "name = \"second\"\nsource_kind = \"local\"\n",
    );
    fs::write(&lock, text).unwrap();
    write_slot(project.path(), "first", LOCKED);
    write_slot(project.path(), "second", OTHER);
    let report = run(project.path());
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert_eq!(report.findings[0].severity, Severity::Warning);
}
