//! Filesystem-boundary oracles for lifecycle effective-world loading.

use super::*;

use vibe_core::manifest::LockedPackage;
use vibe_core::{ContentHash, PackageRef, SourceUrl};

fn manifest(body: &str) -> Manifest {
    Manifest::parse_str(body).unwrap()
}

fn locked(group: &str, name: &str, kind: PackageKind) -> LockedPackage {
    LockedPackage {
        kind,
        name: PackageName::parse(name).unwrap(),
        group: Group::parse(group).unwrap(),
        version: "1.0.0".parse().unwrap(),
        registry: None,
        source_url: SourceUrl::new("file:///fixture".to_string()),
        source_ref: None,
        resolved_commit: None,
        content_hash: ContentHash::parse("sha256:00").unwrap(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: Vec::new(),
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

fn workspace(project: &tempfile::TempDir) -> Workspace {
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"host\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    Workspace::load(project.path()).unwrap()
}

fn seed_slot(workspace: &Workspace, package: &LockedPackage) -> PathBuf {
    let root = if package.materialization.is_in_place() {
        in_place_slot_abs_path(&workspace.root, &package.group, &package.name)
    } else {
        slot_abs_path(
            &workspace.root,
            &package.group,
            &package.name,
            &package.version,
        )
    };
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("vibe.toml"),
        format!(
            "[package]\ngroup = \"{}\"\nname = \"{}\"\nkind = \"{}\"\nversion = \"{}\"\n",
            package.group, package.name, package.kind, package.version
        ),
    )
    .unwrap();
    root
}

#[test]
fn host_identity_is_grouped_ungrouped_or_virtual_without_guessing() {
    let root = PathBuf::from("host");
    let grouped = host_source(
        manifest("[project]\nname = \"demo\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n"),
        root.clone(),
    )
    .unwrap();
    assert!(matches!(
        grouped.provider.identity,
        HostIdentity::Coordinate(_)
    ));

    let ungrouped = host_source(
        manifest("[project]\nname = \"Demo Authored Name\"\nversion = \"0.1.0\"\n"),
        root.clone(),
    )
    .unwrap();
    // The shared host-owner codec: an authored name with spaces has exactly
    // one reversible spelling, the same one `ExtensionKey::for_host` and a
    // mechanism provider pin use.
    assert_eq!(
        ungrouped.provider.identity.to_string(),
        "__host__/Demo%20Authored%20Name"
    );

    let virtual_host = host_source(manifest("[workspace]\nmembers = []\n"), root).unwrap();
    assert!(matches!(
        virtual_host.provider.identity,
        HostIdentity::VirtualWorkspace
    ));
    let envelope = envelope_project(&virtual_host.provider, Path::new("C:/virtual"));
    assert_eq!(envelope.name, "<virtual-workspace>");
    assert_eq!(envelope.version, "");
    assert_eq!(envelope.kind, "workspace");
}

#[test]
fn effective_manifest_kind_is_derived_from_the_selected_node_role() {
    let project = manifest("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n");
    assert_eq!(
        effective_manifest_kind(&project),
        EffectiveManifestKind::Project
    );

    let package = manifest(
        "[package]\ngroup = \"org.demo\"\nname = \"tool\"\nkind = \"tool\"\nversion = \"1.0.0\"\n",
    );
    assert_eq!(
        effective_manifest_kind(&package),
        EffectiveManifestKind::Package(PackageKind::Tool)
    );

    let workspace = manifest("[workspace]\nmembers = []\n");
    assert_eq!(
        effective_manifest_kind(&workspace),
        EffectiveManifestKind::VirtualWorkspace
    );
}

#[test]
fn loader_envelope_paths_are_absolute_forward_slashed_machine_json() {
    let project = tempfile::tempdir().unwrap();
    workspace(&project);
    let ritual = plan_default(project.path(), &[Phase::Validate]).unwrap();
    for path in [
        &ritual.project.root,
        &ritual.project.manifest,
        &ritual.project.spec_roots[0],
        &ritual.world.lockfile,
        &ritual.world.deps_root,
    ] {
        assert!(!path.contains('\\'), "{path}");
        assert!(Path::new(path).is_absolute(), "{path}");
    }
}

#[test]
fn copy_is_versioned_while_in_place_is_unversioned() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);

    let copy = locked("org.demo", "copy", PackageKind::Flow);
    let copy_root = seed_slot(&workspace, &copy);
    assert_eq!(
        dependency_source(&workspace, &copy)
            .unwrap()
            .source
            .provider
            .root,
        copy_root
    );
    assert!(copy_root.ends_with("1.0.0"));

    let mut in_place = locked("org.demo", "native", PackageKind::Tool);
    in_place.materialization = Materialization::InPlace;
    let in_place_root = seed_slot(&workspace, &in_place);
    assert_eq!(
        dependency_source(&workspace, &in_place)
            .unwrap()
            .source
            .provider
            .root,
        in_place_root
    );
    assert_eq!(
        in_place_root.file_name().unwrap().to_string_lossy(),
        "org.demo.native"
    );
}

