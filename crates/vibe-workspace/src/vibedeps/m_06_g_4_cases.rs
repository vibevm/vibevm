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

fn path_length(path: &Path) -> usize {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str().encode_wide().count()
    }
    #[cfg(not(windows))]
    {
        path.as_os_str().as_encoded_bytes().len()
    }
}

fn nested_root(base: &Path, minimum: usize) -> PathBuf {
    let mut root = base.to_path_buf();
    while path_length(&root) < minimum {
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
    assert!(path_length(&output) > 260);
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
    #[cfg(windows)]
    evidence(
        "long-root",
        serde_json::json!({
            "destination": output, "outcome": "installed", "bytes_verified": true,
            "record_verified": true, "path_utf16_units_excluding_nul": path_length(&output),
        }),
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
    #[cfg(windows)]
    evidence(
        "owned-retry",
        serde_json::json!({
            "short_root": temp.path(), "foreign_preserved": true,
            "collision_refused": true, "owned_cleanup_and_retry": true,
        }),
    );
}

#[cfg(not(windows))]
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

#[cfg(windows)]
fn evidence(name: &str, value: serde_json::Value) {
    eprintln!("G4 {name}: {value}");
    if let Some(directory) = std::env::var_os("VIBE_G4_EVIDENCE_DIR") {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            PathBuf::from(directory).join(format!("{name}.json")),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
}

#[cfg(windows)]
#[test]
fn native_windows_component_limit_refuses_new_slot_before_effect() {
    let temp = TempDir::new().unwrap();
    let project = vibe_safefs::Project::open(temp.path()).unwrap();
    let limit = project
        .windows_maximum_component_length()
        .expect("native volume component limit unavailable; proof remains open")
        as usize;
    // Independent on-volume control: legal ASCII at L succeeds; L+1 fails
    // with an actual OS result, without a newly created directory entry.
    let scratch = temp.path().join("control");
    fs::create_dir(&scratch).unwrap();
    let at_limit = scratch.join("a".repeat(limit));
    fs::create_dir(&at_limit).unwrap();
    let over_limit = scratch.join("b".repeat(limit + 1));
    let native = fs::create_dir(&over_limit).unwrap_err();
    assert!(native.raw_os_error().is_some(), "{native}");
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 1);
    assert_eq!(
        fs::read_dir(&scratch)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path(),
        at_limit
    );
    fs::remove_dir(&at_limit).unwrap();

    let name = vibe_core::PackageName::parse("path-fixture").unwrap();
    let group_length = (limit + 1).checked_sub(name.as_str().len() + 1).unwrap();
    let group = Group::parse(&"g".repeat(group_length)).unwrap();
    let coordinate = format!("{group}/{name}");
    write(temp.path(), "vibe.toml", format!(
        "[project]\nname = \"path-consumer\"\nversion = \"1.1.0\"\n[requires.packages]\n\"{coordinate}\" = \"=1.1.0\"\n"
    ).as_bytes());
    let ws = Workspace::load(temp.path()).unwrap();
    let (source, mut dep) = package();
    dep.group = group;
    dep.name = name.into();
    write(
        source.path(),
        "vibe.toml",
        format!(
            "[package]\ngroup = \"{}\"\nname = \"{}\"\nkind = \"flow\"\nversion = \"1.1.0\"\n",
            dep.group, dep.name
        )
        .as_bytes(),
    );
    dep.manifest = Manifest::read(source.path().join("vibe.toml")).unwrap();
    write(source.path(), "payload", b"selected bytes");
    let source_manifest = fs::read(source.path().join("vibe.toml")).unwrap();
    write(temp.path(), "foreign-marker", b"foreign");
    let destination = slot(&ws, &dep);
    let dependency_parent = temp.path().join("vibevm/vibedeps");
    assert!(!dependency_parent.exists());
    let error = apply(&ws, &dep).unwrap_err().to_string();
    for expected in [
        "path-length refusal during slot directory".to_string(),
        destination.display().to_string(),
        format!("{} UTF-16 code units excluding NUL", limit + 1),
        format!("volume component limit {limit} WCHARs"),
        "exceeds limit by 1 UTF-16 code units".to_string(),
        "Root shortening cannot fix this component".to_string(),
        "no native failure code was generated".to_string(),
    ] {
        assert!(error.contains(&expected), "missing {expected}: {error}");
    }
    assert!(
        !dependency_parent.exists(),
        "new slot parent must remain absent"
    );
    assert!(!destination.exists());
    assert!(!destination.join(vibedeps::SLOT_RECORD_FILENAME).exists());
    assert!(!destination.join("payload").exists());
    assert_eq!(
        fs::read(temp.path().join("foreign-marker")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        fs::read(source.path().join("vibe.toml")).unwrap(),
        source_manifest
    );
    assert_eq!(
        fs::read(source.path().join("payload")).unwrap(),
        b"selected bytes"
    );
    assert_eq!(fs::read_dir(source.path()).unwrap().count(), 2);
    // A separately chosen, parser-valid shorter coordinate can use the same
    // physical source; neither shortening the root nor silently renaming is a fix.
    let (retry_source, retry_dep) = package();
    write(retry_source.path(), "payload", b"selected bytes");
    let retry_ws = workspace(temp.path());
    apply(&retry_ws, &retry_dep).unwrap();
    assert_eq!(
        fs::read(slot(&retry_ws, &retry_dep).join("payload")).unwrap(),
        b"selected bytes"
    );
    assert_eq!(
        fs::read(temp.path().join("foreign-marker")).unwrap(),
        b"foreign"
    );
    evidence(
        "component-limit",
        serde_json::json!({
            "limit": limit, "unit": "WCHAR/UTF-16 code units excluding NUL",
            "query_basis": "GetVolumeInformationByHandleW on held Project directory",
            "control_at_limit_created": true, "control_over_limit_os_code": native.raw_os_error(),
            "control_over_limit_error": native.to_string(), "control_no_created_entry": true,
            "destination": destination, "refusal": error, "new_slot_parent_absent": true,
            "record_and_payload_absent": true, "source_and_foreign_preserved": true,
            "explicit_shorter_coordinate_retry": true, "total_path_limit": null,
        }),
    );
}
