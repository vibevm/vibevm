//! REDs for neutral owner-runtime lowering.

use std::collections::BTreeMap;
use std::fs;

use tempfile::TempDir;
use vibe_core::manifest::{ExtensionKey, Manifest, SpecFormat};
use vibe_extension_registry::{
    ExtensionProvider, HostIdentity, HostProvider, SyntheticPresetSource,
};

use super::test_support::{group, id, node, resolved, slot};
use super::*;
use crate::{Workspace, WorkspaceError};

fn fixture() -> (TempDir, Workspace, Vec<crate::install::ResolvedDep>) {
    let root = TempDir::new().expect("workspace");
    slot(
        root.path(),
        "org.pkgs",
        "tools",
        r#"
[package]
group = "org.pkgs"
name = "tools"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "dep-native"
point = "compile:document"
handler = { kind = "native", crate_dir = "native/dep" }
applies_to = { paths = ["**/*.xml"] }

[[extension]]
id = "phase-native"
point = "phase:build"
handler = { kind = "native", crate_dir = "native/phase" }

[[extension]]
id = "slot-native"
point = "slot:pre-install"
handler = { kind = "native", crate_dir = "native/slot" }

[[mechanism]]
id = "cargo-unit"
role = "build"
name = "cargo"
handler = { kind = "native", crate_dir = "native/cargo-mechanism" }
protocol = 1
config_schema = "schemas/cargo.json"
freshness = "provider"

[mechanisms]
"build:cargo" = "org.pkgs/tools#cargo-unit"
"#,
    );
    node(
        root.path(),
        r#"
[project]
group = "org.demo"
name = "root"
version = "0.1.0"

[workspace]
members = ["members/alpha"]

[requires.packages]
"org.pkgs/tools" = "=1.0.0"
"org.pkgs/aaa-root-only" = "=1.0.0"

[[extension]]
id = "root-minify"
point = "compile:emitted"
handler = { kind = "builtin", name = "xml-minify" }

[[extensions.use]]
ref = "org.pkgs/tools#dep-native"

[[mechanism]]
id = "root-provider"
role = "build"
name = "cargo"
handler = { kind = "script", base = "scripts/root" }
protocol = 1
config_schema = "schemas/root.json"
freshness = "engine"

[mechanisms]
"build:cargo" = "org.demo/root#root-provider"
"#,
    );
    let member = root.path().join("members/alpha");
    fs::create_dir_all(&member).expect("member");
    fs::write(
        member.join(Manifest::FILENAME),
        r#"
[project]
group = "org.demo"
name = "alpha"
version = "0.2.0"

[requires.packages]
"org.pkgs/tools" = "=1.0.0"

[[extension]]
id = "member-minify"
point = "compile:emitted"
handler = { kind = "builtin", name = "xml-minify" }

[[mechanism]]
id = "member-provider"
role = "build"
name = "cargo"
handler = { kind = "script", base = "scripts/member" }
protocol = 1
config_schema = "schemas/member.json"
freshness = "engine"

[mechanisms]
"build:cargo" = "org.demo/alpha#member-provider"
"#,
    )
    .expect("member manifest");
    slot(
        root.path(),
        "org.pkgs",
        "aaa-root-only",
        r#"
[package]
group = "org.pkgs"
name = "aaa-root-only"
kind = "tool"
version = "1.0.0"
"#,
    );
    let workspace = Workspace::load(root.path()).expect("workspace loads");
    let resolution = vec![
        resolved(root.path(), "org.pkgs", "tools", &[]),
        resolved(root.path(), "org.pkgs", "aaa-root-only", &[]),
    ];
    (root, workspace, resolution)
}

fn preset(
    root: &std::path::Path,
    declaration: vibe_core::manifest::ExtensionDecl,
) -> SyntheticPresetSource {
    SyntheticPresetSource {
        key: ExtensionKey::authored("@vibe/member-preset"),
        provider: ExtensionProvider::Host(HostProvider {
            identity: HostIdentity::ungrouped_project("preset"),
            root: root.to_path_buf(),
            version: "1".to_owned(),
            kind: None,
            content_hash: None,
        }),
        declaration,
    }
}

