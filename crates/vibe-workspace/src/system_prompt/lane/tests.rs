//! The agent lane: marker pairing and uniqueness, INDEX order, the
//! fixed-host baking it shares with the prefix composer, and the size of the
//! real thing.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-AGENT-LANE",
    r = 1
);

use vibe_core::manifest::{HostChoice, TargetOs, WhenCondition};

use super::*;
use crate::system_prompt::host_repo::{index_rows, repo_root};
use crate::system_prompt::{
    ContinuationBand, ContinuationPlan, ContinuationRow, MarkerKind, marker_line,
    parse_marker_line, verify_lane_markers,
};

const NODE: &str = "org.vibevm.core/vibevm";

fn source<'a>(path: &'a str, bytes: &'a str) -> LaneSource<'a> {
    LaneSource {
        path,
        bytes: bytes.as_bytes(),
    }
}

fn text(lane: &ComposedLane) -> String {
    String::from_utf8(lane.bytes.clone()).expect("UTF-8")
}

#[test]
fn the_lane_is_static_first_then_index_order_each_wrapped_in_its_own_pair() {
    let lane = compose_agent_lane(
        NODE,
        &[
            source("vibevm/vibespecs/boot/STATIC.xml", "<spec/>\n"),
            source("vibevm/vibespecs/boot/00-core.xml", "<core/>\n"),
            source("vibevm/vibespecs/boot/90-user.xml", "<user/>\n"),
        ],
    )
    .expect("the lane composes");
    assert_eq!(
        lane.segments
            .iter()
            .map(|s| s.path.as_str())
            .collect::<Vec<_>>(),
        [
            "vibevm/vibespecs/boot/STATIC.xml",
            "vibevm/vibespecs/boot/00-core.xml",
            "vibevm/vibespecs/boot/90-user.xml"
        ],
        "delivered order is INDEX order, STATIC first"
    );
    // The composer's own output passes the offline check, segment for
    // segment and offset for offset.
    let verified = verify_lane_markers(&lane.bytes).expect("verifies");
    assert_eq!(verified.len(), 3);
    for (declared, found) in lane.segments.iter().zip(&verified) {
        assert_eq!(declared.path, found.path);
        assert_eq!(declared.hash12, found.hash12);
        assert_eq!((declared.offset, declared.len), (found.offset, found.len));
    }
    assert_eq!(
        lane.hash_of("vibevm/vibespecs/boot/STATIC.xml"),
        Some(lane.segments[0].hash12.as_str()),
        "the managed block's expected value is this one"
    );
}

/// No template, no entrypoint, no router, and no marker the composer did not
/// write: the lane is the lane.
#[test]
fn the_lane_carries_no_template_no_entrypoint_and_no_router() {
    let lane =
        compose_agent_lane(NODE, &[source("boot/STATIC.xml", "<spec/>\n")]).expect("composes");
    let composed = text(&lane);
    assert!(!composed.contains("<vibevm>"));
    assert!(!composed.contains("What this request already supplies"));
    assert!(!composed.contains("vibe:prompt-segment"));
    let markers: Vec<_> = composed
        .lines()
        .filter_map(parse_marker_line)
        .map(|marker| marker.kind)
        .collect();
    assert_eq!(markers, [MarkerKind::Begin, MarkerKind::End]);
}

#[test]
fn a_body_without_its_own_newline_gets_one_inside_the_pair_and_the_hash_covers_it() {
    let lane = compose_agent_lane(NODE, &[source("boot/a.xml", "no newline")]).expect("composes");
    assert_eq!(
        &lane.bytes[lane.segments[0].offset..][..lane.segments[0].len],
        b"no newline\n"
    );
    assert_eq!(
        lane.segments[0].hash12,
        crate::system_prompt::hash12(b"no newline\n")
    );
    verify_lane_markers(&lane.bytes).expect("recomputable from the delivered text alone");
}

