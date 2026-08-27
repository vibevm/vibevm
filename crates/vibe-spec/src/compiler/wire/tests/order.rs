//! The global gate order: an earlier-phase fault always beats a later one,
//! whichever nested decoder would have run first. Each test plants two
//! faults and asserts the earlier phase's registry label wins.

use std::path::PathBuf;

use super::super::decode;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

/// One planted fault, applied to a parsed corpus document.
type Drift = fn(&mut serde_json::Value);

fn gate_of(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap();
    decode(&bytes).unwrap_err().to_string()
}

/// A blank node id (gate 2) beats an out-of-range child index (gate 7).
#[test]
fn a_bad_scalar_beats_a_forest_fault() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][1]["id"] = serde_json::json!(" ");
    document["doc"]["tree"]["nodes"][1]["children"] = serde_json::json!([2, 9]);
    let error = gate_of(&document);
    assert!(
        error.contains("gate `scalar-ids`"),
        "the scalar phase owns the first failure, got {error}"
    );
}

/// A tuple that is no context row (gate 3) beats both an out-of-range edge
/// (gate 7) and a drifted address raw (gate 6).
#[test]
fn a_bad_context_beats_address_and_bounds_faults() {
    for name in ["closure_artifact.json", "closure_artifact_compat.json"] {
        let mut document = raw(name);
        document["closure"]["context"]["artifact"] = serde_json::json!("wrong-id");
        document["closure"]["edges"][0]["from"] = serde_json::json!(9);
        if name == "closure_artifact.json" {
            document["closure"]["edges"][0]["requested_target"]["raw"] =
                serde_json::json!("spec://org.demo/other/manual/x.md");
        }
        let error = gate_of(&document);
        assert!(
            error.contains("gate `context-tuple`"),
            "{name}: the context phase owns the first failure, got {error}"
        );
    }
}

/// A bad canonical spelling (gate 5) beats an address drift (gate 6) and an
/// out-of-range index (gate 7).
#[test]
fn a_bad_digest_beats_an_address_or_bounds_fault() {
    let mut document = raw("lane_artifact.json");
    document["lane"]["source_link_digest"] =
        serde_json::json!("ED73521B98C3CAB322C923AFD66C6A5ECBE81A2A24983071F3CA35DE314EA4F8");
    document["lane"]["contributions"][0]["chunks"][1]["node"]["requested_address"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/x.md");
    document["lane"]["contributions"][0]["chunks"][1]["node"]["node"] = serde_json::json!(9);
    let error = gate_of(&document);
    assert!(
        error.contains("gate `digest-base64-canonical`"),
        "the digest phase owns the first failure, got {error}"
    );

    let mut document = raw("emitted_artifact.json");
    document["emitted"]["bytes_b64"] = serde_json::json!("AP9=");
    document["emitted"]["provenance"]["contributions"][0]["seed"] = serde_json::json!(9);
    let error = gate_of(&document);
    assert!(
        error.contains("gate `digest-base64-canonical`"),
        "the base64 canonicality check owns the first failure, got {error}"
    );
}

/// An origin that contradicts its target (gate 4) beats a drifted raw
/// address (gate 6).
#[test]
fn a_bad_origin_relation_beats_an_address_fault() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][0]["meta"]["origin"] = serde_json::json!("org.demo/other");
    document["closure"]["edges"][0]["requested_target"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/x.md");
    let error = gate_of(&document);
    assert!(
        error.contains("gate `origin-package-relation`"),
        "the origin phase owns the first failure, got {error}"
    );
}

