//! The replay seam republishes the prefix, and against **this** replay's
//! lane.
//!
//! **Why existence proves nothing here, and digests do.** The pending pass
//! that seals the replay has already published a prefix through the
//! ordinary generation seam, so `SYSTEM.md` is on disk before the replay
//! begins. A replay republishes a *changed* `STATIC.*` — that is what a
//! replay is — so the question is not whether a prefix exists but whether
//! it describes the lane the replay produced or the one it replaced. P5
//! closed exactly that seam (`#SYS-TRANSACTION-ROLES`), and this is the
//! test it left open.
//!
//! Both halves of the seam are asserted: the sidecar's witness of the lane,
//! which `replay_prepare/prompt.rs` composes, and the managed block's
//! expected lane digest, which the selector the same cell hands over
//! writes. Drop either and one assertion below fails.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-TRANSACTION-ROLES");

use std::fs;
use std::path::Path;

use vibe_core::layout::{AGENT_LANE_MD, SYSTEM_MD, SYSTEM_META_JSON};

use super::prepare_boot_replay;
use super::tests::{collect_replay, node_file};
use crate::boot_artifacts::native_managed_tests::{FakeProvider, FakeReplayFactory, Reply};
use crate::install::tests_epoch_world::native_freshness::{
    COMPILE_NATIVE, NativeGraph, native_graph_full,
};
use crate::install::tests_epoch_world::write;
use crate::system_prompt::{segment_hash12, sha256_hex};

macro_rules! ok {
    ($value:expr, $message:literal) => {
        match $value {
            Ok(value) => value,
            Err(error) => panic!("{}: {error:?}", $message),
        }
    };
}

/// A node graph that has opted in: an authored template, which is the whole
/// opt-in, plus the one entrypoint the prefix exports and the managed block
/// lives in.
fn opted_in_graph() -> NativeGraph {
    // A graph whose root node really compiles a static lane: a node with
    // only its own authored boot forces every row dynamic and publishes no
    // `STATIC.*` at all, so there would be no lane for a replay to change.
    let graph = native_graph_full(COMPILE_NATIVE, "", "", "");
    let root = graph.workspace.root.clone();
    write(
        &root.join("AGENTS.md"),
        "# Host\n\nAuthored preamble.\n\nENTRYPOINT-BODY\n",
    );
    write(&node_file(&graph, "SYSTEM-TEMPLATE.md"), "# My frame\n");
    graph
}

/// The digest the sidecar witnessed for the lane input — the field that was
/// left describing the previous lane before the seam was closed.
fn witnessed_lane_sha(meta: &Path) -> String {
    let document: serde_json::Value = ok!(
        serde_json::from_slice(&ok!(fs::read(meta), "sidecar")),
        "sidecar JSON"
    );
    document["inputs"]
        .as_array()
        .expect("the sidecar lists its inputs")
        .iter()
        .find(|input| input["role"] == "lane")
        .map(|input| input["sha256"].as_str().unwrap_or_default().to_owned())
        .expect("an opted-in node with a static lane witnesses one")
}

#[test]
fn a_replayed_node_republishes_its_prefix_against_this_replays_lane() {
    let graph = opted_in_graph();

    // The pending pass publishes a prefix of its own, so everything below
    // is a comparison rather than an existence check.
    let mut pending = FakeProvider::new(Reply::Missing);
    let replay = collect_replay(&graph, &mut pending);
    let meta = node_file(&graph, SYSTEM_META_JSON);
    let lane_path = node_file(&graph, "STATIC.md");
    let pending_lane = ok!(fs::read(&lane_path), "pending lane");
    let pending_witness = witnessed_lane_sha(&meta);
    assert_eq!(
        pending_witness,
        sha256_hex(&pending_lane),
        "the pending pass already published a coherent pair"
    );

    let mut factory = FakeReplayFactory::new(Reply::Skip);
    let prepared = ok!(
        prepare_boot_replay(replay, &graph.epoch, &mut factory),
        "prepare replay"
    );
    ok!(
        crate::install::bootgen::replay_publish::publish_boot_replay(prepared),
        "publish replay"
    );

    // The replay really changed the lane; without this the test could pass
    // on a seam that republishes nothing.
    let published_lane = ok!(fs::read(&lane_path), "published lane");
    assert_ne!(
        published_lane, pending_lane,
        "the replay published the same lane, so nothing is being tested"
    );

    // Half one: the sidecar witnesses THIS lane, not the replaced one.
    let witness = witnessed_lane_sha(&meta);
    assert_eq!(witness, sha256_hex(&published_lane));
    assert_ne!(
        witness, pending_witness,
        "the prefix still describes the lane the replay replaced"
    );

    // Half two: the managed block's expected lane digest, which the
    // selector the preparation handed over writes.
    let entrypoint = ok!(
        fs::read_to_string(graph.workspace.root.join("AGENTS.md")),
        "entrypoint"
    );
    let expected = segment_hash12(&published_lane);
    assert!(
        entrypoint.contains(&expected),
        "the router's expected lane digest is not {expected}: {entrypoint}"
    );
    assert!(
        entrypoint.contains("ENTRYPOINT-BODY"),
        "the authored preamble was lost"
    );

    // And the pair is whole: a prefix and a lane file, both republished.
    let prefix = ok!(fs::read_to_string(node_file(&graph, SYSTEM_MD)), "prefix");
    assert!(prefix.contains("My frame"), "the template is in band P");
    assert!(
        prefix.contains(&String::from_utf8_lossy(&published_lane).to_string()),
        "band P carries this replay's lane"
    );
    let agent_lane = ok!(
        fs::read_to_string(node_file(&graph, AGENT_LANE_MD)),
        "lane file"
    );
    assert!(agent_lane.contains(&String::from_utf8_lossy(&published_lane).to_string()));
}
