//! Composed REDs for the durable EPOCH-WORLD projection.

use super::bootgen::read_durable_resolution;
use super::*;

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, PackageRef};

use crate::extension_world::{ExtensionWorldEpoch, collect_owner_view};

fn group() -> Group {
    Group::parse("org.lock").unwrap()
}

fn version() -> semver::Version {
    semver::Version::parse("1.0.0").unwrap()
}

fn write(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn slot(root: &Path, name: &str, body: &str) {
    let slot = crate::vibedeps::slot_abs_path(root, &group(), name, &version());
    write(&slot.join(Manifest::FILENAME), body);
}

fn locked(name: &str, hash: &str, dependencies: &[&str]) -> LockedPackage {
    LockedPackage {
        kind: PackageKind::Tool,
        name: PackageName::parse(name).unwrap(),
        group: group(),
        version: version(),
        registry: None,
        source_url: "file:///fixture".into(),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse(hash).unwrap(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: dependencies
            .iter()
            .map(|edge| PackageRef::parse(edge).unwrap())
            .collect(),
        admitted_by: None,
        via_override: None,
        overridden: false,
        source_kind: None,
        via_redirect: None,
        features: Vec::new(),
        subskills_active: Vec::new(),
        describes: None,
        language: None,
        materialization: Materialization::Copy,
    }
}

fn write_lock(root: &Path, packages: Vec<LockedPackage>) {
    let mut lock = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    lock.packages = packages;
    lock.write(root.join(Lockfile::FILENAME)).unwrap();
}

#[test]
fn durable_projection_keeps_lock_order_hash_and_graph_without_slot_records() {
    let workspace = TempDir::new().unwrap();
    let root = workspace.path();
    write(
        &root.join(Manifest::FILENAME),
        "[project]\ngroup = \"org.demo\"\nname = \"host\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.lock/z-tools\" = { version = \"=1.0.0\", link = \"static\" }\n\"org.lock/a-tools\" = { version = \"=1.0.0\", link = \"static\" }\n\n\
         [[extensions.use]]\nref = \"org.lock/a-tools#a-row\"\n\n\
         [[extensions.use]]\nref = \"org.lock/z-tools#z-row\"\n",
    );
    slot(
        root,
        "z-tools",
        r#"
[package]
group = "org.lock"
name = "z-tools"
kind = "tool"
version = "1.0.0"

[requires.packages]
"org.lock/m-orphan" = { version = "=1.0.0", link = "static" }

[boot_snippet]
source = "boot/z.md"
link = "static"

[[extension]]
id = "z-row"
point = "compile:emitted"
handler = { kind = "builtin", name = "xml-minify" }
"#,
    );
    write(
        &crate::vibedeps::slot_abs_path(root, &group(), "z-tools", &version()).join("boot/z.md"),
        "# z tools\n",
    );
    slot(
        root,
        "a-tools",
        r#"
[package]
group = "org.lock"
name = "a-tools"
kind = "tool"
version = "1.0.0"

[boot_snippet]
source = "boot/a.md"
link = "static"

[[extension]]
id = "a-row"
point = "compile:emitted"
handler = { kind = "builtin", name = "xml-minify" }
"#,
    );
    write(
        &crate::vibedeps::slot_abs_path(root, &group(), "a-tools", &version()).join("boot/a.md"),
        "# a tools\n",
    );
    // An orphan slot is deliberately alphabetically between the locked rows.
    // A dependency-root enumeration would admit it; the lock projection must not.
    slot(
        root,
        "m-orphan",
        "[package]\ngroup = \"org.lock\"\nname = \"m-orphan\"\nkind = \"tool\"\nversion = \"1.0.0\"\n\n[boot_snippet]\nsource = \"boot/m.md\"\nlink = \"static\"\n",
    );
    write(
        &crate::vibedeps::slot_abs_path(root, &group(), "m-orphan", &version()).join("boot/m.md"),
        "# m orphan\n",
    );
    write_lock(
        root,
        vec![
            locked("z-tools", "sha256:aa", &["org.lock/a-tools@=1.0.0"]),
            locked("a-tools", "sha256:bb", &[]),
        ],
    );

    let resolution = read_durable_resolution(root).unwrap();
    assert_eq!(
        resolution
            .iter()
            .map(|dep| dep.name.as_str())
            .collect::<Vec<_>>(),
        ["z-tools", "a-tools"],
        "lock order is authority and the orphan is absent"
    );
    assert_eq!(
        resolution
            .iter()
            .map(|dep| dep.source_hash.as_ref().unwrap().to_string())
            .collect::<Vec<_>>(),
        ["sha256:aa", "sha256:bb"],
        "provider hashes come from the lock without any slot record"
    );

    let workspace = Workspace::load(root).unwrap();
    let epoch = ExtensionWorldEpoch::from_resolution(root, &resolution).unwrap();
    let registry = collect_owner_view(
        epoch
            .node_owner_view(root, &workspace.root_manifest)
            .unwrap(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        registry
            .rows()
            .iter()
            .filter(|row| row.provider().is_dependency())
            .map(|row| row.key().as_str())
            .collect::<Vec<_>>(),
        ["org.lock/z-tools#z-row", "org.lock/a-tools#a-row"],
    );

    regenerate_boot_with_spec_format(&workspace, SpecFormat::Xml).unwrap();
    let written = fs::read(
        root.join(vibe_core::layout::current_boot_dir())
            .join(crate::boot_artifacts::static_file(SpecFormat::Xml)),
    )
    .unwrap();
    let header = "<!-- vibe:transforms org.lock/a-tools#a-row org.lock/z-tools#z-row -->";
    assert!(
        String::from_utf8_lossy(&written).contains(header),
        "the transform header independently records authored activation order"
    );
    let written_text = String::from_utf8_lossy(&written);
    let z_frame = written_text.find("vibe:static org.lock/z-tools").unwrap();
    let a_frame = written_text.find("vibe:static org.lock/a-tools").unwrap();
    assert!(
        a_frame < z_frame,
        "the effective lock edge z→a must compile dependency a before z: {written_text}"
    );
    let a_body = written_text
        .find("<title>a tools</title>")
        .expect("the locked dependency body is emitted");
    let z_body = written_text
        .find("<title>z tools</title>")
        .expect("the locked owner body is emitted");
    assert!(
        a_body < z_body,
        "public bodies follow the same dependency-first graph"
    );
    assert!(!written_text.contains("org.lock/m-orphan"));
    let analyzed = analyze_node_lane(&workspace, ".", None)
        .unwrap()
        .expect("the locked lane has static content");
    assert_eq!(analyzed.artifact.bytes(), written);
    assert!(
        String::from_utf8_lossy(analyzed.artifact.bytes()).contains(header),
        "the analyzer preserves the independent activation order"
    );
    let analyzed_text = String::from_utf8_lossy(analyzed.artifact.bytes());
    let analyzed_a = analyzed_text
        .find("vibe:static org.lock/a-tools")
        .expect("the analyzed dependency frame is emitted");
    let analyzed_z = analyzed_text
        .find("vibe:static org.lock/z-tools")
        .expect("the analyzed owner frame is emitted");
    assert!(
        analyzed_a < analyzed_z,
        "the analyzer preserves dependency-first frame order"
    );
    assert!(!analyzed_text.contains("org.lock/m-orphan"));
    assert!(verify_boot_graph(&workspace).unwrap().is_empty());
}

#[test]
fn durable_projection_distinguishes_empty_from_malformed_or_missing_named_state() {
    let empty = TempDir::new().unwrap();
    slot(
        empty.path(),
        "m-orphan",
        "[package]\ngroup = \"org.lock\"\nname = \"m-orphan\"\nkind = \"tool\"\nversion = \"1.0.0\"\n",
    );
    assert!(read_durable_resolution(empty.path()).unwrap().is_empty());

    let malformed = TempDir::new().unwrap();
    fs::write(malformed.path().join(Lockfile::FILENAME), "not a lockfile").unwrap();
    let error = read_durable_resolution(malformed.path()).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::ExtensionWorld { source }
            if matches!(*source, crate::extension_world::ExtensionWorldError::InvalidLock { .. })
    ));

    let nonregular = TempDir::new().unwrap();
    fs::create_dir(nonregular.path().join(Lockfile::FILENAME)).unwrap();
    let error = read_durable_resolution(nonregular.path()).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::ExtensionWorld { source }
            if matches!(*source, crate::extension_world::ExtensionWorldError::NonRegularLock { .. })
    ));

    let missing = TempDir::new().unwrap();
    write_lock(missing.path(), vec![locked("z-tools", "sha256:aa", &[])]);
    assert!(read_durable_resolution(missing.path()).is_err());

    let mismatch = TempDir::new().unwrap();
    slot(
        mismatch.path(),
        "z-tools",
        "[package]\ngroup = \"org.lock\"\nname = \"z-tools\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    );
    write_lock(mismatch.path(), vec![locked("z-tools", "sha256:aa", &[])]);
    assert!(read_durable_resolution(mismatch.path()).is_err());

    let materialization = TempDir::new().unwrap();
    slot(
        materialization.path(),
        "z-tools",
        "[package]\ngroup = \"org.lock\"\nname = \"z-tools\"\nkind = \"tool\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n",
    );
    let orphan =
        crate::vibedeps::in_place_slot_abs_path(materialization.path(), &group(), "z-tools");
    write(
        &orphan.join(Manifest::FILENAME),
        "[package]\ngroup = \"org.lock\"\nname = \"z-tools\"\nkind = \"tool\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n",
    );
    write_lock(
        materialization.path(),
        vec![locked("z-tools", "sha256:aa", &[])],
    );
    let error = read_durable_resolution(materialization.path()).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::ExtensionWorld { source }
            if matches!(
                *source,
                crate::extension_world::ExtensionWorldError::SlotMaterializationMismatch {
                    declared: "in-place",
                    locked: "copy",
                    ..
                }
            )
    ));
}
