//! The marker grammar, and the one part of it that is a mechanism.
//!
//! `#SYS-LANE-MARKERS` gives the markers four uses and says plainly that
//! three of them are instructions to a model. The fourth — the offline
//! check — is the one that can be relied on, so it is the one with tests:
//! pairing, uniqueness, hash agreement, and the refusal of a marker-shaped
//! line inside a segment's own bytes.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-LANE-MARKERS",
    r = 1
);

use super::*;

const NODE: &str = "org.vibevm.core/vibevm";

fn wrap(path: &str, body: &str) -> String {
    let digest = hash12(body.as_bytes());
    format!(
        "{}\n{body}{}\n",
        marker_line(MarkerKind::Begin, NODE, path, &digest).unwrap(),
        marker_line(MarkerKind::End, NODE, path, &digest).unwrap()
    )
}

#[test]
fn a_marker_line_is_a_well_formed_xml_comment_with_three_fields() {
    let line = marker_line(MarkerKind::Begin, NODE, "boot/STATIC.xml", "0123456789ab").unwrap();
    assert_eq!(
        line,
        "<!-- vibe:lane-begin org.vibevm.core/vibevm boot/STATIC.xml 0123456789ab -->"
    );
    assert!(!line.contains('@'), "a marker carries no import character");
    let parsed = parse_marker_line(&line).expect("round-trips");
    assert_eq!(parsed.kind, MarkerKind::Begin);
    assert_eq!(parsed.node_id, NODE);
    assert_eq!(parsed.path, "boot/STATIC.xml");
    assert_eq!(parsed.hash12, "0123456789ab");
}

/// A marker is recognised wherever a harness indented it, and prose that
/// merely mentions the keyword is not a marker.
#[test]
fn recognition_is_on_the_trimmed_line_and_only_on_a_whole_comment() {
    let line = marker_line(MarkerKind::End, NODE, "a/b.xml", "0123456789ab").unwrap();
    assert!(parse_marker_line(&format!("    {line}  \n")).is_some());
    assert!(parse_marker_line("the vibe:lane-begin convention is new").is_none());
    assert!(parse_marker_line("<!-- vibe:c1 something -->").is_none());
    assert!(
        parse_marker_line("<!-- vibe:lane-begin a b c d -->").is_none(),
        "a fourth field is not this grammar"
    );
}

#[test]
fn a_field_that_would_corrupt_the_comment_refuses_and_names_itself() {
    for (field, value) in [
        ("node id", "two words"),
        ("node id", "a-->b"),
        ("node id", "<nope>"),
        ("node id", ""),
    ] {
        let error = marker_line(MarkerKind::Begin, value, "a.xml", "0123456789ab")
            .expect_err("a corrupting field refuses");
        let message = error.to_string();
        assert!(message.contains(field), "{message}");
    }
    let error = marker_line(MarkerKind::Begin, NODE, "a--b.xml", "0123456789ab")
        .expect_err("a path with `--` refuses");
    assert!(error.to_string().contains("segment path"));
}

#[test]
fn every_begin_has_its_end_and_each_hash_matches_its_own_bytes() {
    let lane = format!(
        "{}{}",
        wrap("boot/STATIC.xml", "<spec/>\n"),
        wrap("boot/90-user.xml", "<user/>\n")
    );
    let segments = verify_lane_markers(lane.as_bytes()).expect("the composed lane verifies");
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].path, "boot/STATIC.xml");
    assert_eq!(
        &lane[segments[0].offset..segments[0].offset + segments[0].len],
        "<spec/>\n"
    );
    assert_eq!(segments[1].hash12, hash12(b"<user/>\n"));
}

#[test]
fn a_truncated_lane_is_reported_as_truncation_and_names_the_file() {
    let whole = wrap("boot/STATIC.xml", "<spec/>\n");
    let cut = whole.lines().take(2).collect::<Vec<_>>().join("\n");
    let error = verify_lane_markers(cut.as_bytes()).expect_err("truncation refuses");
    let message = error.to_string();
    assert!(message.contains("never closed"), "{message}");
    assert!(message.contains("boot/STATIC.xml"), "{message}");
    assert!(message.contains("read from disk"), "{message}");
}

#[test]
fn a_duplicated_pair_refuses() {
    let once = wrap("boot/STATIC.xml", "<spec/>\n");
    let error = verify_lane_markers(format!("{once}{once}").as_bytes())
        .expect_err("the same pair twice refuses");
    assert!(error.to_string().contains("delivered twice"));
}

#[test]
fn an_end_without_a_begin_refuses() {
    let line = marker_line(MarkerKind::End, NODE, "a.xml", "0123456789ab").unwrap();
    let error =
        verify_lane_markers(format!("{line}\n").as_bytes()).expect_err("a stray end refuses");
    assert!(error.to_string().contains("has no `vibe:lane-begin`"));
}

#[test]
fn a_hash_that_does_not_match_the_delivered_bytes_refuses() {
    let tampered = wrap("boot/STATIC.xml", "<spec/>\n").replace("<spec/>", "<edited/>");
    let error = verify_lane_markers(tampered.as_bytes()).expect_err("an edited body refuses");
    let message = error.to_string();
    assert!(
        message.contains("but its delivered bytes hash to"),
        "{message}"
    );
}

/// A marker-shaped line inside a body is the quiet defect: the scan would
/// read it as a boundary. It refuses rather than being tolerated.
#[test]
fn a_marker_shaped_line_inside_a_body_refuses() {
    let inner = marker_line(MarkerKind::Begin, NODE, "other.xml", "0123456789ab").unwrap();
    let lane = wrap("boot/STATIC.xml", &format!("before\n{inner}\nafter\n"));
    let error = verify_lane_markers(lane.as_bytes()).expect_err("a nested marker refuses");
    assert!(error.to_string().contains("occurs inside the segment"));
}

#[test]
fn an_empty_lane_verifies_as_no_segments() {
    assert!(verify_lane_markers(b"").expect("empty verifies").is_empty());
}

#[test]
fn hash12_is_the_first_twelve_digits_of_the_full_digest() {
    let bytes = b"the lane";
    assert_eq!(hash12(bytes), sha256_hex(bytes)[..HASH12_LEN]);
    assert_eq!(hash12(bytes).len(), HASH12_LEN);
}
