//! Wire-parity oracle for the `hello` handshake schema
//! (`schemas/hello/e1/hello.jtd.json` →
//! `vibe_wire::generated::hello::e1::hello`).
//!
//! This format has no hand-written twin and never will: it is born
//! generated (`Handshake` exists only as the schema's output — the
//! writer arrives with the Ф6.1 step and writes THROUGH this type), so
//! "parity" here cannot mean comparing two implementations. It means
//! what the `wire_parity_*` family became once its differentials
//! collapsed (see `wire_parity_by_name.rs`): the census at every depth
//! and the serde layer the codegen stamps — renames and skip guards
//! must not eat or add a key at any depth.
//!
//! The full fixture deliberately carries everything: all four optional
//! keys, `sunset` inside BOTH worlds, two worlds (a one-element array
//! would not exercise the array as an array). The degenerate writer of
//! the first implementation will not write like this — its document is
//! `vibe` plus one world — but an oracle must exercise every optional
//! branch; semantics is the writer's business, not the schema's. The
//! minimal fixture is that degenerate document, and its assertion is
//! the absence side of `x-default: null`: the optional keys are ABSENT
//! from the output, never written as `null`.

use serde_json::json;
use vibe_wire::generated::hello::e1::hello::Handshake;

/// Key counts a fully populated document puts on the wire at each
/// nesting level: 5 on the handshake itself (`vibe`, `worlds`, plus
/// every optional present), 3 on each world (`epoch`, `path`, plus
/// `sunset`). Declared as constants so the fixture's exhaustiveness is
/// asserted at every depth, not assumed — an optional branch the
/// fixture quietly left out would prove nothing about the key the
/// schema forgot.
const HANDSHAKE_KEY_COUNT: usize = 5;
const WORLD_KEY_COUNT: usize = 3;

/// Every key present, at every depth, survives the schema unchanged:
/// the document parses into the generated root and comes back
/// byte-for-byte at the `Value` level. The census runs on BOTH sides —
/// the fixture's own counts prove the fixture exercises every branch,
/// the round-tripped counts prove the schema carries every key the
/// fixture wrote — and a schema edit that thins either level (an
/// optional dropped at the root, a `sunset` dropped from the world)
/// changes its count here before anywhere else. The permissive reader
/// (the registry rules `[format.handshake]` `foreign_parsers = "many"`,
/// so there is no `deny_unknown_fields`) would otherwise swallow the
/// loss silently; this test is where it cannot.
#[test]
fn fully_populated_handshake_round_trips_through_the_generated_type() {
    let j1 = json!({
        "vibe": "hello/1",
        "worlds": [
            {"epoch": 1, "path": ".", "sunset": "2027-01-01T00:00:00Z"},
            {"epoch": 2, "path": "e2", "sunset": "2028-01-01T00:00:00Z"},
        ],
        "min_client": "0.1.0",
        "notice": "epoch 1 sunsets 2027-01-01; move to the e2 world",
        "successor": "https://registry.example/e2/hello.json",
    });

    assert_eq!(
        j1.as_object().map(|object| object.len()),
        Some(HANDSHAKE_KEY_COUNT),
        "the fixture must exercise every top-level key — a sparse fixture proves nothing"
    );
    assert_eq!(
        j1["worlds"].as_array().map(|array| array.len()),
        Some(2),
        "two worlds — a one-element array would not exercise the array as an array"
    );
    assert_eq!(
        j1["worlds"][0].as_object().map(|object| object.len()),
        Some(WORLD_KEY_COUNT),
        "each world must carry all three keys, `sunset` included"
    );
    assert_eq!(
        j1["worlds"][1].as_object().map(|object| object.len()),
        Some(WORLD_KEY_COUNT),
        "the second world must be as fully populated as the first"
    );

    let parsed: Handshake =
        serde_json::from_value(j1.clone()).expect("the generated root parses the document");
    let j2 = serde_json::to_value(&parsed).expect("the generated root serialises");

    assert_eq!(
        j2.as_object().map(|object| object.len()),
        Some(HANDSHAKE_KEY_COUNT),
        "the schema's echo must carry every top-level key — a key the schema \
         lost is the exact drift this family exists to catch"
    );
    assert_eq!(
        j2["worlds"][0].as_object().map(|object| object.len()),
        Some(WORLD_KEY_COUNT),
        "the schema's echo must carry every world key — depth is the specific \
         risk, a `sunset` the schema dropped would surface nowhere else"
    );
    assert_eq!(
        j1, j2,
        "wire drift between the document and the schema's root — a field the \
         schema misses (at any depth) is silently dropped by the permissive \
         reader"
    );
}

/// The degenerate document — `vibe` and one world, nothing else —
/// round-trips with every optional key ABSENT: `x-default: null` means
/// absence ("no sunset", "no floor"), never a written `null`, and the
/// skip guards the codegen stamps are what keep it that way.
#[test]
fn minimal_document_omits_every_optional_key() {
    let j1 = json!({
        "vibe": "hello/1",
        "worlds": [{"epoch": 1, "path": "."}],
    });

    let parsed: Handshake =
        serde_json::from_value(j1.clone()).expect("the generated root parses the minimal document");
    let j2 = serde_json::to_value(&parsed).expect("the generated root serialises");

    let object = j2
        .as_object()
        .expect("the document serialises as an object");
    assert_eq!(
        object.len(),
        2,
        "only `vibe` and `worlds` remain — every optional key must be absent"
    );
    for key in ["min_client", "notice", "successor"] {
        assert!(
            object.get(key).is_none(),
            "the absent optional key `{key}` must stay absent — `x-default: null` \
             is absence, never a written null"
        );
    }
    let world = j2["worlds"][0]
        .as_object()
        .expect("the world serialises as an object");
    assert_eq!(world.len(), 2, "the world keeps exactly `epoch` and `path`");
    assert!(
        world.get("sunset").is_none(),
        "the absent `sunset` must stay absent — absence IS the fact it carries"
    );
}
