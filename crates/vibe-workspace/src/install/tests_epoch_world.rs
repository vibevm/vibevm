//! Composed REDs for the durable EPOCH-WORLD projection.

use super::bootgen::read_durable_resolution;
use super::*;

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vibe_core::manifest::{LockedPackage, Lockfile, Materialization, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind, PackageName, PackageRef};

use crate::boot_artifacts::native_managed_tests::{FakeProvider, Reply};
use crate::extension_world::{
    ExtensionWorldEpoch, LoweredOwnerRuntimes, OwnerRuntimeId, OwnerRuntimeLowering,
    OwnerRuntimeRunFacts, collect_owner_view, lower_owner_runtimes,
};
use vibe_extension_registry::ExtensionProvider;

fn bind_epoch(
    lowered: LoweredOwnerRuntimes,
    root: &Path,
) -> crate::extension_world::OwnerRuntimeEpoch {
    lowered.bind_run(OwnerRuntimeRunFacts {
        run_id: "0123456789abcdef0123456789abcdef".to_owned(),
        state_root: root.join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-01T00:00:00Z".to_owned(),
    })
}

#[derive(Debug, PartialEq, Eq)]
struct RuntimeAuthority {
    registry_keys: Vec<String>,
    provider_hashes: Vec<String>,
    compile_keys: Vec<String>,
    plan_digest: Option<String>,
    build_route: String,
    project: (String, String),
    world: Vec<(String, String, String, String)>,
    deps_root: String,
    lockfile: String,
    unit_plans: Vec<(String, Option<String>)>,
}

fn runtime_authority(runtimes: &LoweredOwnerRuntimes) -> RuntimeAuthority {
    let root = runtimes.node(".").expect("root runtime");
    let rows = root.rows().expect("runtime rows");
    RuntimeAuthority {
        registry_keys: root
            .registry()
            .rows()
            .iter()
            .map(|row| row.key().to_string())
            .collect(),
        provider_hashes: root
            .registry()
            .rows()
            .iter()
            .filter_map(|row| match row.provider() {
                ExtensionProvider::Dependency(provider) => Some(provider.content_hash.to_string()),
                ExtensionProvider::Host(_) => None,
            })
            .collect(),
        compile_keys: rows
            .compile()
            .iter()
            .map(|row| row.key().to_string())
            .collect(),
        plan_digest: root.transform_plan().digest_hex(),
        build_route: root
            .routes()
            .get("build:cargo")
            .expect("build route")
            .to_string(),
        project: (
            runtimes.project().name.clone(),
            runtimes.project().root.clone(),
        ),
        world: runtimes
            .world()
            .packages
            .iter()
            .map(|package| {
                (
                    package.group.clone(),
                    package.name.clone(),
                    package.slot.clone(),
                    package.version.clone(),
                )
            })
            .collect(),
        deps_root: runtimes.world().deps_root.clone(),
        lockfile: runtimes.world().lockfile.clone(),
        unit_plans: runtimes
            .units()
            .iter()
            .map(|(owner, runtime)| (owner.to_string(), runtime.transform_plan().digest_hex()))
            .collect(),
    }
}

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

