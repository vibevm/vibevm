use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_spec::CompilerNativePolicy;

use super::{BootReplaySet, PreparedOwnerKind, prepare_boot_replay};
use crate::Workspace;
use crate::boot_artifacts::native_managed_tests::{
    FakePolicyKind, FakeProvider, FakeReplayFactory, Reply,
};
use crate::extension_world::{
    ExtensionWorldEpoch, OwnerRuntimeId, OwnerRuntimeLowering, OwnerRuntimeRunFacts,
    lower_owner_runtimes,
};
use crate::install::tests_epoch_world::native_freshness::{
    COMPILE_NATIVE, NativeGraph, native_graph, native_graph_full, regenerate, unit_file,
};
use crate::install::tests_epoch_world::{locked, resolved, slot, write, write_lock};

macro_rules! ok {
    ($value:expr, $message:literal) => {
        match $value {
            Ok(value) => value,
            Err(error) => panic!("{}: {error:?}", $message),
        }
    };
}
type PolicyMap = BTreeMap<OwnerRuntimeId, CompilerNativePolicy>;
fn owner(graph: &NativeGraph, name: &str) -> OwnerRuntimeId {
    let provider = graph
        .epoch
        .lowered()
        .units()
        .keys()
        .find(|provider| provider.name().as_str() == name)
        .cloned()
        .unwrap_or_else(|| panic!("missing owner {name}"));
    OwnerRuntimeId::Unit { provider }
}
fn root_owner() -> OwnerRuntimeId {
    OwnerRuntimeId::Node {
        rel: ".".to_owned(),
    }
}
fn node_only_graph() -> NativeGraph {
    let root = ok!(TempDir::new(), "node-only workspace");
    write(
        &root.path().join("vibe.toml"),
        &format!(
            "[project]\ngroup='org.demo'\nname='host'\nversion='0.1.0'\n\n\
             [boot]\ndefault_link='static'\n\n{COMPILE_NATIVE}"
        ),
    );
    write(
        &root
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("00-core.md"),
        "# core\n",
    );
    let workspace = ok!(Workspace::load(root.path()), "node-only workspace load");
    let epoch = ok!(
        lower_owner_runtimes(
            &workspace,
            &ExtensionWorldEpoch::empty(),
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        ),
        "node-only runtimes"
    )
    .bind_run(OwnerRuntimeRunFacts {
        run_id: "8123456789abcdef0123456789abcdef".to_owned(),
        state_root: root.path().join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-01T00:00:00Z".to_owned(),
    });
    NativeGraph {
        _root: root,
        workspace,
        resolution: Vec::new(),
        epoch,
        middle: root_owner(),
    }
}
fn diamond_graph(reverse: bool) -> NativeGraph {
    let mut graph = native_graph();
    let root = graph.workspace.root.clone();
    slot(
        &root,
        "diamond",
        "[package]\ngroup='org.lock'\nname='diamond'\nkind='tool'\nversion='1.0.0'\n\n\
         [requires.packages]\n'org.lock/top'={version='=1.0.0',link='static'}\n\
         'org.lock/middle'={version='=1.0.0',link='static'}\n\n\
         [boot_snippet]\nsource='boot/diamond.md'\nlink='static'\n",
    );
    write(
        &crate::vibedeps::slot_abs_path(
            &root,
            &graph.resolution[0].group,
            "diamond",
            &graph.resolution[0].version,
        )
        .join("boot/diamond.md"),
        "# diamond\n",
    );
    graph.resolution.push(resolved(
        &root,
        "diamond",
        "sha256:66",
        &["org.lock/top@=1.0.0", "org.lock/middle@=1.0.0"],
    ));
    if reverse {
        graph.resolution.reverse();
    }
    write_lock(
        &root,
        vec![
            locked("top", "sha256:11", &["org.lock/middle@=1.0.0"]),
            locked("middle", "sha256:22", &["org.lock/leaf@=1.0.0"]),
            locked("leaf", "sha256:33", &[]),
            locked(
                "dynamic",
                "sha256:44",
                &["org.lock/middle@=1.0.0", "org.lock/support@=1.0.0"],
            ),
            locked("support", "sha256:55", &[]),
            locked(
                "diamond",
                "sha256:66",
                &["org.lock/top@=1.0.0", "org.lock/middle@=1.0.0"],
            ),
        ],
    );
    graph.workspace = ok!(Workspace::load(&root), "diamond workspace");
    let world = ok!(
        ExtensionWorldEpoch::from_resolution(&root, &graph.resolution),
        "diamond world"
    );
    graph.epoch = ok!(
        lower_owner_runtimes(
            &graph.workspace,
            &world,
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        ),
        "diamond runtimes"
    )
    .bind_run(OwnerRuntimeRunFacts {
        run_id: "9123456789abcdef0123456789abcdef".to_owned(),
        state_root: root.join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        created_at: "2026-09-01T00:00:00Z".to_owned(),
    });
    graph
}

