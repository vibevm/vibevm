use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};

use super::*;

fn group(value: &str) -> Group {
    Group::parse(value).unwrap()
}

fn version(value: &str) -> semver::Version {
    semver::Version::parse(value).unwrap()
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn mixed_materialisation_writes_the_slot_record_from_its_footprint() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "README.md", "# Package\n");
    write(source.path(), "nested/data.txt", "payload\n");
    let source_hash = ContentHash::parse(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();

    let footprint = materialise_with_spec_format(
        workspace.path(),
        &group("org.example"),
        "recorded",
        &version("1.0.0"),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Mixed,
        &source_hash,
    )
    .unwrap();

    let slot = slot_abs_path(
        workspace.path(),
        &group("org.example"),
        "recorded",
        &version("1.0.0"),
    );
    let record = read_slot_record(&slot).expect("materialisation writes its slot record last");
    assert_eq!(record.source_hash, source_hash);
    assert_eq!(record.spec_format, SpecFormat::Mixed);
    assert_eq!(
        footprint,
        vec![PathBuf::from("README.md"), PathBuf::from("nested/data.txt")]
    );
    assert_eq!(
        record
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["README.md", "nested/data.txt"]
    );
    for file in &record.files {
        assert_eq!(file.sha256, sha256_file(&slot.join(&file.path)).unwrap());
        assert!(file.source.is_none());
        assert!(file.disposition.is_none());
    }
    assert!(
        record
            .files
            .iter()
            .all(|file| file.path != SLOT_RECORD_FILENAME),
        "the record cannot include its own hash"
    );
}

#[test]
fn mixed_materialisation_orders_rows_by_flattened_slash_path() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    // `guide.md` beside `guide/child.md` — a directory whose name prefixes a
    // sibling file. Host `Path` order compares component-wise and puts
    // `guide/child.md` first (`guide` < `guide.md`); the canonical flattened
    // forward-slash order puts `guide.md` first (`.` sorts before `/`).
    // Path-ordered rows violate `validate_file_rows`' strictly ascending
    // order, so the record write itself would fail.
    write(source.path(), "guide.md", "# Guide\n");
    write(source.path(), "guide/child.md", "# Child\n");
    let source_hash = ContentHash::parse(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();

    materialise(
        workspace.path(),
        &group("org.example"),
        "order",
        &version("1.0.0"),
        source.path(),
        &source_hash,
    )
    .expect("mixed materialisation orders its rows canonically and succeeds");

    let slot = slot_abs_path(
        workspace.path(),
        &group("org.example"),
        "order",
        &version("1.0.0"),
    );
    let record = read_slot_record(&slot).expect("strict read revalidates the record");
    assert_eq!(
        record
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["guide.md", "guide/child.md"],
        "rows must be pinned in ascending flattened forward-slash order"
    );
    verify_recorded_files(&slot, &record).expect("payload verifies against the record");
}

#[test]
fn legacy_slot_receives_a_record_on_explicit_rematerialisation() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "README.md", "replacement\n");
    let slot = slot_abs_path(
        workspace.path(),
        &group("org.example"),
        "legacy",
        &version("1.0.0"),
    );
    write(&slot, "OLD", "legacy payload\n");
    assert!(!slot.join(SLOT_RECORD_FILENAME).exists());

    let source_hash = ContentHash::parse("sha256:aaaaaaaaaaaaaaaa").unwrap();
    materialise(
        workspace.path(),
        &group("org.example"),
        "legacy",
        &version("1.0.0"),
        source.path(),
        &source_hash,
    )
    .unwrap();

    assert!(!slot.join("OLD").exists());
    assert_eq!(read_slot_record(&slot).unwrap().source_hash, source_hash);
}

#[test]
fn authored_root_slot_record_is_reserved() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), SLOT_RECORD_FILENAME, "authored\n");
    let source_hash = ContentHash::parse("sha256:bbbbbbbbbbbbbbbb").unwrap();

    let error = materialise(
        workspace.path(),
        &group("org.example"),
        "reserved",
        &version("1.0.0"),
        source.path(),
        &source_hash,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("reserved for materialisation metadata"),
        "{error}"
    );
}