/// Gate 4 must not raw-reparse: on the SAME normal/hoisted target, the
/// origin relation is judged from carried fields, and a drifted `raw` stays
/// gate 6 (repair 2, finding 3).
#[test]
fn the_origin_relation_does_not_consume_the_raw_reparse() {
    // bad origin relation + raw drift on the same target -> gate 4.
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][0]["meta"]["origin"] = serde_json::json!("org.demo/other");
    document["closure"]["contributions"][0]["seed_address"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/entry#root");
    let error = gate_of(&document);
    assert!(
        error.contains("gate `origin-package-relation`"),
        "gate 4 wins: {error}"
    );

    // valid relation + a bad digest elsewhere + raw drift -> gate 5.
    let mut document = raw("closure_artifact.json");
    document["closure"]["link"]["result"]["input_digest"] =
        serde_json::json!("ED73521B98C3CAB322C923AFD66C6A5ECBE81A2A24983071F3CA35DE314EA4F8");
    document["closure"]["contributions"][0]["seed_address"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/entry#root");
    let error = gate_of(&document);
    assert!(
        error.contains("gate `digest-base64-canonical`"),
        "gate 5 beats gate 6 even with raw drift present: {error}"
    );

    // raw drift alone -> gate 6.
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][0]["seed_address"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/entry#root");
    let error = gate_of(&document);
    assert!(
        error.contains("gate `address-reparse`"),
        "raw drift alone is gate 6: {error}"
    );
}

/// The verdict of the ORDERED PHASES ALONE, with no construction behind them.
/// A carrier that only `decode_carrier` would refuse is red here — which is
/// exactly the difference between "the phase walks it" and "something later
/// happens to notice".
fn preflight_gate_of(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap();
    let parsed = super::super::json::from_strict_slice(&bytes).unwrap();
    super::super::preflight::run(&parsed)
        .expect_err("the ordered phases must own this fault")
        .to_string()
}

/// The ordered preflight — not a later construction — owns the address law.
/// Running the phases alone on the parsed carrier is red, so the top-level
/// `source-document` arm cannot rely on `decode_carrier` catching its drift.
#[test]
fn the_preflight_itself_owns_the_top_level_source_document_address() {
    let mut document = raw("source_document.json");
    document["doc"]["address"]["address"]["raw"] =
        serde_json::json!("spec://org.demo/lib/manual/other.md#root");
    let error = preflight_gate_of(&document);
    assert!(
        error.contains("gate `address-reparse`"),
        "the preflight owns it, got {error}"
    );
}

/// A `SourceDoc` address drift (gate 6) beats a forest fault (gate 8) in the
/// document that carries it.
#[test]
fn a_source_doc_address_drift_beats_a_forest_fault() {
    let mut document = raw("document_document.json");
    document["doc"]["source"]["address"]["address"]["raw"] =
        serde_json::json!("spec://org.demo/lib/manual/other.md#root");
    document["doc"]["tree"]["nodes"][0]["parent"] = serde_json::json!(0);
    let error = gate_of(&document);
    assert!(
        error.contains("gate `address-reparse`"),
        "the address phase beats the forest gate, got {error}"
    );
}

/// A `Simple` contribution's whole lowered document rides OUTSIDE the graph.
/// Each of its three address families beats a forest fault planted in that
/// same embedded tree (gate 6 before gate 8) — and the ORDERED PHASES own
/// each one, so the guarantee is mechanical rather than "construction
/// happened to look at aliases before trees".
#[test]
fn a_simple_contributions_embedded_addresses_beat_its_own_forest_fault() {
    let drifts: [(&str, Drift); 3] = [
        ("identity", |document| {
            let mut address = document["closure"]["nodes"][0]["address"].clone();
            address["address"]["raw"] = serde_json::json!("spec://x/y/z#q");
            document["closure"]["contributions"][1]["document"]["address"] = address;
        }),
        ("directive", |document| {
            let mut directive =
                document["closure"]["nodes"][2]["tree"]["directives"]["directives"][0].clone();
            directive["line"] = serde_json::json!(0);
            directive["address"]["raw"] = serde_json::json!("spec://x/y/z#q");
            document["closure"]["contributions"][1]["document"]["tree"]["directives"]["directives"] =
                serde_json::json!([directive]);
        }),
        ("alias", |document| {
            let mut alias =
                document["closure"]["nodes"][2]["tree"]["directives"]["aliases"]["Part"].clone();
            alias["raw"] = serde_json::json!("spec://x/y/z#q");
            document["closure"]["contributions"][1]["document"]["aliases"]["Part"] = alias;
        }),
    ];
    for (family, drift) in drifts {
        let mut document = raw("closure_artifact.json");
        drift(&mut document);
        document["closure"]["contributions"][1]["document"]["tree"]["nodes"][0]["parent"] =
            serde_json::json!(0);
        for (path, error) in [
            ("decode", gate_of(&document)),
            ("preflight", preflight_gate_of(&document)),
        ] {
            assert!(
                error.contains("gate `address-reparse`"),
                "{family} via {path}: the address phase beats the embedded forest gate, got {error}"
            );
        }
    }
}