#[test]
fn slot_manifest_kind_must_match_locked_provider_metadata() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    let mut package = locked("org.demo", "kind-drift", PackageKind::Flow);
    seed_slot(&workspace, &package);
    package.kind = PackageKind::Stack;

    let error = dependency_source(&workspace, &package)
        .unwrap_err()
        .to_string();
    assert!(error.contains("declares `flow:"), "{error}");
    assert!(error.contains("requires `stack:"), "{error}");
}

#[test]
fn slot_manifest_coordinate_must_match_the_lock() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    let package = locked("org.demo", "coordinate", PackageKind::Flow);
    let root = seed_slot(&workspace, &package);
    let manifest = root.join("vibe.toml");
    let body = std::fs::read_to_string(&manifest).unwrap().replacen(
        "name = \"coordinate\"",
        "name = \"different\"",
        1,
    );
    std::fs::write(manifest, body).unwrap();

    let error = dependency_source(&workspace, &package)
        .unwrap_err()
        .to_string();
    assert!(error.contains("org.demo/different@1.0.0"), "{error}");
    assert!(error.contains("org.demo/coordinate@1.0.0"), "{error}");
}

#[test]
fn selected_host_closure_filters_unrelated_lock_rows_but_keeps_lock_order() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    let unrelated = locked("org.demo", "unrelated", PackageKind::Flow);
    let b = locked("org.demo", "b", PackageKind::Flow);
    let mut a = locked("org.demo", "a", PackageKind::Flow);
    a.dependencies = vec![PackageRef::parse("org.demo/b@=1.0.0").unwrap()];
    seed_slot(&workspace, &a);
    seed_slot(&workspace, &b);
    // Deliberately do not seed the unrelated slot: selection must not read it.
    let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    lock.packages = vec![b, unrelated, a];
    let host = manifest(
        "[project]\nname = \"host\"\nversion = \"0.1.0\"\n\n[requires.packages]\n\"org.demo/a\" = \"=1.0.0\"\n",
    );
    let sources = dependency_sources(&workspace, &host, &lock, WorldLoadMode::Default).unwrap();
    assert_eq!(
        sources
            .iter()
            .map(|source| source.source.provider.id.name().as_str())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
}

#[test]
fn a_reachable_lock_row_without_its_slot_is_a_loud_partial_world() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    let a = locked("org.demo", "a", PackageKind::Flow);
    let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    lock.packages = vec![a];
    let host = manifest(
        "[project]\nname = \"host\"\nversion = \"0.1.0\"\n\n[requires.packages]\n\"org.demo/a\" = \"=1.0.0\"\n",
    );

    let error = dependency_sources(&workspace, &host, &lock, WorldLoadMode::Default)
        .unwrap_err()
        .to_string();
    assert!(error.contains("has no materialised"), "{error}");
}

#[test]
fn only_pre_clean_may_intersect_a_new_host_root_with_the_old_lock() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    let lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    let host = manifest(
        "[project]\nname = \"host\"\nversion = \"0.1.0\"\n\n[requires.packages]\n\"org.demo/new\" = \"=1.0.0\"\n",
    );

    assert!(
        dependency_sources(&workspace, &host, &lock, WorldLoadMode::PreClean)
            .unwrap()
            .is_empty()
    );
    let error = dependency_sources(&workspace, &host, &lock, WorldLoadMode::Default)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("absent from effective-world lock"),
        "{error}"
    );
    assert!(error.contains("org.demo/new"), "{error}");
}

#[test]
fn an_orphaned_vibedeps_root_without_a_lock_is_loud() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    std::fs::create_dir_all(workspace.vibedeps_root()).unwrap();

    let error = plan_default(project.path(), &[Phase::Build])
        .unwrap_err()
        .to_string();
    assert!(error.contains("exists without"), "{error}");
    assert!(error.contains("vibe.lock"), "{error}");
}

#[test]
fn only_pre_clean_treats_an_absent_root_as_an_empty_installed_world() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname = \"host\"\nversion = \"0.1.0\"\n\n[requires.packages]\n\"org.demo/a\" = \"=1.0.0\"\n",
    )
    .unwrap();
    let a = locked("org.demo", "a", PackageKind::Flow);
    let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    lock.packages = vec![a];
    lock.write(workspace.lockfile_path()).unwrap();

    assert!(plan_clean(project.path()).unwrap().executions.is_empty());
    let error = plan_default(project.path(), &[Phase::Build])
        .unwrap_err()
        .to_string();
    assert!(error.contains("has no materialised"), "{error}");
}

