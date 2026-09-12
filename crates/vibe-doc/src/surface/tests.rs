//! What a snapshot is, and what it refuses to be.

use std::path::{Path, PathBuf};

use vibe_wire::generated::doc_surface::DocSurface;

use super::*;

/// The tree this test suite stands in — the one that holds the schemas
/// and the format registry a snapshot reads.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .to_path_buf()
}

/// A program that answers `--help` and is on every machine that runs
/// this suite: the test binary itself. The point of the two runs below
/// is that one binary produces one snapshot, and any binary that answers
/// makes that point.
fn env() -> Option<SurfaceEnv> {
    Some(SurfaceEnv {
        binary: std::env::current_exe().ok()?,
        repo_root: repo_root(),
        timeout_secs: 60,
    })
}

#[test]
fn one_binary_snapshotted_twice_is_the_same_snapshot() {
    let Some(env) = env() else {
        return;
    };
    let first = record("1.0.0", &[], &env).expect("the first snapshot");
    let second = record("1.0.0", &[], &env).expect("the second snapshot");
    assert_eq!(
        to_json(&first),
        to_json(&second),
        "two readings of one product must be one snapshot — a surface that \
         is not a function of the tree makes every diff meaningless"
    );
    assert!(
        !first.manifest_fields.is_empty(),
        "the manifest surface is empty, so the probe learned nothing"
    );
    assert!(
        !first.formats.is_empty(),
        "the format registry is empty, so the registry was not read"
    );
}

#[test]
fn the_version_is_the_callers_word_and_nothing_is_read_for_it() {
    let Some(env) = env() else {
        return;
    };
    let surface = record("whatever-the-owner-said", &[], &env).expect("a snapshot");
    assert_eq!(surface.version, "whatever-the-owner-said");
}

#[test]
fn a_snapshot_carries_no_hash_no_date_and_no_state() {
    let Some(env) = env() else {
        return;
    };
    let text = to_json(&record("1.0.0", &[], &env).expect("a snapshot"));
    let document: serde_json::Value = serde_json::from_str(&text).expect("a JSON document");
    let keys: Vec<&String> = document
        .as_object()
        .expect("an object")
        .keys()
        .collect::<Vec<_>>();
    // The document's OWN members, and not a search of its text: a
    // snapshot legitimately records that some schema has a member named
    // `sha256`, and the rule being kept here is about what the snapshot
    // keys itself on.
    assert_eq!(
        keys,
        vec![
            "commands",
            "facts",
            "formats",
            "lock_fields",
            "manifest_fields",
            "schema_version",
            "schemas",
            "version",
        ],
        "a surface snapshot is keyed on the declared version and nothing else — no \
         hash, no build date, no state identifier"
    );
}

#[test]
fn a_snapshot_goes_to_the_directory_the_site_does_not_render() {
    let package = Path::new("pkg");
    assert_eq!(
        path_for(package, "2.0.0"),
        package.join("maintenance/surface").join("2.0.0.json")
    );
}

#[test]
fn a_package_with_no_snapshots_reports_none_rather_than_refusing() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    assert!(
        recorded(tmp.path()).expect("a reading").is_empty(),
        "a package before its first version bump holds no snapshots, which is a \
         state and not a defect"
    );
}

#[test]
fn recorded_versions_come_back_by_name() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    for version in ["1.0.0", "2.0.0"] {
        write(&surface(version), &path_for(tmp.path(), version)).expect("a written snapshot");
    }
    assert_eq!(
        recorded(tmp.path()).expect("a reading"),
        vec!["1.0.0".to_string(), "2.0.0".to_string()]
    );
    let read_back = read(&path_for(tmp.path(), "2.0.0")).expect("a snapshot");
    assert_eq!(read_back.version, "2.0.0");
}

#[test]
fn a_file_that_is_not_a_snapshot_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let path = path_for(tmp.path(), "1.0.0");
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    std::fs::write(&path, "not json").expect("the file");
    let error = read(&path).expect_err("a refusal").to_string();
    assert!(error.contains("is not a surface snapshot"), "{error}");
    assert!(error.contains("OBS-SURFACE-SNAPSHOTS"), "{error}");
}

#[test]
fn the_text_of_a_fact_is_collapsed_so_rewrapping_is_not_a_change() {
    assert_eq!(
        collapse("a\n  rule   spread\nover lines"),
        "a rule spread over lines"
    );
}

fn surface(version: &str) -> DocSurface {
    DocSurface {
        schema_version: SCHEMA_VERSION,
        version: version.to_owned(),
        commands: Vec::new(),
        manifest_fields: Vec::new(),
        lock_fields: Vec::new(),
        schemas: Vec::new(),
        facts: Vec::new(),
        formats: Vec::new(),
    }
}
