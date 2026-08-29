//! Shared fixture scaffolding for the durable world adapter's test cells —
//! split out along the same seam `transform/plan_test_support.rs` cuts, so
//! each assertion cell keeps its own file-length budget while the one world
//! fixture stays spelled once.

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::manifest::{ExtensionKey, LockedPackage, Lockfile, Manifest, Materialization};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, PackageRef};
use vibe_extension_registry::{DependencyProviderId, ExtensionRegistry, ExtensionRegistryRow};

use super::DurableExtensionWorld;
use crate::vibedeps::slot_abs_path;

pub(super) fn group(spelling: &str) -> Group {
    Group::parse(spelling).unwrap()
}

pub(super) fn name(spelling: &str) -> PackageName {
    PackageName::parse(spelling).unwrap()
}

pub(super) fn id(group_spelling: &str, name_spelling: &str) -> DependencyProviderId {
    DependencyProviderId::new(group(group_spelling), name(name_spelling))
}

pub(super) fn version(spelling: &str) -> semver::Version {
    semver::Version::parse(spelling).unwrap()
}

pub(super) fn write(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

/// Materialise one locked package's slot with the manifest body given.
pub(super) fn slot(root: &Path, group_spelling: &str, name_spelling: &str, body: &str) {
    let slot = slot_abs_path(
        root,
        &group(group_spelling),
        name_spelling,
        &version("1.0.0"),
    );
    write(&slot.join(Manifest::FILENAME), body);
}

/// One `[[package]]` lock row at `1.0.0`, with its locked dependency edges.
pub(super) fn locked(group_spelling: &str, name_spelling: &str, edges: &[&str]) -> LockedPackage {
    LockedPackage {
        kind: PackageKind::Tool,
        name: name(name_spelling),
        group: group(group_spelling),
        version: version("1.0.0"),
        registry: None,
        source_url: "file:///fixture".into(),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse("sha256:aa").unwrap(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: edges
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

pub(super) fn lock(packages: Vec<LockedPackage>) -> Lockfile {
    let mut lockfile = Lockfile::empty("fixture", "1970-01-01T00:00:00Z");
    lockfile.packages = packages;
    lockfile
}

/// Write the selected node's own manifest and parse it back, so the fixture
/// exercises the real manifest grammar rather than a struct literal.
pub(super) fn node(root: &Path, body: &str) -> Manifest {
    write(&root.join(Manifest::FILENAME), body);
    Manifest::read(root.join(Manifest::FILENAME)).unwrap()
}

pub(super) fn key(spelling: &str) -> ExtensionKey {
    ExtensionKey::authored(spelling)
}

pub(super) fn keys(rows: impl IntoIterator<Item = String>) -> Vec<String> {
    rows.into_iter().collect()
}

pub(super) fn row<'registry>(
    registry: &'registry ExtensionRegistry,
    suffix: &str,
) -> &'registry ExtensionRegistryRow {
    registry
        .rows()
        .iter()
        .find(|row| row.key().as_str().ends_with(suffix))
        .unwrap_or_else(|| panic!("row `{suffix}` exists in this owner's registry"))
}

pub(super) fn found(registry: &ExtensionRegistry, suffix: &str) -> bool {
    registry
        .rows()
        .iter()
        .any(|row| row.key().as_str().ends_with(suffix))
}

/// The shared three-package world.
///
/// Lock order is `org.zed/z-tools`, `org.mid/m-tools`, `org.aaa/a-tools` —
/// the exact REVERSE of the alphabetical order of every one of its
/// coordinates, so any order this world produces that happens to be sorted is
/// a sort, not the lock.
///
/// * the node requires all three, declares `#node-doc`, and disables
///   `org.aaa/a-tools#a-loud`;
/// * `org.zed/z-tools` requires `org.aaa/a-tools`, declares `#z-src`, and
///   carries its OWN activation of `org.aaa/a-tools#a-src`;
/// * `org.mid/m-tools` declares `#m-src` and is reachable from the node only;
/// * `org.aaa/a-tools` declares `#a-src` and `#a-loud` and controls nothing.
pub(super) fn fixture() -> (TempDir, Manifest, Lockfile) {
    let workspace = TempDir::new().unwrap();
    let root = workspace.path();

    slot(
        root,
        "org.zed",
        "z-tools",
        r#"
[package]
group = "org.zed"
name = "z-tools"
kind = "tool"
version = "1.0.0"

[requires.packages]
"org.aaa/a-tools" = "=1.0.0"

[[extension]]
id = "z-src"
point = "compile:source"
handler = { kind = "builtin", name = "log" }

[[extensions.use]]
ref = "org.aaa/a-tools#a-src"
"#,
    );
    slot(
        root,
        "org.mid",
        "m-tools",
        r#"
[package]
group = "org.mid"
name = "m-tools"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "m-src"
point = "compile:source"
handler = { kind = "builtin", name = "log" }
"#,
    );
    slot(
        root,
        "org.aaa",
        "a-tools",
        r#"
[package]
group = "org.aaa"
name = "a-tools"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "a-src"
point = "compile:source"
handler = { kind = "builtin", name = "log" }

[[extension]]
id = "a-loud"
point = "phase:test"
handler = { kind = "builtin", name = "log" }
"#,
    );

    let manifest = node(
        root,
        r#"
[project]
name = "demo"
version = "0.1.0"

[requires.packages]
"org.zed/z-tools" = "=1.0.0"
"org.mid/m-tools" = "=1.0.0"
"org.aaa/a-tools" = "=1.0.0"

[[extension]]
id = "node-doc"
point = "compile:document"
handler = { kind = "builtin", name = "log" }

[extensions]
disable = ["org.aaa/a-tools#a-loud"]
"#,
    );

    let lockfile = lock(vec![
        locked("org.zed", "z-tools", &["org.aaa/a-tools@=1.0.0"]),
        locked("org.mid", "m-tools", &[]),
        locked("org.aaa", "a-tools", &[]),
    ]);
    (workspace, manifest, lockfile)
}

pub(super) fn world(
    workspace: &TempDir,
    manifest: &Manifest,
    lockfile: &Lockfile,
) -> DurableExtensionWorld {
    DurableExtensionWorld::from_lock(workspace.path(), workspace.path(), manifest, lockfile)
        .unwrap()
}