fn resolved(root: &Path, name: &str, hash: &str, dependencies: &[&str]) -> ResolvedDep {
    let slot = crate::vibedeps::slot_abs_path(root, &group(), name, &version());
    ResolvedDep {
        kind: PackageKind::Tool,
        group: group(),
        name: name.to_owned(),
        version: version(),
        content_dir: slot.clone(),
        source_hash: Some(ContentHash::parse(hash).unwrap()),
        manifest: Manifest::read(slot.join(Manifest::FILENAME)).unwrap(),
        requires: dependencies
            .iter()
            .map(|edge| {
                let edge = PackageRef::parse(edge).unwrap();
                (edge.group.expect("grouped"), edge.name.to_string())
            })
            .collect(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    }
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
         [[extensions.use]]\nref = \"org.lock/z-tools#z-row\"\n\n\
         [mechanisms]\n\"build:cargo\" = \"org.vibevm/vibe#cargo\"\n",
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
handler = { kind = "native", crate_dir = "native" }
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
    // Stale ambient authority deliberately disagrees with the Ready overlay.
    write_lock(
        root,
        vec![
            locked("a-tools", "sha256:cc", &[]),
            locked("z-tools", "sha256:dd", &[]),
        ],
    );
    let stale_resolution = read_durable_resolution(root).unwrap();
    assert_eq!(
        stale_resolution
            .iter()
            .map(|dep| dep.name.as_str())
            .collect::<Vec<_>>(),
        ["a-tools", "z-tools"]
    );
    assert_eq!(
        stale_resolution
            .iter()
            .map(|dep| dep.source_hash.as_ref().unwrap().to_string())
            .collect::<Vec<_>>(),
        ["sha256:cc", "sha256:dd"]
    );

    let resolution = vec![
        resolved(root, "z-tools", "sha256:aa", &["org.lock/a-tools@=1.0.0"]),
        resolved(root, "a-tools", "sha256:bb", &[]),
    ];
    assert_eq!(
        resolution
            .iter()
            .map(|dep| dep.name.as_str())
            .collect::<Vec<_>>(),
        ["z-tools", "a-tools"],
        "supplied Ready order is authority and the orphan is absent"
    );
    assert_eq!(
        resolution
            .iter()
            .map(|dep| dep.source_hash.as_ref().unwrap().to_string())
            .collect::<Vec<_>>(),
        ["sha256:aa", "sha256:bb"],
        "Ready provider hashes come from the supplied overlay"
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

    let ready_world = ExtensionWorldEpoch::from_resolution(root, &resolution).unwrap();
    let ready_epoch = bind_epoch(
        lower_owner_runtimes(
            &workspace,
            &ready_world,
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        )
        .unwrap(),
        root,
    );
    for mismatched in [
        vec![resolution[1].clone(), resolution[0].clone()],
        {
            let mut changed = resolution.clone();
            changed[0].source_hash = Some(ContentHash::parse("sha256:ee").unwrap());
            changed
        },
        {
            let mut changed = resolution.clone();
            changed[0].requires.clear();
            changed
        },
        {
            let mut changed = resolution.clone();
            changed[0].manifest.extensions[0].auto = Some(false);
            changed
        },
    ] {
        let mut unused = FakeProvider::new(Reply::Skip);
        assert!(matches!(
            bootgen::native_managed::regenerate_boot_from_bound_native(
                &workspace,
                &mismatched,
                SpecFormat::Xml,
                None,
                &ready_epoch,
                Some(&mut unused),
            ),
            Err(WorkspaceError::OwnerRuntimeResolutionMismatch)
        ));
        assert!(unused.owners.is_empty());
    }
    let mut ready_provider = FakeProvider::new(Reply::Skip);
    let prepared = bootgen::native_managed::regenerate_boot_from_bound_native(
        &workspace,
        &resolution,
        SpecFormat::Xml,
        None,
        &ready_epoch,
        Some(&mut ready_provider),
    )
    .unwrap();
    assert_eq!(prepared.nodes, ["."]);
    assert_eq!(ready_epoch.lowered().units().len(), 2);
    let node_owner = OwnerRuntimeId::Node {
        rel: ".".to_owned(),
    };
    let unit_owner = OwnerRuntimeId::Unit {
        provider: vibe_extension_registry::DependencyProviderId::new(
            group(),
            PackageName::parse("z-tools").unwrap(),
        ),
    };
    assert_eq!(
        ready_provider.owners,
        [unit_owner.clone(), node_owner.clone()]
    );
    assert_eq!(
        prepared.native.keys().collect::<Vec<_>>(),
        [&node_owner, &unit_owner]
    );
    assert!(matches!(
        prepared.native.get(&node_owner),
        Some(crate::boot_artifacts::OwnerNativeCompileContinuation::Ready { .. })
    ));
    assert!(matches!(
        prepared.native.get(&unit_owner),
        Some(crate::boot_artifacts::OwnerNativeCompileContinuation::Ready { .. })
    ));
    let ready_authority = runtime_authority(ready_epoch.lowered());
    assert_eq!(
        ready_authority.registry_keys,
        ["org.lock/z-tools#z-row", "org.lock/a-tools#a-row"]
    );
    assert_eq!(ready_authority.provider_hashes, ["sha256:aa", "sha256:bb"]);
    assert_eq!(
        ready_authority.compile_keys,
        ["org.lock/a-tools#a-row", "org.lock/z-tools#z-row"]
    );
    assert_eq!(ready_authority.build_route, "org.vibevm/vibe#cargo");
    assert_eq!(
        ready_authority
            .world
            .iter()
            .map(|(_, name, _, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["z-tools", "a-tools"]
    );
    let ready_bytes = fs::read(
        root.join(vibe_core::layout::current_boot_dir())
            .join(crate::boot_artifacts::static_file(SpecFormat::Xml)),
    )
    .unwrap();
    write_lock(
        root,
        vec![
            locked("z-tools", "sha256:aa", &["org.lock/a-tools@=1.0.0"]),
            locked("a-tools", "sha256:bb", &[]),
        ],
    );
    let fresh_resolution = read_durable_resolution(root).unwrap();
    let mut reparsed_provider = FakeProvider::new(Reply::Skip);
    let reparsed = bootgen::native_managed::regenerate_boot_from_bound_native(
        &workspace,
        &fresh_resolution,
        SpecFormat::Xml,
        None,
        &ready_epoch,
        Some(&mut reparsed_provider),
    )
    .expect("independently reparsed matching resolution");
    assert_eq!(reparsed.nodes, ["."]);
    let fresh_world = ExtensionWorldEpoch::from_resolution(root, &fresh_resolution).unwrap();
    let fresh_epoch = bind_epoch(
        lower_owner_runtimes(
            &workspace,
            &fresh_world,
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        )
        .unwrap(),
        root,
    );
    let mut fresh_provider = FakeProvider::new(Reply::Skip);
    bootgen::native_managed::regenerate_boot_from_bound_native(
        &workspace,
        &fresh_resolution,
        SpecFormat::Xml,
        None,
        &fresh_epoch,
        Some(&mut fresh_provider),
    )
    .unwrap();
    assert_eq!(runtime_authority(fresh_epoch.lowered()), ready_authority);
    let written = fs::read(
        root.join(vibe_core::layout::current_boot_dir())
            .join(crate::boot_artifacts::static_file(SpecFormat::Xml)),
    )
    .unwrap();
    assert_eq!(
        written, ready_bytes,
        "Fresh and supplied Ready inputs agree"
    );
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
    let analyzed = analyze_node_lane_bound_native(
        &workspace,
        ".",
        &fresh_resolution,
        &fresh_epoch,
        Some(&mut fresh_provider),
        None,
    )
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