fn node_file(graph: &NativeGraph, file: &str) -> PathBuf {
    graph
        .workspace
        .root
        .join(vibe_core::layout::current_boot_dir())
        .join(file)
}

fn artifact_bytes(graph: &NativeGraph, owner: &OwnerRuntimeId) -> (Vec<u8>, Vec<u8>) {
    let (index, static_lane) = match owner {
        OwnerRuntimeId::Unit { provider } => (
            unit_file(graph, provider.name().as_str(), "INDEX.md"),
            unit_file(graph, provider.name().as_str(), "STATIC.md"),
        ),
        OwnerRuntimeId::Node { .. } => {
            (node_file(graph, "INDEX.md"), node_file(graph, "STATIC.md"))
        }
    };
    (
        ok!(fs::read(index), "INDEX bytes"),
        ok!(fs::read(static_lane), "STATIC bytes"),
    )
}

fn artifact_state(
    graph: &NativeGraph,
    owners: &[OwnerRuntimeId],
) -> Vec<(Vec<u8>, Vec<u8>, SystemTime, SystemTime)> {
    owners
        .iter()
        .map(|owner| {
            let (index, static_lane) = match owner {
                OwnerRuntimeId::Unit { provider } => (
                    unit_file(graph, provider.name().as_str(), "INDEX.md"),
                    unit_file(graph, provider.name().as_str(), "STATIC.md"),
                ),
                OwnerRuntimeId::Node { .. } => {
                    (node_file(graph, "INDEX.md"), node_file(graph, "STATIC.md"))
                }
            };
            (
                ok!(fs::read(&index), "INDEX bytes"),
                ok!(fs::read(&static_lane), "STATIC bytes"),
                ok!(
                    ok!(fs::metadata(&index), "INDEX metadata").modified(),
                    "INDEX mtime"
                ),
                ok!(
                    ok!(fs::metadata(&static_lane), "STATIC metadata").modified(),
                    "STATIC mtime"
                ),
            )
        })
        .collect()
}

fn collect_replay(graph: &NativeGraph, provider: &mut FakeProvider) -> BootReplaySet {
    ok!(
        regenerate(graph, provider).into_replay_set(&graph.epoch),
        "seal replay"
    )
}

fn prepared_map(
    prepared: &super::PreparedBootReplay,
) -> BTreeMap<OwnerRuntimeId, (Vec<u8>, Vec<u8>)> {
    prepared
        .publications()
        .iter()
        .map(|publication| {
            (
                publication.owner().clone(),
                (
                    publication.index().to_vec(),
                    publication.static_lane().unwrap_or_default().to_vec(),
                ),
            )
        })
        .collect()
}

#[test]
fn empty_replay_is_zero_factory_and_exact_epoch_is_mandatory() {
    let graph = native_graph();
    let mut ready = FakeProvider::new(Reply::Skip);
    let empty = collect_replay(&graph, &mut ready);
    assert!(matches!(empty, BootReplaySet::Empty));
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let prepared = ok!(
        prepare_boot_replay(empty, &graph.epoch, &mut factory),
        "empty replay"
    );
    assert!(prepared.publications().is_empty());
    assert_eq!((factory.creates, factory.finishes), (0, 0));

    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = collect_replay(&graph, &mut pending);
    let equivalent = native_graph();
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    assert!(prepare_boot_replay(replay, &equivalent.epoch, &mut factory).is_err());
    assert_eq!(factory.creates, 0);
}