#[test]
fn one_runtime_per_node_and_global_unit_retains_one_row_authority() {
    let (_root, workspace, resolution) = fixture();
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, &resolution).expect("world");
    let member_manifest = workspace
        .iter_nodes()
        .find(|(rel, _)| rel == &"members/alpha")
        .map(|(_, manifest)| manifest)
        .expect("member");
    let mut presets = BTreeMap::new();
    presets.insert(
        "members/alpha".to_owned(),
        vec![preset(
            &workspace.node_abs_path("members/alpha"),
            member_manifest.extensions[0].clone(),
        )],
    );
    let (lowered, events) = super::runtime::observe_lowerings(|| {
        lower_owner_runtimes(
            &workspace,
            &world,
            OwnerRuntimeLowering::new("members/alpha", presets),
        )
    });
    let runtimes = lowered.expect("runtime epoch");
    assert_eq!(
        events,
        [
            OwnerRuntimeId::Node { rel: ".".into() },
            OwnerRuntimeId::Node {
                rel: "members/alpha".into(),
            },
            OwnerRuntimeId::Unit {
                provider: id("org.pkgs", "aaa-root-only"),
            },
            OwnerRuntimeId::Unit {
                provider: id("org.pkgs", "tools"),
            },
        ],
        "each node lowers once in workspace order, then each unit once canonically"
    );

    assert_eq!(runtimes.nodes().len(), 2);
    let expected_tools_slot = vibe_core::machine_json_path(&crate::vibedeps::slot_abs_path(
        &workspace.root,
        &group("org.pkgs"),
        "tools",
        &semver::Version::parse("1.0.0").expect("version"),
    ));
    assert_eq!(
        runtimes.units().len(),
        2,
        "tools lowers once globally despite two consumers, plus root-only"
    );
    assert_eq!(runtimes.project().name, "alpha");
    assert_eq!(
        runtimes.project().root,
        vibe_core::machine_json_path(&workspace.node_abs_path("members/alpha"))
    );
    assert_eq!(
        runtimes
            .world()
            .packages
            .iter()
            .map(|package| (
                package.group.as_str(),
                package.name.as_str(),
                package.slot.as_str(),
                package.version.as_str(),
            ))
            .collect::<Vec<_>>(),
        [("org.pkgs", "tools", expected_tools_slot.as_str(), "1.0.0",)],
        "selected member world excludes the root-only package"
    );
    assert_eq!(
        runtimes.world().deps_root,
        vibe_core::machine_json_path(&workspace.vibedeps_root())
    );
    assert_eq!(
        runtimes.world().lockfile,
        vibe_core::machine_json_path(&workspace.lockfile_path())
    );

    let root = runtimes.node(".").expect("root runtime");
    let member = runtimes.node("members/alpha").expect("member runtime");
    let unit = runtimes
        .unit(&id("org.pkgs", "tools"))
        .expect("unit runtime");
    assert_ne!(
        root.transform_plan().digest_hex(),
        member.transform_plan().digest_hex()
    );
    assert_eq!(
        root.routes()
            .get("build:cargo")
            .expect("root route")
            .to_string(),
        "org.demo/root#root-provider"
    );
    assert_eq!(
        member
            .routes()
            .get("build:cargo")
            .expect("member route")
            .to_string(),
        "org.demo/alpha#member-provider"
    );
    assert_eq!(
        unit.routes()
            .get("build:cargo")
            .expect("unit route")
            .to_string(),
        "org.pkgs/tools#cargo-unit"
    );
    let preset_count = |runtime: &OwnerRuntime| {
        runtime
            .registry()
            .rows()
            .iter()
            .filter(|row| row.key().as_str() == "@vibe/member-preset")
            .count()
    };
    assert_eq!(preset_count(member), 1);
    assert_eq!(preset_count(root), 0);
    assert_eq!(preset_count(unit), 0);

    let root_dependency = root
        .registry()
        .rows()
        .iter()
        .find(|row| row.key().as_str() == "org.pkgs/tools#dep-native")
        .expect("root sees package declaration");
    assert!(matches!(
        root_dependency.provider(),
        ExtensionProvider::Dependency(provider)
            if provider.id == id("org.pkgs", "tools")
    ));
    let unit_own = unit
        .registry()
        .rows()
        .iter()
        .find(|row| row.key().as_str() == "org.pkgs/tools#dep-native")
        .expect("unit sees its own declaration");
    assert!(matches!(
        unit_own.provider(),
        ExtensionProvider::Host(provider)
            if provider.identity
                == HostIdentity::coordinate(id("org.pkgs", "tools"))
    ));
    assert!(
        unit.registry().rows().iter().all(|row| {
            !row.key().as_str().ends_with("#root-minify")
                && !row.key().as_str().ends_with("#member-minify")
        }),
        "node host rows never enter the package owner registry"
    );
    assert!(
        root.mechanisms()
            .find(&"org.demo/root#root-provider".parse().expect("root pin"))
            .is_some()
    );
    assert!(
        member
            .mechanisms()
            .find(
                &"org.demo/alpha#member-provider"
                    .parse()
                    .expect("member pin")
            )
            .is_some()
    );
    assert!(
        unit.mechanisms()
            .find(&"org.pkgs/tools#cargo-unit".parse().expect("unit pin"))
            .is_some()
    );

    let rows = unit.rows().expect("row guards");
    let compile_native = rows
        .compile()
        .iter()
        .find(|row| row.key().as_str().ends_with("#dep-native"))
        .expect("selector compile row retained");
    let candidate_native = rows
        .native()
        .iter()
        .find(|row| row.key().as_str().ends_with("#dep-native"))
        .expect("same native candidate retained");
    assert!(std::ptr::eq(*compile_native, *candidate_native));
    assert!(
        rows.native()
            .iter()
            .any(|row| row.key().as_str().ends_with("#phase-native"))
    );
    assert!(
        rows.native()
            .iter()
            .any(|row| row.key().as_str().ends_with("#slot-native"))
    );
}