#[test]
fn a_crlf_body_keeps_crlf_while_the_marker_lines_stay_lf() {
    let lane =
        compose_agent_lane(NODE, &[source("boot/a.xml", "one\r\ntwo\r\n")]).expect("composes");
    let composed = text(&lane);
    assert!(composed.starts_with("<!-- vibe:lane-begin"));
    assert_eq!(
        &lane.bytes[lane.segments[0].offset..][..lane.segments[0].len],
        b"one\r\ntwo\r\n"
    );
    assert!(composed.ends_with(" -->\n"));
}

#[test]
fn the_same_inputs_give_the_same_bytes() {
    let once = compose_agent_lane(
        NODE,
        &[source("a.xml", "<a/>\n"), source("b.xml", "<b/>\n")],
    )
    .expect("composes");
    let twice = compose_agent_lane(
        NODE,
        &[source("a.xml", "<a/>\n"), source("b.xml", "<b/>\n")],
    )
    .expect("composes");
    assert_eq!(once.bytes, twice.bytes);
    assert_eq!(once.sha256, twice.sha256);
}

#[test]
fn two_nodes_get_different_markers_for_the_same_path() {
    let mine = compose_agent_lane("org.vibevm.core/vibevm", &[source("a.xml", "<a/>\n")]).unwrap();
    let theirs = compose_agent_lane("org.other/thing", &[source("a.xml", "<a/>\n")]).unwrap();
    assert_ne!(
        mine.bytes, theirs.bytes,
        "the node id keeps the lanes apart"
    );
    assert_eq!(
        mine.segments[0].hash12, theirs.segments[0].hash12,
        "the hash is of the bytes, which are the same"
    );
}

#[test]
fn the_same_path_twice_refuses() {
    let error = compose_agent_lane(
        NODE,
        &[source("a.xml", "<a/>\n"), source("a.xml", "<a/>\n")],
    )
    .expect_err("one INDEX order delivers each file once");
    assert!(error.to_string().contains("delivered twice"));
}

#[test]
fn a_router_inside_a_source_refuses() {
    let error = compose_agent_lane(NODE, &[source("a.md", "x\n<vibevm>\ny\n</vibevm>\n")])
        .expect_err("a managed block inside the lane refuses");
    assert!(error.to_string().contains("no router can enter it"));
}

#[test]
fn a_marker_shaped_line_inside_a_source_refuses() {
    let planted = marker_line(MarkerKind::Begin, NODE, "other.xml", "0123456789ab").unwrap();
    let body = format!("before\n{planted}\n");
    let error =
        compose_agent_lane(NODE, &[source("a.xml", &body)]).expect_err("a planted marker refuses");
    assert!(error.to_string().contains("read as a segment boundary"));
}

#[test]
fn a_path_a_marker_cannot_carry_refuses_before_any_byte_is_produced() {
    let error =
        compose_agent_lane(NODE, &[source("a b.xml", "<a/>\n")]).expect_err("whitespace refuses");
    assert!(error.to_string().contains("segment path"));
    let error = compose_agent_lane("two words", &[source("a.xml", "<a/>\n")])
        .expect_err("a bad node id refuses");
    assert!(error.to_string().contains("node id"));
}

#[test]
fn an_empty_source_set_composes_an_empty_lane() {
    let lane = compose_agent_lane(NODE, &[]).expect("composes");
    assert!(lane.bytes.is_empty());
    assert!(lane.segments.is_empty());
    assert!(lane.inertness.is_inert());
}

#[test]
fn the_census_travels_with_the_lane() {
    let lane = compose_agent_lane(NODE, &[source("a.xml", "@spec://x/y#z\n")]).expect("composes");
    assert_eq!(lane.inertness.len(), 1);
    assert_eq!(lane.inertness.unfenced(), 1);
    assert!(
        !lane.inertness.is_inert(),
        "this is why `import` stays refused"
    );
    assert_eq!(lane.inertness.sites[0].segment, "a.xml");
}

// --- the fixed-host baking the two composers share ------------------------

fn gated(path: &str, os: TargetOs) -> ContinuationRow {
    ContinuationRow {
        path: path.to_string(),
        band: ContinuationBand::Gated(WhenCondition::Os(os)),
    }
}