#[test]
fn direct_unit_static_closure_and_node_prepare_from_overlay_without_writes() {
    let graph = native_graph();
    let middle = owner(&graph, "middle");
    let top = owner(&graph, "top");
    let root = root_owner();
    let expected_order = [middle.clone(), top.clone(), root.clone()];

    let mut ready = FakeProvider::new(Reply::Skip);
    let _ = regenerate(&graph, &mut ready);
    let clean = expected_order
        .iter()
        .map(|owner| (owner.clone(), artifact_bytes(&graph, owner)))
        .collect::<BTreeMap<_, _>>();

    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = collect_replay(&graph, &mut pending);
    let pending_state = artifact_state(&graph, &expected_order);
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let prepared = ok!(
        prepare_boot_replay(replay, &graph.epoch, &mut factory),
        "prepare replay"
    );
    assert_eq!((factory.creates, factory.finishes), (1, 1));
    assert_eq!(factory.visits.as_slice(), std::slice::from_ref(&middle));
    let owners = prepared
        .publications()
        .iter()
        .map(|publication| publication.owner().clone())
        .collect::<Vec<_>>();
    assert_eq!(owners, expected_order);
    assert!(owners.iter().all(|owner| !matches!(owner, OwnerRuntimeId::Unit { provider } if provider.name().as_str() == "dynamic")));
    let prepared_bytes = prepared_map(&prepared);
    assert_eq!(prepared_bytes, clean);
    assert!(prepared.publications().iter().all(|publication| {
        !String::from_utf8_lossy(publication.index()).contains("vibe:native-pending")
            && !String::from_utf8_lossy(publication.static_lane().unwrap_or_default())
                .contains("vibe:transforms-pending")
    }));
    assert!(prepared.publications().iter().all(|publication| {
        publication.index_path().is_absolute()
            && publication.static_path().is_absolute()
            && publication.stale_path().is_absolute()
            && match publication.owner() {
                OwnerRuntimeId::Node { .. } => publication.kind() == PreparedOwnerKind::Node,
                OwnerRuntimeId::Unit { .. } => publication.kind() == PreparedOwnerKind::Unit,
            }
    }));
    assert_eq!(artifact_state(&graph, &expected_order), pending_state);
    assert_ne!(prepared_bytes[&middle].1, pending_state[0].1);
}

const TOP_SELECTOR: &str = "[[extension]]\nid='top-native'\npoint='compile:source'\n\
handler={kind='native',crate_dir='native'}\napplies_to=[['static-xml']]\n";

fn replace_middle_with_collect(policies: &mut BTreeMap<OwnerRuntimeId, CompilerNativePolicy>) {
    let middle = policy_owner(policies, "/middle");
    policies.insert(middle, CompilerNativePolicy::collect());
}

fn replace_top_with_collect(policies: &mut BTreeMap<OwnerRuntimeId, CompilerNativePolicy>) {
    let top = policy_owner(policies, "/top");
    policies.insert(top, CompilerNativePolicy::collect());
}

fn drop_middle(policies: &mut BTreeMap<OwnerRuntimeId, CompilerNativePolicy>) {
    let middle = policy_owner(policies, "/middle");
    policies.remove(&middle);
}

fn policy_owner(policies: &PolicyMap, needle: &str) -> OwnerRuntimeId {
    policies
        .keys()
        .find(|owner| owner.to_string().contains(needle))
        .cloned()
        .unwrap_or_else(|| panic!("missing {needle} policy"))
}

fn add_unused(policies: &mut BTreeMap<OwnerRuntimeId, CompilerNativePolicy>) {
    policies.insert(
        OwnerRuntimeId::Node {
            rel: "unused".to_owned(),
        },
        CompilerNativePolicy::fail(),
    );
}

fn swap_units(policies: &mut BTreeMap<OwnerRuntimeId, CompilerNativePolicy>) {
    let owners = policies
        .keys()
        .filter(|owner| matches!(owner, OwnerRuntimeId::Unit { .. }))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 2);
    let left = policies
        .remove(&owners[0])
        .unwrap_or_else(|| panic!("left policy"));
    let right = policies
        .remove(&owners[1])
        .unwrap_or_else(|| panic!("right policy"));
    policies.insert(owners[0].clone(), right);
    policies.insert(owners[1].clone(), left);
}

