//! P7 — **differential boot parity**, the campaign's central claim as a
//! mechanical test.
//!
//! The claim: the prefix plus the executed continuation delivers the same
//! instruction text, in the same order, as today's file-reading boot —
//! modulo exactly the managed-block substitution and nothing else
//! (`PROP-076#SYS-BANDS`, `#SYS-BAND-ORDER`, `#SYS-REDIRECT-IN-PLACE`).
//!
//! **The construction rule, which is this test's whole validity.** The
//! reference stream is built from the *published contract* — the entrypoint,
//! then `STATIC.*`, then the `INDEX.md` rows read as a TOML document — by
//! [`classic`], which reads the files directly and shares no helper with the
//! code under test. A reference built from the composer, the continuation
//! plan or `verify_lane_markers` would prove only that one function agrees
//! with itself, and `PLAN.md` P7 names that as a reviewer's first question.
//! The comparison side likewise goes through the published artifacts — the
//! bytes of `SYSTEM.md`, the sidecar read as plain JSON, the bytes of
//! `AGENT-LANE.md` — never through the functions that wrote them.
//!
//! The one declared normalisation is asserted at a **named byte range**: the
//! managed block's own span inside the entrypoint, located by the published
//! marker rule. Everything on either side of it must be byte-identical, so a
//! second normalisation cannot hide anywhere.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-BANDS",
    r = 1
);

use std::collections::BTreeSet;

#[path = "system_prompt_parity/classic.rs"]
mod classic;
#[path = "system_prompt_parity/extension_world.rs"]
mod extension_world;
#[path = "system_prompt_parity/fixtures.rs"]
mod fixtures;
#[path = "system_prompt_parity/invariants.rs"]
mod invariants;

use classic::ClassicBoot;
use fixtures::{ENTRYPOINT_PROSE, Fixture, Shape, TEMPLATE, build};
use vibe_core::layout;

/// The frame line the composer writes before each segment.
fn frame(id: &str) -> String {
    format!("<!-- vibe:prompt-segment {id} -->\n")
}

/// Split the published prefix into its segment bodies **using only the
/// sidecar's own offsets**, and prove in passing that every byte of the file
/// is either a frame line or a segment body — the composition identity, which
/// is what makes "modulo exactly one substitution" a statement about the
/// whole artifact rather than about three slices of it.
fn segment_bodies(fixture: &Fixture) -> Vec<(String, Vec<u8>)> {
    let prefix = fixture.artifact(layout::SYSTEM_MD);
    let sidecar = fixture.sidecar();
    let mut rows: Vec<(String, usize, usize)> = sidecar["segments"]
        .as_array()
        .expect("the sidecar carries a segment map")
        .iter()
        .map(|segment| {
            (
                segment["id"].as_str().expect("a segment id").to_owned(),
                segment["offset"].as_u64().expect("an offset") as usize,
                segment["len"].as_u64().expect("a length") as usize,
            )
        })
        .collect();
    rows.sort_by_key(|(_, offset, _)| *offset);

    let mut cursor = 0usize;
    let mut bodies = Vec::new();
    for (id, offset, len) in rows {
        let gap = String::from_utf8(prefix[cursor..offset].to_vec()).expect("framing is UTF-8");
        let expected = frame(&id);
        assert!(
            gap == expected || gap == format!("\n{expected}"),
            "between two segment bodies the prefix carries only a frame line \
             (and at most the one LF the composer owns), not {gap:?}"
        );
        bodies.push((id, prefix[offset..offset + len].to_vec()));
        cursor = offset + len;
    }
    let tail = String::from_utf8(prefix[cursor..].to_vec()).expect("the tail is UTF-8");
    assert!(
        tail.is_empty() || tail == "\n",
        "the prefix ends with at most the one LF the composer owns, not {tail:?}"
    );
    bodies
}

/// One segment's body, by id.
fn body(bodies: &[(String, Vec<u8>)], id: &str) -> Vec<u8> {
    bodies
        .iter()
        .find(|(seen, _)| seen == id)
        .unwrap_or_else(|| panic!("the prefix carries a `{id}` segment"))
        .1
        .clone()
}

/// The classic boot of a fixture's root node.
fn reference(fixture: &Fixture) -> ClassicBoot {
    ClassicBoot::read(fixture.ws.path(), fixture.ws.path(), "AGENTS.md")
}

// ---------------------------------------------------------------------------
// The central claim
// ---------------------------------------------------------------------------