#[test]
fn empty_world_and_typed_input_refusals_are_exact() {
    let root = TempDir::new().expect("workspace");
    node(
        root.path(),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n[workspace]\nmembers = [\"members/alpha\"]\n",
    );
    let member = root.path().join("members/alpha");
    fs::create_dir_all(&member).expect("member");
    fs::write(
        member.join(Manifest::FILENAME),
        "[project]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
    )
    .expect("member manifest");
    let workspace = Workspace::load(root.path()).expect("workspace");
    let world = ExtensionWorldEpoch::empty();
    let runtimes = lower_owner_runtimes(
        &workspace,
        &world,
        OwnerRuntimeLowering::new(".", BTreeMap::new()),
    )
    .expect("empty runtime epoch");
    assert_eq!(runtimes.nodes().len(), 2);
    assert!(runtimes.units().is_empty());
    assert!(
        runtimes
            .nodes()
            .values()
            .all(|runtime| runtime.transform_plan().is_empty())
    );

    let error = lower_owner_runtimes(
        &workspace,
        &world,
        OwnerRuntimeLowering::new("missing", BTreeMap::new()),
    )
    .expect_err("unknown selected node refuses");
    assert!(matches!(
        error,
        WorkspaceError::UnknownRuntimeNode {
            role: "selected",
            ..
        }
    ));
    let mut presets = BTreeMap::new();
    presets.insert("missing".to_owned(), Vec::new());
    let error = lower_owner_runtimes(&workspace, &world, OwnerRuntimeLowering::new(".", presets))
        .expect_err("unknown preset node refuses");
    assert!(matches!(
        error,
        WorkspaceError::UnknownRuntimeNode { role: "preset", .. }
    ));
}

#[test]
fn plan_refusals_are_typed_and_bad_units_choose_the_canonical_first_owner() {
    let root = TempDir::new().expect("workspace");
    node(
        root.path(),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n",
    );
    for (name, builtin) in [("zeta", "zetaonly"), ("alpha", "alphaonly")] {
        slot(
            root.path(),
            "org.bad",
            name,
            &format!(
                "[package]\ngroup = \"org.bad\"\nname = \"{name}\"\nkind = \"tool\"\nversion = \"1.0.0\"\n\n[[extension]]\nid = \"bad\"\npoint = \"compile:document\"\nhandler = {{ kind = \"builtin\", name = \"{builtin}\" }}\n"
            ),
        );
    }
    let workspace = Workspace::load(root.path()).expect("workspace");
    let resolution = vec![
        resolved(root.path(), "org.bad", "zeta", &[]),
        resolved(root.path(), "org.bad", "alpha", &[]),
    ];
    let world = ExtensionWorldEpoch::from_resolution(root.path(), &resolution).expect("world");
    let error = lower_owner_runtimes(
        &workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .expect_err("both package-owned plans refuse");
    assert!(matches!(error, WorkspaceError::TransformPlan { .. }));
    let message = error.to_string();
    assert!(message.contains("unit:org.bad/alpha"), "{message}");
    assert!(message.contains("alphaonl"), "{message}");
    assert!(!message.contains("zetaonly"), "{message}");

    let node_root = TempDir::new().expect("node workspace");
    node(
        node_root.path(),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n[[extension]]\nid = \"bad-node\"\npoint = \"compile:document\"\nhandler = { kind = \"builtin\", name = \"nodeonly\" }\n",
    );
    let workspace = Workspace::load(node_root.path()).expect("workspace");
    let error = lower_owner_runtimes(
        &workspace,
        &ExtensionWorldEpoch::empty(),
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .expect_err("node-owned plan refuses typed");
    assert!(matches!(error, WorkspaceError::TransformPlan { .. }));
    assert!(error.to_string().contains("node:."));

    let collection_root = TempDir::new().expect("collection workspace");
    slot(
        collection_root.path(),
        "org.good",
        "tools",
        "[package]\ngroup = \"org.good\"\nname = \"tools\"\nkind = \"tool\"\nversion = \"1.0.0\"\n\n[[extension]]\nid = \"minify\"\npoint = \"compile:emitted\"\nhandler = { kind = \"builtin\", name = \"xml-minify\" }\n",
    );
    node(
        collection_root.path(),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n[requires.packages]\n\"org.good/tools\" = \"=1.0.0\"\n\n[[extensions.use]]\nref = \"org.good/tools#minify\"\n\n[[extensions.use]]\nref = \"org.good/tools#minify\"\n",
    );
    let workspace = Workspace::load(collection_root.path()).expect("workspace");
    let resolution = vec![resolved(collection_root.path(), "org.good", "tools", &[])];
    let world =
        ExtensionWorldEpoch::from_resolution(collection_root.path(), &resolution).expect("world");
    let error = lower_owner_runtimes(
        &workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .expect_err("collection refusal propagates");
    assert!(matches!(error, WorkspaceError::ExtensionWorld { .. }));
    assert!(error.to_string().contains("duplicate [[extensions.use]]"));
}

#[test]
fn prepared_lowering_refusal_publishes_nothing() {
    let root = TempDir::new().expect("workspace");
    node(
        root.path(),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n[[extension]]\nid = \"bad-node\"\npoint = \"compile:document\"\nhandler = { kind = \"builtin\", name = \"nodeonly\" }\n",
    );
    let boot = root.path().join(vibe_core::layout::current_boot_dir());
    fs::create_dir_all(&boot).expect("boot");
    let index = boot.join(crate::boot_artifacts::INDEX_FILE);
    let static_path = boot.join(crate::boot_artifacts::STATIC_FILE);
    fs::write(&index, "OLD INDEX").expect("index");
    fs::write(&static_path, "OLD STATIC").expect("static");
    let workspace = Workspace::load(root.path()).expect("workspace");
    let error = crate::install::regenerate_boot_from_traced_prepared(
        &workspace,
        &[],
        SpecFormat::Mixed,
        None,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .expect_err("runtime lowering refuses before publication");
    assert!(matches!(error, WorkspaceError::TransformPlan { .. }));
    assert_eq!(fs::read_to_string(index).expect("index"), "OLD INDEX");
    assert_eq!(
        fs::read_to_string(static_path).expect("static"),
        "OLD STATIC"
    );
}

#[test]
fn binding_run_is_move_only_and_owner_view_borrows_common_facts() {
    let (_root, workspace, resolution) = fixture();
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, &resolution).expect("world");
    let lowered = lower_owner_runtimes(
        &workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )
    .expect("lowered");
    let epoch = lowered.bind_run(OwnerRuntimeRunFacts {
        run_id: "run-1".to_owned(),
        state_root: workspace.root.clone(),
        platform: "windows-x86_64".to_owned(),
        offline: true,
        created_at: "1970-01-01T00:00:00Z".to_owned(),
    });
    let selected = epoch.selected().expect("selected runtime");
    let unit = epoch
        .unit(&id("org.pkgs", "tools"))
        .expect("unit runtime view");
    assert!(std::ptr::eq(selected.project(), epoch.lowered().project()));
    assert!(std::ptr::eq(selected.world(), epoch.lowered().world()));
    assert!(std::ptr::eq(selected.project(), unit.project()));
    assert!(std::ptr::eq(selected.world(), unit.world()));
    assert_eq!(selected.run().run_id, "run-1");
}

#[test]
fn lowering_observer_clears_its_thread_local_after_unwind() {
    let caught = std::panic::catch_unwind(|| {
        let _ = super::runtime::observe_lowerings(|| -> () { panic!("observed panic") });
    });
    assert!(caught.is_err());

    let root = TempDir::new().expect("workspace");
    node(
        root.path(),
        "[project]\nname = \"root\"\nversion = \"0.1.0\"\n",
    );
    let workspace = Workspace::load(root.path()).expect("workspace");
    let world = ExtensionWorldEpoch::empty();
    let (lowered, events) = super::runtime::observe_lowerings(|| {
        lower_owner_runtimes(
            &workspace,
            &world,
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        )
    });
    lowered.expect("subsequent observation succeeds");
    assert_eq!(
        events,
        [OwnerRuntimeId::Node { rel: ".".into() }],
        "the recovered observation contains only its own successful lowering"
    );
}