#[test]
fn member_selection_uses_member_requires_and_controls_not_the_workspace_root() {
    let project = tempfile::tempdir().unwrap();
    let member_root = project.path().join("member");
    std::fs::create_dir_all(&member_root).unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        r#"[project]
name = "root"
version = "0.1.0"

[workspace]
members = ["member"]

[requires.packages]
"org.demo/unrelated" = "=1.0.0"

[extensions]
disable = ["org.demo/a#build"]
"#,
    )
    .unwrap();
    std::fs::write(
        member_root.join("vibe.toml"),
        r#"[package]
group = "org.host"
name = "member"
kind = "flow"
version = "0.1.0"

[requires.packages]
"org.demo/a" = "=1.0.0"
"#,
    )
    .unwrap();
    let workspace = Workspace::load(project.path()).unwrap();
    let a = locked("org.demo", "a", PackageKind::Flow);
    let unrelated = locked("org.demo", "unrelated", PackageKind::Flow);
    let a_root = seed_slot(&workspace, &a);
    let mut a_manifest = std::fs::read_to_string(a_root.join("vibe.toml")).unwrap();
    a_manifest.push_str(
        r#"

[[extension]]
id = "build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "build" }
"#,
    );
    std::fs::write(a_root.join("vibe.toml"), a_manifest).unwrap();
    // The root-only dependency intentionally has no slot. Selecting the member
    // must neither load it nor apply the root's disable control.
    let mut lock = Lockfile::empty("test", "2026-08-25T00:00:00Z");
    lock.packages = vec![unrelated, a];
    lock.write(workspace.lockfile_path()).unwrap();

    let ritual = plan_default(&member_root, &[Phase::Build]).unwrap();
    assert_eq!(ritual.executions.len(), 1);
    assert_eq!(
        ritual.executions[0].row.key().to_string(),
        "org.demo/a#build"
    );
}

#[test]
fn package_skill_presets_use_only_selected_member_and_reachable_world() {
    let project = tempfile::tempdir().unwrap();
    let selected = project.path().join("selected");
    let sibling = project.path().join("sibling");
    std::fs::create_dir_all(selected.join("skills/selected")).unwrap();
    std::fs::create_dir_all(sibling.join("skills/sibling")).unwrap();
    std::fs::write(
        project.path().join("vibe.toml"),
        "[project]\nname='root'\nversion='0.1.0'\n[workspace]\nmembers=['selected','sibling']\n",
    )
    .unwrap();
    std::fs::write(
        selected.join("vibe.toml"),
        "[package]\ngroup='org.host'\nname='selected'\nkind='tool'\nversion='0.1.0'\n\
         [[skill]]\nname='selected-skill'\npath='skills/selected'\n",
    )
    .unwrap();
    std::fs::write(selected.join("skills/selected/SKILL.md"), "selected").unwrap();
    std::fs::write(
        sibling.join("vibe.toml"),
        "[package]\ngroup='org.host'\nname='sibling'\nkind='tool'\nversion='0.1.0'\n\
         [[skill]]\nname='sibling-skill'\npath='skills/sibling'\n",
    )
    .unwrap();
    std::fs::write(sibling.join("skills/sibling/SKILL.md"), "sibling").unwrap();

    let workspace = Workspace::load(project.path()).unwrap();
    let unreachable = locked("org.demo", "unreachable", PackageKind::Tool);
    let slot = seed_slot(&workspace, &unreachable);
    std::fs::create_dir_all(slot.join("skills/unreachable")).unwrap();
    let mut manifest = std::fs::read_to_string(slot.join("vibe.toml")).unwrap();
    manifest.push_str("\n[[skill]]\nname='unreachable-skill'\npath='skills/unreachable'\n");
    std::fs::write(slot.join("vibe.toml"), manifest).unwrap();
    std::fs::write(slot.join("skills/unreachable/SKILL.md"), "unreachable").unwrap();
    let mut lock = Lockfile::empty("test", "2026-08-26T00:00:00Z");
    lock.packages = vec![unreachable];
    lock.write(workspace.lockfile_path()).unwrap();

    let ritual = plan_default(&selected, &[Phase::Package]).unwrap();
    assert_eq!(
        ritual
            .package_bindings
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["@vibe/package/skill/org.host/selected/selected-skill"]
    );
    assert!(
        ritual
            .world
            .packages
            .iter()
            .all(|package| package.name != "unreachable")
    );
}

#[test]
fn active_stack_short_name_is_exact_and_ambiguity_is_loud() {
    let project = tempfile::tempdir().unwrap();
    let workspace = workspace(&project);
    let first = locked("org.one", "rust", PackageKind::Stack);
    let second = locked("org.two", "rust", PackageKind::Stack);
    seed_slot(&workspace, &first);
    seed_slot(&workspace, &second);
    let loaded = [
        dependency_source(&workspace, &first).unwrap(),
        dependency_source(&workspace, &second).unwrap(),
    ];
    let installed = loaded
        .iter()
        .map(|dependency| dependency.source.clone())
        .collect::<Vec<_>>();
    let host =
        manifest("[project]\nname = \"host\"\nversion = \"0.1.0\"\n\n[active]\nstack = \"rust\"\n");
    let error = effective_stack(&host, &installed, WorldLoadMode::Default)
        .unwrap_err()
        .to_string();
    assert!(error.contains("ambiguous"), "{error}");

    let host = manifest(
        "[project]\nname = \"host\"\nversion = \"0.1.0\"\n\n[active]\nstack = \"missing\"\n",
    );
    let error = effective_stack(&host, &installed, WorldLoadMode::Default)
        .unwrap_err()
        .to_string();
    assert!(error.contains("names no installed"), "{error}");
    assert_eq!(
        effective_stack(&host, &installed, WorldLoadMode::PreClean).unwrap(),
        None
    );
}