/// ***The campaign's central claim.*** Named exactly as `PLAN.md` P7's
/// acceptance line spells it, so `-- --exact` reaches it.
#[test]
fn boot_parity_is_exact() {
    let fixture = build(Shape::standard());
    let reference = reference(&fixture);
    let bodies = segment_bodies(&fixture);

    // Band P, segment 1 — the user's own frame, verbatim.
    assert_eq!(
        body(&bodies, "project.template"),
        TEMPLATE.as_bytes(),
        "the template segment is the authored bytes"
    );

    // Band P, segment 2 — the entrypoint, with exactly one substitution at
    // the managed block's own span. Asserted as two byte-identical halves
    // around a named range rather than as a search over the document.
    let span = reference
        .managed_block()
        .expect("the regenerated entrypoint carries a managed block");
    let exported = body(&bodies, "project.entrypoint");
    assert!(
        !span.is_empty(),
        "the fixture's managed block is a real byte range, so the assertion \
         below is not vacuous"
    );
    assert_eq!(
        &exported[..span.start],
        &reference.entrypoint[..span.start],
        "every authored byte before the managed block is verbatim"
    );
    let substituted = exported.len() + span.len() - reference.entrypoint.len();
    assert!(substituted > 0, "something was substituted for the block");
    assert_eq!(
        &exported[span.start + substituted..],
        &reference.entrypoint[span.end..],
        "every authored byte after the managed block is verbatim"
    );

    // And the substituted range is the transport contract: generated prose
    // that carries no router and no import line, which is the half of
    // `#SYS-NO-RECURSION` composition owns.
    let contract = String::from_utf8(exported[span.start..span.start + substituted].to_vec())
        .expect("the contract is UTF-8");
    for forbidden in ["<vibevm>", "</vibevm>", "vibe:lane-begin"] {
        assert!(
            !contract.contains(forbidden),
            "the one normalisation carries no `{forbidden}`:\n{contract}"
        );
    }
    assert!(
        !contract
            .lines()
            .any(|line| line.trim_start().starts_with('@')),
        "the one normalisation carries no import line:\n{contract}"
    );
    assert!(
        contract.contains(&reference.entrypoint_rel),
        "the contract names `{}`, the source it accounts for:\n{contract}",
        reference.entrypoint_rel
    );

    // Band P, segment 3 — the compiled lane, opaque and verbatim.
    let (lane_rel, lane_bytes) = reference
        .lane
        .as_ref()
        .expect("the fixture publishes a static lane");
    assert_eq!(
        &body(&bodies, "project.lane"),
        lane_bytes,
        "the lane segment is the committed `{lane_rel}`, byte for byte"
    );

    // Bands U and G — identity, order and conditions equal to the published
    // INDEX, and not one row's bytes anywhere in the prefix.
    let sidecar = fixture.sidecar();
    let rows: Vec<(String, String, Option<String>)> = sidecar["continuation"]
        .as_array()
        .expect("the sidecar carries the continuation plan")
        .iter()
        .map(|row| {
            (
                row["path"].as_str().expect("a row path").to_owned(),
                row["band"].as_str().expect("a row band").to_owned(),
                row.get("when")
                    .and_then(|when| when.as_str())
                    .map(ToOwned::to_owned),
            )
        })
        .collect();
    let expected: Vec<(String, String, Option<String>)> = reference
        .rows
        .iter()
        .map(|row| {
            (
                row.path.clone(),
                match row.when {
                    None => "unconditional".to_owned(),
                    Some(_) => "gated".to_owned(),
                },
                row.when.clone(),
            )
        })
        .collect();
    assert_eq!(
        rows, expected,
        "the continuation plan is the INDEX rows, in INDEX order, with their own conditions"
    );
    assert!(!rows.is_empty(), "the fixture publishes continuation rows");

    // The plan's identity is the published `INDEX.md`'s own digest and
    // nothing else — computed here from the file's bytes, independently
    // (`#SYS-SIDECAR`, `#SYS-CONTINUATION-WITNESS-IS-RUNTIME`).
    assert_eq!(
        sidecar["index_sha256"].as_str(),
        Some(classic::sha256_of(&reference.index_bytes).as_str()),
        "the sidecar's plan identity is the digest of the INDEX it was built beside"
    );
    let prefix = fixture.artifact(layout::SYSTEM_MD);
    assert!(
        !String::from_utf8_lossy(&prefix).contains("OWN-USER-BYTES"),
        "no INDEX row's bytes enter the prefix, however unconditional the row"
    );
}

