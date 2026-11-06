//! The node's side of the de-substitution fixpoint, seen from replay.
//!
//! With one snippet per unit the nested zone top → middle → leaf is
//! covered in the node's lane, and the once-each pass (B-006) rolls BOTH
//! units back to their own snippets. A single pass used to stop halfway:
//! `middle` was rolled back, `top` stayed substituted because `middle`
//! was still substituted in the snapshot it was judged against, and the
//! node's lane carried top's compiled artifact — middle's and leaf's text
//! inside it — beside the rolled-back copies. The node then also waited
//! for a unit artifact it had no reason to embed. The sibling file pins
//! the replay mechanics on a unit that keeps its artifact for a standing
//! reason (a transitively selected unit with several static contributions);
//! this one pins that a covered
//! zone leaves the node out of a unit replay altogether.

use std::fs;

use super::prepare_boot_replay;
use super::tests::{collect_replay, node_file, owner};
use crate::boot_artifacts::native_managed_tests::{FakeProvider, FakeReplayFactory, Reply};
use crate::install::tests_epoch_world::native_freshness::native_graph;

/// B-006 ownership: a covered authored lane is independent of unit output;
/// a retained substituted unit is consumed. The node still runs its own pass.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#UNIT-ARTIFACT-STATIC-CONSUMER-ELIDES"
)]
fn native_unit_bytes_follow_consumed_artifacts_and_node_pass_stays_owned() {
    use crate::extension_world::OwnerRuntimeId;
    use crate::install::tests_epoch_world::native_freshness::{
        COMPILE_NATIVE, native_graph_keeping_top, native_graph_projecting_top, unit_file,
    };
    use vibe_core::manifest::SpecFormat;
    const UNIT: &str = "\n<!-- G3_UNIT_NATIVE_OUTPUT -->\n";
    const NODE: &str = "\n<!-- G3_NODE_NATIVE_OUTPUT -->\n";
    for (retained, format) in [
        (false, SpecFormat::Mixed),
        (true, SpecFormat::Mixed),
        (false, SpecFormat::Markdown),
        (false, SpecFormat::Xml),
    ] {
        let graph = if retained {
            native_graph_keeping_top(COMPILE_NATIVE, COMPILE_NATIVE, "")
        } else {
            native_graph_projecting_top(COMPILE_NATIVE, COMPILE_NATIVE, "")
        };
        let top = owner(&graph, "top");
        let root = OwnerRuntimeId::Node {
            rel: ".".to_owned(),
        };
        let mut provider = FakeProvider::new(Reply::Skip)
            .with_reply(top.clone(), Reply::AppendEmitted(UNIT))
            .with_reply(root.clone(), Reply::AppendEmitted(NODE));
        super::super::native_managed::regenerate_boot_from_bound_native(
            &graph.workspace,
            &graph.resolution,
            format,
            None,
            &graph.epoch,
            Some(&mut provider),
        )
        .unwrap_or_else(|error| panic!("marker compile retained={retained} {format:?}: {error}"));
        let filename = crate::boot_artifacts::static_file(format);
        let unit = fs::read_to_string(unit_file(&graph, "top", filename)).unwrap();
        let node = fs::read_to_string(node_file(&graph, filename)).unwrap();
        assert_eq!(unit.matches("G3_UNIT_NATIVE_OUTPUT").count(), 1, "{unit}");
        assert!(!unit.contains("G3_NODE_NATIVE_OUTPUT"));
        assert_eq!(
            node.matches("G3_UNIT_NATIVE_OUTPUT").count(),
            usize::from(retained),
            "{node}"
        );
        assert_eq!(node.matches("G3_NODE_NATIVE_OUTPUT").count(), 1, "{node}");
        for body in ["top, continued", "top"] {
            assert!(node.contains(body), "{body}: {node}");
        }
        assert_eq!(provider.owners, [top, root]);
        eprintln!(
            "G3 native output retained={retained} {format:?}: unit marker=1; node unit marker={}; node-owned marker=1; native owners={:?}",
            usize::from(retained),
            provider.owners
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#STATIC-EMITS-ONCE-EACH"
)]
fn transitive_suggested_multifragment_unit_retains_exact_generated_dependency() {
    let graph =
        crate::install::tests_epoch_world::native_freshness::native_graph_keeping_top("", "", "");
    let table = super::super::build_unit_table(&graph.workspace.root, &graph.resolution);
    let with_static = super::super::hybrid_emit::with_static_set(&table);
    let dependencies = super::super::node_dependency_boot(
        &graph.workspace.root,
        &graph.workspace.root_manifest,
        &graph.resolution,
        &table,
        &with_static,
        vibe_core::manifest::SpecFormat::Mixed,
    );
    let mut effective = match crate::boot::compute_effective_boot(crate::boot::NodeBootInputs {
        own_boot: &[],
        inherited_foundation: &[],
        dependencies: &dependencies,
        default_link: graph.workspace.root_manifest.boot.default_link,
    }) {
        Ok(boot) => boot,
        Err(error) => panic!("retained unit lane: {error}"),
    };
    super::super::desubstitute_covered_units(&mut effective, &table);
    let read_set = effective
        .entries
        .iter()
        .map(|entry| {
            (
                entry.origin.as_str(),
                entry.path.as_str(),
                entry.unit_substituted,
                entry.link,
            )
        })
        .collect::<Vec<_>>();
    eprintln!("G3 retained-unit node read-set: {read_set:?}");
    assert_eq!(
        super::node_generated_dependencies(&effective, &with_static).as_ref(),
        &[(graph.resolution[0].group.clone(), "top".to_owned()),]
    );
    assert_eq!(
        effective
            .entries
            .iter()
            .filter(|entry| entry.unit_substituted)
            .count(),
        1
    );
    let retained = effective
        .entries
        .iter()
        .find(|entry| entry.unit_substituted)
        .unwrap_or_else(|| panic!("missing retained top"));
    assert_eq!(retained.origin, "org.lock/top");
    assert!(retained.path.ends_with("/boot/STATIC.md"));
}

#[test]
fn a_node_whose_nested_zones_are_covered_is_independent_of_unit_replay() {
    let graph = native_graph();
    let middle = owner(&graph, "middle");
    let top = owner(&graph, "top");

    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = collect_replay(&graph, &mut pending);
    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let prepared = match prepare_boot_replay(replay, &graph.epoch, &mut factory) {
        Ok(prepared) => prepared,
        Err(error) => panic!("covered zones: {error:?}"),
    };
    let owners = prepared
        .publications()
        .iter()
        .map(|publication| publication.owner().clone())
        .collect::<Vec<_>>();
    assert_eq!(
        owners,
        [middle, top],
        "the units are prepared child first; the node embeds no unit artifact"
    );

    let lane = match fs::read(node_file(&graph, "STATIC.md")) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => panic!("node STATIC: {error}"),
    };
    for text in ["# top\n", "# middle\n", "# leaf\n"] {
        assert_eq!(
            lane.matches(text).count(),
            1,
            "`{}` stands once in the node lane:\n{lane}",
            text.trim_end()
        );
    }
}
