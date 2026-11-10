//! The invariants the parity claim rests on: no growth, determinism across
//! checkouts, the privacy sentinels, and the two halves of the band model.
//!
//! Out of line from the differential itself so each file stays inside the
//! 600-line budget. The split is along a real seam: the parent asserts that
//! the prefix *equals* the classic boot, while these assert the properties
//! that make that equality worth having — that it does not drift with a
//! second run, a second directory or an edit the band model says is free.

use super::ENTRYPOINT_PROSE;
use super::fixtures::{self, SENTINEL, Shape, build, write};
use vibe_core::layout;

/// Ten consecutive regenerations leave the prefix's length unchanged. A
/// prefix that grows on a second run is a self-inclusion bug by definition
/// (`#SYS-NO-EQUAL-REWRITE`).
#[test]
fn ten_regenerations_leave_the_prefix_unchanged() {
    let fixture = build(Shape::standard());
    let first: Vec<(String, Vec<u8>)> = [
        layout::SYSTEM_MD,
        layout::SYSTEM_META_JSON,
        layout::AGENT_LANE_MD,
    ]
    .into_iter()
    .map(|name| (name.to_owned(), fixture.artifact(name)))
    .collect();
    let entrypoint = std::fs::read(fixture.ws.path().join("AGENTS.md")).expect("the entrypoint");

    for round in 1..=10 {
        fixture.regenerate();
        for (name, bytes) in &first {
            let now = fixture.artifact(name);
            assert_eq!(
                now.len(),
                bytes.len(),
                "`{name}` changed length on round {round}"
            );
            assert_eq!(&now, bytes, "`{name}` changed bytes on round {round}");
        }
        assert_eq!(
            std::fs::read(fixture.ws.path().join("AGENTS.md")).expect("the entrypoint"),
            entrypoint,
            "the entrypoint changed on round {round}"
        );
    }
}

/// The same fixture at two paths composes equal bytes — the determinism
/// claim, which is what makes a provider's exact-prefix match possible at
/// all (`#SYS-DETERMINISM`).
#[test]
fn the_same_fixture_at_two_paths_composes_equal_bytes() {
    let one = build(Shape::standard());
    let two = build(Shape::standard());
    assert_ne!(one.ws.path(), two.ws.path(), "two different directories");
    for name in [
        layout::SYSTEM_MD,
        layout::SYSTEM_META_JSON,
        layout::AGENT_LANE_MD,
    ] {
        assert_eq!(
            one.artifact(name),
            two.artifact(name),
            "`{name}` differs between two checkout paths"
        );
    }
}

/// Privacy sentinels: a boot file that *mentions* user-local material is an
/// instruction to a tool-using agent, never permission to embed the file.
/// The files exist in the fixture and their contents appear in no output
/// (`#SYS-NO-HOME-EXPANSION`).
#[test]
fn user_local_sentinels_appear_in_no_generated_artifact() {
    let fixture = build(Shape {
        sentinels: true,
        ..Shape::standard()
    });
    for name in [
        layout::SYSTEM_MD,
        layout::SYSTEM_META_JSON,
        layout::AGENT_LANE_MD,
        layout::INDEX_MD,
        layout::STATIC_MD,
    ] {
        let text = String::from_utf8_lossy(&fixture.artifact(name)).into_owned();
        assert!(
            !text.contains(SENTINEL),
            "`{name}` carries a user-local sentinel's contents"
        );
    }
    // The *mention* travels, because the authored text is never rewritten.
    let lane = String::from_utf8_lossy(&fixture.artifact(layout::AGENT_LANE_MD)).into_owned();
    assert!(
        lane.contains("~/.vibe/steward/contexts/*/plan.toml"),
        "the authored sentence itself is delivered verbatim"
    );
    assert!(
        fixture
            .ws
            .path()
            .join("home/.vibe/steward/plan.toml")
            .exists(),
        "the sentinel files genuinely exist, so the assertion is not vacuous"
    );
}

/// The authored entrypoint prose outside the managed block reaches the
/// prefix, which is the half of `#SYS-NO-REWRITE-OF-AUTHORED-TEXT` a reader
/// most wants proved: nothing is summarised away.
#[test]
fn the_authored_entrypoint_prose_reaches_the_prefix_verbatim() {
    let fixture = build(Shape::standard());
    let prefix = String::from_utf8(fixture.artifact(layout::SYSTEM_MD)).expect("UTF-8");
    let authored = ENTRYPOINT_PROSE.trim_end_matches('\n');
    assert!(
        prefix.contains(authored),
        "the authored entrypoint prose is in the prefix verbatim"
    );
    assert_eq!(
        prefix.matches("ENTRYPOINT-CANARY-9f21").count(),
        1,
        "and exactly once — the entrypoint is one segment, never concatenated twice"
    );
}

/// An edit of a band-U file changes no artifact's bytes: the plan's identity
/// is `INDEX.md`'s digest and no row's content is witnessed anywhere
/// (`#SYS-CONTINUATION-WITNESS-IS-RUNTIME`). The lane is recomposed at every
/// launch, so the edit is seen there and only there.
#[test]
fn an_edit_of_a_band_u_file_leaves_the_prefix_byte_identical() {
    let fixture = build(Shape::standard());
    let before = fixture.artifact(layout::SYSTEM_MD);
    let sidecar_before = fixture.artifact(layout::SYSTEM_META_JSON);
    write(
        fixture.ws.path(),
        fixtures::boot_rel("90-user.md"),
        "# own user\n\nOWN-USER-BYTES-EDITED\n",
    );
    fixture.regenerate();
    assert_eq!(
        fixture.artifact(layout::SYSTEM_MD),
        before,
        "a band-U edit moves no byte of the prefix"
    );
    assert_eq!(
        fixture.artifact(layout::SYSTEM_META_JSON),
        sidecar_before,
        "and none of the sidecar, which witnesses no row's content"
    );
    assert!(
        String::from_utf8_lossy(&fixture.artifact(layout::AGENT_LANE_MD))
            .contains("OWN-USER-BYTES-EDITED"),
        "the lane carries the current bytes, which is what band U means"
    );
}