fn plain(path: &str) -> ContinuationRow {
    ContinuationRow {
        path: path.to_string(),
        band: ContinuationBand::Unconditional,
    }
}

#[test]
fn a_portable_host_leaves_every_os_row_gated() {
    let plan = ContinuationPlan {
        rows: vec![
            plain("boot/00-core.xml"),
            gated("boot/win.xml", TargetOs::Windows),
        ],
    };
    assert_eq!(plan.resolve_host(HostChoice::Portable), plan);
    assert_eq!(plan.gated().count(), 1);
}

#[test]
fn a_fixed_host_bakes_the_matching_row_and_drops_the_rest_leaving_band_g_empty() {
    let plan = ContinuationPlan {
        rows: vec![
            plain("boot/00-core.xml"),
            gated("boot/win.xml", TargetOs::Windows),
            gated("boot/nix.xml", TargetOs::Linux),
        ],
    };
    let resolved = plan.resolve_host(HostChoice::Fixed(TargetOs::Linux));
    assert_eq!(
        resolved
            .rows
            .iter()
            .map(|r| r.path.as_str())
            .collect::<Vec<_>>(),
        ["boot/00-core.xml", "boot/nix.xml"],
        "the matching row is included in place, the other is dropped entirely"
    );
    assert_eq!(
        resolved.gated().count(),
        0,
        "a fixed host leaves band G empty"
    );
    assert_eq!(resolved.unconditional().count(), 2);
    // And the lane composer takes the resolved rows as ordinary sources.
    let lane = compose_agent_lane(
        NODE,
        &[
            source("boot/STATIC.xml", "<spec/>\n"),
            source("boot/00-core.xml", "<core/>\n"),
            source("boot/nix.xml", "<nix/>\n"),
        ],
    )
    .expect("composes");
    assert_eq!(lane.segments.len(), 3);
    assert!(lane.hash_of("boot/win.xml").is_none());
}

// --- the real lane, on this repository ------------------------------------

/// `AGENT-LANE.md` is `STATIC.xml` plus band U plus the marker lines, which
/// `SP-FMT-103` sizes at about 310 KB here. The measurement is printed and
/// the structure is asserted; no byte count is pinned, because every one of
/// these files changes with the resolution.
#[test]
fn the_host_repositorys_lane_verifies_and_is_the_size_the_campaign_states() {
    let root = repo_root();
    let rows = index_rows(&root);
    let lane_path = root.join("vibevm/vibespecs/boot/STATIC.xml");
    if !lane_path.is_file() || rows.is_empty() {
        eprintln!("not a host checkout — skipping the lane measurement");
        return;
    }
    let mut bodies: Vec<(String, Vec<u8>)> = vec![(
        "vibevm/vibespecs/boot/STATIC.xml".to_string(),
        std::fs::read(&lane_path).expect("the lane"),
    )];
    for row in &rows {
        match std::fs::read(root.join(row)) {
            Ok(bytes) => bodies.push((row.clone(), bytes)),
            Err(_) => return,
        }
    }
    let sources: Vec<LaneSource<'_>> = bodies
        .iter()
        .map(|(path, bytes)| LaneSource {
            path: path.as_str(),
            bytes: bytes.as_slice(),
        })
        .collect();
    let lane = compose_agent_lane("org.vibevm.core/vibevm", &sources).expect("the real lane");
    let verified = verify_lane_markers(&lane.bytes).expect("the real lane verifies");
    assert_eq!(verified.len(), sources.len());
    let bodies_len: usize = lane.segments.iter().map(|s| s.len).sum();
    eprintln!(
        "AGENT-LANE.md on this repository: {} segments, {} B of bodies, {} B in all ({} B of \
         markers); @-sites {} ({} unfenced)",
        lane.segments.len(),
        bodies_len,
        lane.bytes.len(),
        lane.bytes.len() - bodies_len,
        lane.inertness.len(),
        lane.inertness.unfenced(),
    );
    assert!(
        lane.bytes.len() > 280_000,
        "about 310 KB on this repository"
    );
}
