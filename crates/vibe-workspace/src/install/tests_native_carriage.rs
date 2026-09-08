//! Focused carriage proofs for compiler-native INSTALL epochs.

use std::collections::BTreeMap;

use super::native_freshness::{native_graph, unit_file};
use super::*;

#[test]
fn install_entry_returns_the_supplied_epoch_and_all_owner_build_plan() {
    let graph = native_graph();
    let world = ExtensionWorldEpoch::from_resolution(&graph.workspace.root, &graph.resolution)
        .expect("supplied install world");
    let facts = OwnerRuntimeRunFacts {
        run_id: "install-entry-epoch".to_owned(),
        state_root: graph.workspace.root.join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-08T00:00:00Z".to_owned(),
    };
    let mut make_provider = |_policies| Ok(FakeProvider::new(Reply::Missing));
    let (nodes, carriage) = regenerate_boot_from_traced_native(
        &graph.workspace,
        &graph.resolution,
        world,
        SpecFormat::Mixed,
        None,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
        facts,
        &mut make_provider,
    )
    .expect("native-aware install regeneration");
    assert_eq!(nodes, ["."]);
    assert_eq!(carriage.epoch().run().run_id, "install-entry-epoch");
    assert_eq!(carriage.build_owners(), std::slice::from_ref(&graph.middle));
    let index =
        fs::read_to_string(unit_file(&graph, "middle", "INDEX.md")).expect("pending unit index");
    assert!(index.contains("vibe:native-pending"), "{index}");
}

#[test]
fn unselected_member_and_package_unit_enter_the_carried_build_plan() {
    let root = TempDir::new().expect("multi-member native workspace");
    write(
        &root.path().join("vibe.toml"),
        "[project]\ngroup='org.demo'\nname='root'\nversion='0.1.0'\n\
         [workspace]\nmembers=['member']\n\
         [requires.packages]\n'org.demo/member'={version='=0.1.0',link='static'}\n",
    );
    let member_manifest = "[package]\ngroup='org.demo'\nname='member'\nkind='tool'\nversion='0.1.0'\n\
         [boot_snippet]\nsource='boot/member.md'\nlink='static'\n\
         [[extension]]\nid='member-native'\npoint='compile:emitted'\n\
         handler={kind='native',crate_dir='native'}\n";
    write(&root.path().join("member/vibe.toml"), member_manifest);
    write(&root.path().join("member/boot/member.md"), "# member\n");
    let group = Group::parse("org.demo").expect("member group");
    let version = "0.1.0".parse().expect("member version");
    let member_slot = crate::vibedeps::slot_abs_path(root.path(), &group, "member", &version);
    write(&member_slot.join("vibe.toml"), member_manifest);
    write(&member_slot.join("boot/member.md"), "# member\n");
    let workspace = Workspace::load(root.path()).expect("multi-member workspace");
    let resolution = vec![ResolvedDep {
        kind: PackageKind::Tool,
        group,
        name: "member".to_owned(),
        version,
        content_dir: member_slot,
        source_hash: Some(ContentHash::parse("sha256:aa").expect("member hash")),
        manifest: Manifest::read(root.path().join("member/vibe.toml")).expect("member manifest"),
        requires: Vec::new(),
        admitted_by: None,
        via_override: None,
        source_mutable: false,
        in_place_changed: None,
    }];
    let world = ExtensionWorldEpoch::from_resolution(root.path(), &resolution)
        .expect("member dependency world");
    let facts = OwnerRuntimeRunFacts {
        run_id: "multi-member-build-plan".to_owned(),
        state_root: root.path().join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-08T00:00:00Z".to_owned(),
    };
    let mut make_provider = |_policies| Ok(FakeProvider::new(Reply::Missing));
    let (_, carriage) = regenerate_boot_from_traced_native(
        &workspace,
        &resolution,
        world,
        SpecFormat::Mixed,
        None,
        OwnerRuntimeLowering::new(".", BTreeMap::new()),
        facts,
        &mut make_provider,
    )
    .expect("multi-member Collect regeneration");
    assert!(
        carriage
            .build_owners()
            .iter()
            .any(|owner| matches!(owner, OwnerRuntimeId::Node { rel } if rel == "member"))
    );
    assert!(carriage.build_owners().iter().any(|owner| {
        matches!(owner, OwnerRuntimeId::Unit { provider } if provider.to_string() == "org.demo/member")
    }));
}

fn write(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create fixture directory");
    fs::write(path, body).expect("write fixture");
}
