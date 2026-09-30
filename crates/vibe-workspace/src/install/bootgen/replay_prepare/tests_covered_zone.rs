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
//! reason (several static contributions); this one pins that a covered
//! zone leaves the node out of a unit replay altogether.

use std::fs;

use super::prepare_boot_replay;
use super::tests::{collect_replay, node_file, owner};
use crate::boot_artifacts::native_managed_tests::{FakeProvider, FakeReplayFactory, Reply};
use crate::install::tests_epoch_world::native_freshness::native_graph;

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