#[test]
fn replay_policies_are_resolve_fail_never_collect_and_terminal() {
    let graph = native_graph_full("", COMPILE_NATIVE, COMPILE_NATIVE, "");
    let middle = owner(&graph, "middle");
    let top = owner(&graph, "top");
    let mut pending = FakeProvider::new(Reply::Missing).with_reply(top.clone(), Reply::Skip);
    let replay = collect_replay(&graph, &mut pending);
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    assert!(prepare_boot_replay(replay, &graph.epoch, &mut factory).is_ok());

    for (mutation, reply) in [
        (replace_middle_with_collect as fn(&mut _), Reply::Skip),
        (drop_middle as fn(&mut _), Reply::Skip),
        (add_unused as fn(&mut _), Reply::Skip),
        (replace_middle_with_collect as fn(&mut _), Reply::Missing),
    ] {
        let mut pending = FakeProvider::new(Reply::Missing).with_reply(top.clone(), Reply::Skip);
        let replay = collect_replay(&graph, &mut pending);
        let mut factory = FakeReplayFactory::new(reply).with_mutation(mutation);
        assert!(prepare_boot_replay(replay, &graph.epoch, &mut factory).is_err());
        assert_eq!(factory.finishes, 1);
    }

    let mut pending = FakeProvider::new(Reply::Missing).with_reply(top.clone(), Reply::Skip);
    let replay = collect_replay(&graph, &mut pending);
    let mut wrong_fail = FakeReplayFactory::new(Reply::Skip)
        .with_reply(middle.clone(), Reply::Skip)
        .with_reply(top.clone(), Reply::Missing)
        .with_mutation(replace_top_with_collect);
    assert!(prepare_boot_replay(replay, &graph.epoch, &mut wrong_fail).is_err());
    assert_eq!(wrong_fail.policy_kinds[&middle], FakePolicyKind::Resolve);
    assert_eq!(wrong_fail.policy_kinds[&top], FakePolicyKind::Fail);
}

#[test]
fn two_pending_owners_cannot_swap_and_prepare_once_in_dependency_order() {
    let graph = native_graph_full("", COMPILE_NATIVE, COMPILE_NATIVE, "");
    let middle = owner(&graph, "middle");
    let top = owner(&graph, "top");
    let root = root_owner();
    let mut ready = FakeProvider::new(Reply::Skip);
    let _ = regenerate(&graph, &mut ready);
    let clean_node = artifact_bytes(&graph, &root);
    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = collect_replay(&graph, &mut pending);
    let pending_node = artifact_bytes(&graph, &root);
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let prepared = ok!(
        prepare_boot_replay(replay, &graph.epoch, &mut factory),
        "two owner replay"
    );
    let owners = prepared
        .publications()
        .iter()
        .map(|publication| publication.owner())
        .collect::<Vec<_>>();
    assert_eq!(owners[0..2], [&middle, &top]);
    let prepared_node = prepared_map(&prepared)
        .remove(&root)
        .unwrap_or_else(|| panic!("node"));
    assert_eq!(prepared_node, clean_node);
    assert_ne!(prepared_node.1, pending_node.1);

    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = collect_replay(&graph, &mut pending);
    let mut swapped = FakeReplayFactory::new(Reply::Skip).with_mutation(swap_units);
    assert!(prepare_boot_replay(replay, &graph.epoch, &mut swapped).is_err());
}

#[test]
fn diamond_is_prepared_once_child_first_independent_of_resolution_order() {
    let mut orders = Vec::new();
    for reverse in [false, true] {
        let graph = diamond_graph(reverse);
        let middle = owner(&graph, "middle");
        let top = owner(&graph, "top");
        let diamond = owner(&graph, "diamond");
        let mut pending = FakeProvider::new(Reply::Missing);
        let replay = collect_replay(&graph, &mut pending);
        let mut factory = FakeReplayFactory::new(Reply::Skip);
        let prepared = ok!(
            prepare_boot_replay(replay, &graph.epoch, &mut factory),
            "diamond replay"
        );
        let order = prepared
            .publications()
            .iter()
            .map(|publication| publication.owner().clone())
            .collect::<Vec<_>>();
        assert_eq!(order, [middle, top, diamond]);
        assert_eq!(
            order
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            order.len()
        );
        orders.push(order);
    }
    assert_eq!(orders[0], orders[1]);
}

