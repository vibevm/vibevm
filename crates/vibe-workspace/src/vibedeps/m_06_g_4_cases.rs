//! Disposable real install-composition fixtures. Native Windows is a separate
//! obligation: execution of the POSIX cases is never Windows evidence.

use crate::{
    Workspace, WorkspaceError,
    install::{self, ResolvedDep, SlotLifecycleMode},
    vibedeps,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use vibe_core::{
    ContentHash, Group, PackageKind,
    manifest::{Manifest, SpecFormat},
    user_config::SlotIntegrity,
};

fn write(root: &Path, relative: impl AsRef<Path>, bytes: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).expect("fixture parent creation");
    fs::write(path, bytes).expect("fixture source write");
}

fn workspace(root: &Path) -> Workspace {
    write(root, "vibe.toml", b"[project]\nname = \"path-consumer\"\nversion = \"1.1.0\"\n[requires.packages]\n\"org.example/path-fixture\" = \"=1.1.0\"\n");
    Workspace::load(root).expect("fixture workspace discovery")
}

fn package() -> (TempDir, ResolvedDep) {
    let source = TempDir::new().unwrap();
    write(source.path(), "vibe.toml", b"[package]\ngroup = \"org.example\"\nname = \"path-fixture\"\nkind = \"flow\"\nversion = \"1.1.0\"\n");
    let dep = ResolvedDep {
        kind: PackageKind::Flow,
        group: Group::parse("org.example").unwrap(),
        name: "path-fixture".into(),
        version: semver::Version::parse("1.1.0").unwrap(),
        content_dir: source.path().to_path_buf(),
        source_hash: Some(ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap()),
        embedded_sources: vec![],
        manifest: Manifest::read(source.path().join("vibe.toml")).unwrap(),
        requires: vec![],
        admitted_by: None,
        via_override: None,
        source_mutable: true,
        in_place_changed: None,
    };
    (source, dep)
}

fn apply(ws: &Workspace, dep: &ResolvedDep) -> Result<install::InstallOutcome, WorkspaceError> {
    // The same public composition entry used by vibe-install/src/apply.rs.
    // Verify + no verifier deliberately reconciles rather than trusting presence.
    install::apply_resolution_with_configuration_recovery(
        ws,
        std::slice::from_ref(dep),
        SlotIntegrity::Verify,
        SpecFormat::Mixed,
        None,
        SlotLifecycleMode::None,
        None,
        None,
        None,
    )
}

fn slot(ws: &Workspace, dep: &ResolvedDep) -> PathBuf {
    vibedeps::slot_abs_path(&ws.root, &dep.group, &dep.name, &dep.version)
}

fn nested_root(base: &Path, minimum: usize) -> PathBuf {
    let mut root = base.to_path_buf();
    while root.as_os_str().as_encoded_bytes().len() < minimum {
        root.push("root-segment-".repeat(6));
    }
    fs::create_dir_all(&root).expect("fixture long-root creation");
    root
}

fn long_root_install(minimum: usize) {
    let temp = TempDir::new().unwrap();
    let root = nested_root(temp.path(), minimum);
    let ws = workspace(&root);
    let (source, dep) = package();
    let relative = PathBuf::from(format!(
        "vibevm/vibespecs/{}.xml",
        "long-spec-name-".repeat(12)
    ));
    let bytes =
        b"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Path fixture</title></spec>";
    write(source.path(), &relative, bytes);
    let output = slot(&ws, &dep).join(&relative);
    assert!(output.as_os_str().as_encoded_bytes().len() > 260);
    eprintln!("G4 materialization destination: {}", output.display());
    let result = apply(&ws, &dep).unwrap_or_else(|error| {
        panic!(
            "install composition failed at {}: {error}",
            output.display()
        )
    });
    assert_eq!(
        result.materialised,
        vec![vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)]
    );
    assert_eq!(fs::read(&output).unwrap(), bytes);
    let record = vibedeps::read_slot_record(&slot(&ws, &dep)).unwrap();
    assert_eq!(Some(&record.source_hash), dep.source_hash.as_ref());
    assert!(
        record
            .files
            .iter()
            .any(|row| row.path == relative.to_string_lossy().replace('\\', "/"))
    );
    assert!(
        fs::read_dir(output.parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".vibe-place-"))
    );
}