/// The reference's own INDEX reading is not vacuous: the fixture really does
/// publish the rows the parity assertion compares against.
#[test]
fn the_reference_index_names_the_authored_rows() {
    let fixture = build(Shape::standard());
    let reference = reference(&fixture);
    let paths: BTreeSet<&str> = reference.rows.iter().map(|row| row.path.as_str()).collect();
    for tail in ["00-core.md", "90-user.md"] {
        let rel = classic::relative(
            fixture.ws.path(),
            &fixture
                .ws
                .path()
                .join(layout::current_boot_dir())
                .join(tail),
        );
        assert!(paths.contains(rel.as_str()), "{paths:?} names `{rel}`");
    }
    assert!(
        !paths
            .iter()
            .any(|path| path.ends_with(layout::SYSTEM_TEMPLATE_MD)),
        "neither the authored template nor a generated output is a boot input: {paths:?}"
    );
    for reserved in [layout::SYSTEM_MD, layout::AGENT_LANE_MD] {
        assert!(
            !paths.iter().any(|path| path.ends_with(reserved)),
            "`{reserved}` is not a boot input: {paths:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Launch delivery: the composed lane against the same reference
// ---------------------------------------------------------------------------

/// What a launch delivery hands a harness — the lane file both the Claude
/// Code and the Codex adapters point at — is `STATIC.*` followed by band U in
/// INDEX order, each file's bytes verbatim between its own markers.
#[test]
fn the_delivered_lane_is_the_classic_lane_then_band_u_in_index_order() {
    let fixture = build(Shape::standard());
    let reference = reference(&fixture);
    let delivered = classic::split_delivered_lane(&fixture.artifact(layout::AGENT_LANE_MD));

    let (lane_rel, lane_bytes) = reference.lane.as_ref().expect("a static lane");
    let mut expected: Vec<(String, Vec<u8>)> = vec![(lane_rel.clone(), lane_bytes.clone())];
    for row in reference.unconditional() {
        expected.push((
            row.path.clone(),
            std::fs::read(fixture.ws.path().join(&row.path)).expect("an INDEX row's file"),
        ));
    }

    assert_eq!(
        delivered
            .iter()
            .map(|segment| segment.path.clone())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>(),
        "the lane delivers the static lane first, then band U in INDEX order"
    );
    for (segment, (path, bytes)) in delivered.iter().zip(&expected) {
        // The delivered body is the file's bytes, plus at most the one LF the
        // composer inserts so the end marker is a line of its own — and that
        // byte lies between the markers, so it is part of what the id covers.
        let mut delivered_body = bytes.clone();
        if !delivered_body.is_empty() && !delivered_body.ends_with(b"\n") {
            delivered_body.push(b'\n');
        }
        assert_eq!(
            segment.body, delivered_body,
            "`{path}` is delivered verbatim"
        );
        assert_eq!(
            segment.hash12,
            classic::hash12_of(&segment.body),
            "`{path}`'s markers carry the id of their own delivered bytes"
        );
        assert_eq!(segment.node_id, "demo", "every pair carries the node id");
    }
    assert!(
        !delivered.is_empty(),
        "a fixture with a lane and band-U rows delivers segments"
    );
}

/// Read delivery: the router block alone must point a reader at the same
/// files, in the same order — and at the delivered lane's own id, so a stale
/// copy is detectable.
#[test]
fn the_router_names_the_files_a_classic_boot_reads() {
    let fixture = build(Shape::standard());
    let reference = reference(&fixture);
    let entrypoint = String::from_utf8(reference.entrypoint.clone()).expect("UTF-8");
    let (lane_rel, _) = reference.lane.as_ref().expect("a static lane");
    let index_rel = classic::relative(
        fixture.ws.path(),
        &fixture.ws.path().join(layout::current_boot_index()),
    );

    assert!(
        entrypoint.contains(&format!(
            "The boot lane is `{lane_rel}`, then every file listed in"
        )),
        "the router names the static lane and the INDEX:\n{entrypoint}"
    );
    assert!(
        entrypoint.contains(&index_rel),
        "the router names `{index_rel}`:\n{entrypoint}"
    );

    // The expected id is the delivered lane's own, which is what makes the
    // staleness rule checkable at all (`#SYS-LANE-MARKERS`).
    let delivered = classic::split_delivered_lane(&fixture.artifact(layout::AGENT_LANE_MD));
    let lane_id = delivered
        .iter()
        .find(|segment| &segment.path == lane_rel)
        .expect("the lane segment")
        .hash12
        .clone();
    assert!(
        entrypoint.contains(&format!("The expected id of `{lane_rel}` is `{lane_id}`")),
        "the router carries the delivered lane's id `{lane_id}`:\n{entrypoint}"
    );
    // No positional word, because in launch delivery the lane arrives first
    // (`#SYS-BOOT-ORDER-IS-PRECEDENCE`).
    let block = &entrypoint[reference.managed_block().expect("a block")];
    for positional in ["above", "below", "you have just read"] {
        assert!(
            !block.to_ascii_lowercase().contains(positional),
            "the router carries no `{positional}`:\n{block}"
        );
    }
}

// ---------------------------------------------------------------------------
// The gated band, the no-lane case, and the both-spellings refusal
// ---------------------------------------------------------------------------

/// An `os:` gate reaches band G and nothing else: its bytes are in no
/// artifact, and the prefix is the same portable bytes whatever OS reads it.
#[test]
fn a_gated_row_stays_gated_and_enters_neither_artifact() {
    let fixture = build(Shape {
        gated_rows: true,
        ..Shape::standard()
    });
    let reference = reference(&fixture);
    let gated: Vec<&classic::IndexRow> = reference
        .rows
        .iter()
        .filter(|row| row.when.is_some())
        .collect();
    assert_eq!(gated.len(), 2, "two gated rows: {:?}", reference.rows);
    assert!(
        gated
            .iter()
            .all(|row| row.when.as_deref().is_some_and(|w| w.starts_with("os:"))),
        "only an `os:` gate can reach the gated band: {gated:?}"
    );

    for name in [layout::SYSTEM_MD, layout::AGENT_LANE_MD] {
        let text = String::from_utf8_lossy(&fixture.artifact(name)).into_owned();
        for bytes in ["WINDOWS-GATED-BYTES", "MACOS-GATED-BYTES"] {
            assert!(
                !text.contains(bytes),
                "`{bytes}` is gated, so it is read per session and never baked into `{name}`"
            );
        }
    }

    // The sidecar records them as gated, with their condition verbatim.
    let sidecar = fixture.sidecar();
    let recorded: Vec<(&str, &str)> = sidecar["continuation"]
        .as_array()
        .expect("a plan")
        .iter()
        .filter(|row| row["band"].as_str() == Some("gated"))
        .map(|row| {
            (
                row["path"].as_str().expect("a path"),
                row["when"].as_str().expect("a condition"),
            )
        })
        .collect();
    assert_eq!(recorded.len(), 2, "{recorded:?}");
}

/// A node with no static contributions builds a valid prefix with no lane
/// segment — a normal case, not an error (`#SYS-LANE-SPELLING`) — and its
/// router says so in the same five numbered rules.
#[test]
fn a_node_with_no_static_lane_still_composes_a_coherent_prefix() {
    let fixture = build(Shape {
        static_lane: false,
        ..Shape::standard()
    });
    let reference = reference(&fixture);
    assert!(reference.lane.is_none(), "no static lane was published");

    let ids: Vec<String> = segment_bodies(&fixture)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        ids,
        vec!["project.template", "project.entrypoint"],
        "no lane segment, and nothing invented to fill its place"
    );
    assert_eq!(fixture.sidecar()["lane"].as_str(), Some("none"));

    let entrypoint = String::from_utf8(reference.entrypoint.clone()).expect("UTF-8");
    assert!(
        entrypoint.contains("This project publishes no static lane"),
        "the router says there is no lane rather than naming an absent file:\n{entrypoint}"
    );
    // Same shape: still five numbered rules, so two projects' blocks compare.
    for rule in ["\n1. ", "\n2. ", "\n3. ", "\n4. ", "\n5. "] {
        assert!(entrypoint.contains(rule), "{entrypoint}");
    }

    // The lane artifact still delivers band U, which is the whole of what a
    // lane-less project has to deliver.
    let delivered = classic::split_delivered_lane(&fixture.artifact(layout::AGENT_LANE_MD));
    assert_eq!(
        delivered
            .iter()
            .map(|segment| segment.path.as_str())
            .collect::<Vec<_>>(),
        reference
            .unconditional()
            .iter()
            .map(|row| row.path.as_str())
            .collect::<Vec<_>>()
    );
}

/// Both `STATIC` spellings on disk is the existing owner's refusal, and the
/// prefix propagates it rather than picking a side.
#[test]
fn both_static_spellings_propagate_the_refusal() {
    let fixture = build(Shape::standard());
    let stray = fixture.boot(layout::STATIC_XML);
    let selected = fixture.boot(layout::STATIC_MD);
    assert!(
        selected.exists(),
        "the Markdown spelling is the selected one"
    );
    std::fs::write(&stray, "<!-- a stray lane -->\n").expect("the stray");

    let workspace =
        vibe_workspace::Workspace::load(fixture.ws.path()).expect("the workspace still loads");
    let error = vibe_workspace::install::apply_resolution_with_spec_format(
        &workspace,
        &fixture.resolution,
        vibe_core::user_config::SlotIntegrity::TrustPresence,
        fixture.format,
        None,
        None,
    )
    .expect_err("two spellings is a refusal");
    let message = error.to_string();
    assert!(
        message.contains("STATIC"),
        "the refusal names the lane: {message}"
    );
}