#[test]
fn direct_node_is_independent_and_last_lane_failure_leaves_disk_and_journals_absent() {
    let direct_node = native_graph_full(COMPILE_NATIVE, "", "", "");
    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = collect_replay(&direct_node, &mut pending);
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let prepared = ok!(
        prepare_boot_replay(replay, &direct_node.epoch, &mut factory),
        "direct node"
    );
    assert_eq!(prepared.publications().len(), 1);
    assert_eq!(prepared.publications()[0].owner(), &root_owner());

    let graph = native_graph_full(COMPILE_NATIVE, TOP_SELECTOR, COMPILE_NATIVE, "");
    let root = root_owner();
    let owners = [owner(&graph, "middle"), owner(&graph, "top"), root.clone()];
    let mut collect = FakeProvider::new(Reply::Missing).with_reply(root.clone(), Reply::Skip);
    let replay = collect_replay(&graph, &mut collect);
    let before = artifact_state(&graph, &owners);
    let mut factory = FakeReplayFactory::new(Reply::Skip).with_reply(root, Reply::Hard);
    assert!(prepare_boot_replay(replay, &graph.epoch, &mut factory).is_err());
    assert_eq!(artifact_state(&graph, &owners), before);
    for owner in owners {
        let directory = match owner {
            OwnerRuntimeId::Unit { provider } => {
                unit_file(&graph, provider.name().as_str(), "INDEX.md")
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_path_buf()
            }
            OwnerRuntimeId::Node { .. } => node_file(&graph, "INDEX.md")
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf(),
        };
        assert!(
            !directory
                .join(".vibe-boot-artifacts.transaction.toml")
                .exists()
        );
        assert!(
            !directory
                .join(".vibe-boot-artifacts.rollback.toml")
                .exists()
        );
    }
}

#[test]
fn markdown_and_xml_replay_targets_are_deterministic() {
    for format in [SpecFormat::Mixed, SpecFormat::Xml] {
        let graph = node_only_graph();
        let mut pending = FakeProvider::new(Reply::Missing);
        let collected = ok!(
            super::super::native_managed::regenerate_boot_from_bound_native(
                &graph.workspace,
                &graph.resolution,
                format,
                None,
                &graph.epoch,
                Some(&mut pending),
            ),
            "format collect"
        );
        let replay = ok!(collected.into_replay_set(&graph.epoch), "format seal");
        let mut factory = FakeReplayFactory::new(Reply::Skip);
        let prepared = ok!(
            prepare_boot_replay(replay, &graph.epoch, &mut factory),
            "format replay"
        );
        assert!(prepared.publications().iter().all(|owner| {
            owner
                .static_path()
                .ends_with(crate::boot_artifacts::static_file(format))
        }));
        let first = prepared_map(&prepared);

        let second_graph = node_only_graph();
        let mut pending = FakeProvider::new(Reply::Missing);
        let collected = ok!(
            super::super::native_managed::regenerate_boot_from_bound_native(
                &second_graph.workspace,
                &second_graph.resolution,
                format,
                None,
                &second_graph.epoch,
                Some(&mut pending),
            ),
            "repeat format collect"
        );
        let replay = ok!(
            collected.into_replay_set(&second_graph.epoch),
            "repeat format seal"
        );
        let mut factory = FakeReplayFactory::new(Reply::Skip);
        let second = ok!(
            prepare_boot_replay(replay, &second_graph.epoch, &mut factory),
            "repeat replay"
        );
        assert_eq!(first, prepared_map(&second));
    }
}

#[test]
fn prepared_type_and_replay_driver_name_no_forbidden_planes() {
    let driver = include_str!("../replay_prepare.rs");
    let prepared = include_str!("prepared.rs");
    let validation = include_str!("validate.rs");
    let receipt_check = validation.find("validate_receipts");
    let evidence_consume = validation.find("lane.direct.take()");
    assert!(receipt_check.is_some() && receipt_check < evidence_consume);
    for forbidden in [
        "write_production",
        "publish_unit_artifacts",
        "TraceRun",
        "CompileObserver",
        "transforms-pending",
        "journal",
        "cargo",
        "lower_owner_runtimes",
        "from_resolution",
    ] {
        assert!(!driver.contains(forbidden));
        assert!(!prepared.contains(forbidden));
    }
}