#[test]
fn ordinary_short_root_and_owned_retry_keep_neighbors() {
    let temp = TempDir::new().unwrap();
    let ws = workspace(temp.path());
    let (source, dep) = package();
    write(
        source.path(),
        "vibevm/vibespecs/guide.xml",
        b"authored payload",
    );
    apply(&ws, &dep).unwrap();
    let destination = slot(&ws, &dep);
    write(&destination, "foreign-output", b"foreign");
    let record_path = destination.join(vibedeps::SLOT_RECORD_FILENAME);
    let record = fs::read(&record_path).unwrap();
    // A real failed reconciliation must not be reported as install success,
    // or erase this occupied, unrecorded path to make retry succeed.
    write(&destination, "new-output", b"foreign occupant");
    write(source.path(), "new-output", b"incoming");
    let error = apply(&ws, &dep).unwrap_err().to_string();
    assert!(error.contains("collides"), "{error}");
    assert_eq!(fs::read(&record_path).unwrap(), record);
    assert_eq!(
        fs::read(destination.join("new-output")).unwrap(),
        b"foreign occupant"
    );
    // The fixture owner releases only its blocker; normal reconciliation retries.
    fs::remove_file(destination.join("new-output")).unwrap();
    apply(&ws, &dep).unwrap();
    assert_eq!(
        fs::read(destination.join("new-output")).unwrap(),
        b"incoming"
    );
    assert_eq!(
        fs::read(destination.join("foreign-output")).unwrap(),
        b"foreign"
    );
    assert!(vibedeps::validate_owned_slot_removal(&destination).is_err());
    fs::remove_file(destination.join("foreign-output")).unwrap();
    vibedeps::validate_owned_slot_removal(&destination).unwrap();
    assert!(vibedeps::remove_slot(&ws.root, &dep.group, &dep.name, &dep.version).unwrap());
    apply(&ws, &dep).unwrap();
    assert_eq!(
        fs::read(destination.join("new-output")).unwrap(),
        b"incoming"
    );
}

#[test]
fn long_root_spec_path_installs_on_this_host() {
    long_root_install(320);
}

#[cfg(windows)]
#[test]
fn native_windows_long_root_spec_path_installs_or_refuses_before_effect() {
    // Success is required for these legal individual components on a supported
    // Windows host. An unrelated error must never count as a portability pass.
    long_root_install(320);
}

#[cfg(target_os = "linux")]
#[test]
fn unsupported_path_diagnostic_is_not_nesting_or_false_success() {
    let temp = TempDir::new().unwrap();
    let root = nested_root(temp.path(), 3850);
    let ws = workspace(&root);
    let (source, dep) = package();
    write(
        source.path(),
        "old-output",
        b"keep until successful reconciliation",
    );
    apply(&ws, &dep).unwrap();
    let destination = slot(&ws, &dep);
    write(&destination, "foreign-output", b"foreign");
    let old_record = fs::read(destination.join(vibedeps::SLOT_RECORD_FILENAME)).unwrap();
    fs::remove_file(source.path().join("old-output")).unwrap();
    let relative = format!("vibevm/vibespecs/{}.xml", "spec-name-".repeat(23));
    write(
        source.path(),
        &relative,
        b"same selected bytes after root shortening",
    );
    let output = destination.join(&relative);
    // Observed native failure, not an injected classifier-only oracle.
    let native = fs::symlink_metadata(&output).unwrap_err();
    assert_eq!(
        native.raw_os_error(),
        Some(36),
        "{}: {native}",
        output.display()
    );
    eprintln!("G4 native probe: {}: {native}", output.display());
    let error = apply(&ws, &dep).unwrap_err().to_string();
    eprintln!("G4 install refusal: {error}");
    for expected in [
        "path-length refusal",
        "payload destination",
        "bytes excluding NUL",
        "Shorten the project root",
        "exact effective path/component limit",
        &output.display().to_string(),
    ] {
        assert!(error.contains(expected), "missing {expected}: {error}");
    }
    assert_eq!(
        fs::read(destination.join(vibedeps::SLOT_RECORD_FILENAME)).unwrap(),
        old_record
    );
    assert_eq!(
        fs::read(destination.join("old-output")).unwrap(),
        b"keep until successful reconciliation"
    );
    assert_eq!(
        fs::read(destination.join("foreign-output")).unwrap(),
        b"foreign"
    );
    let shorter = TempDir::new().unwrap();
    let shorter_ws = workspace(shorter.path());
    apply(&shorter_ws, &dep).unwrap();
    assert_eq!(
        fs::read(slot(&shorter_ws, &dep).join(&relative)).unwrap(),
        b"same selected bytes after root shortening"
    );
    let record = vibedeps::read_slot_record(&slot(&shorter_ws, &dep)).unwrap();
    assert_eq!(Some(&record.source_hash), dep.source_hash.as_ref());
}
