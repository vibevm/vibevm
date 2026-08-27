//! The GLOBAL fifteen-gate order across the structural half (7→15).
//!
//! Each case plants two real faults in ONE carrier and asserts the
//! lower-numbered gate wins. Every case is asserted through production
//! `decode` AND through `decode_unverified` — the whole conversion-gate
//! pipeline minus the immutable verifier — so a verdict can never be the
//! verifier's borrowed authority.
//!
//! Gates 1–11 are pure wire phases, and those cases additionally prove the
//! ORDERED PREFLIGHT ALONE owns the verdict. Gates 12, 13 and 14 are replays
//! of production laws over the CONSTRUCTED value; construction is staged
//! between phase 11 and gate 12 on purpose, so `preflight::run` alone is
//! silent for them by design and the honest proof is the pipeline order —
//! which `decode_unverified` is exactly.

use std::path::PathBuf;

use super::super::{decode, decode_unverified, json, preflight};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

fn label_of(error: &str) -> String {
    error
        .split_once("gate `")
        .and_then(|(_, rest)| rest.split_once('`'))
        .map(|(label, _)| label.to_string())
        .unwrap_or_else(|| format!("<no gate in: {error}>"))
}

/// The gate the CONVERSION pipeline owns: production `decode` and the same
/// pipeline without the immutable verifier must name the same one.
fn winner(document: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(document).unwrap();
    let decoded = decode(&bytes).unwrap_err().to_string();
    let converted = decode_unverified(&bytes)
        .expect_err("a conversion gate must own this fault, not the verifier")
        .to_string();
    let (through_decode, through_conversion) = (label_of(&decoded), label_of(&converted));
    assert_eq!(
        through_decode, through_conversion,
        "the verdict must come from a conversion gate ({decoded} / {converted})"
    );
    through_decode
}

fn assert_wins(name: &str, document: &serde_json::Value, expected: &str) {
    assert_eq!(winner(document), expected, "{name}");
}

/// For gates 1–11: the ordered preflight ALONE, with no construction behind
/// it, already owns the verdict.
fn assert_preflight_wins(name: &str, document: &serde_json::Value, expected: &str) {
    assert_wins(name, document, expected);
    let bytes = serde_json::to_vec(document).unwrap();
    let parsed = json::from_strict_slice(&bytes).expect("the fixture is readable JSON");
    let phased = preflight::run(&parsed)
        .expect_err("an ordered phase must own this fault")
        .to_string();
    assert_eq!(
        label_of(&phased),
        expected,
        "{name}: through the phases alone"
    );
}

/// 10 vs 7 ACROSS documents: an anchor-coherence fault in the FIRST document
/// must not beat an arena-bounds fault in the SECOND. Per-document gating
/// would report `anchor-coherence`; the carrier-wide phase reports gate 7.
#[test]
fn a_later_documents_arena_fault_beats_an_earlier_documents_anchor_fault() {
    let mut document = raw("documents_artifact.json");
    document["documents"][0]["tree"]["anchors"]["install"] = serde_json::json!(0);
    document["documents"][1]["tree"]["nodes"][0]["children"] = serde_json::json!([9]);
    assert_preflight_wins("documents batch", &document, "arena-bounds");
}

/// 8 vs 7, same shape: a forest fault in the first document loses to an
/// arena-bounds fault in the second.
#[test]
fn a_later_documents_arena_fault_beats_an_earlier_documents_forest_fault() {
    let mut document = raw("documents_artifact.json");
    document["documents"][0]["tree"]["nodes"][0]["level"] = serde_json::json!(1);
    document["documents"][1]["tree"]["nodes"][0]["children"] = serde_json::json!([9]);
    assert_preflight_wins("documents batch", &document, "arena-bounds");
}

/// 9 vs 8 ACROSS documents: a span fault in the first loses to a forest
/// fault in the second.
#[test]
fn a_later_documents_forest_fault_beats_an_earlier_documents_span_fault() {
    let mut document = raw("documents_artifact.json");
    document["documents"][0]["tree"]["nodes"][1]["span"]["end"] = serde_json::json!(99);
    document["documents"][1]["tree"]["nodes"][0]["level"] = serde_json::json!(1);
    assert_preflight_wins("documents batch", &document, "forest");
}

/// 10 vs 9 ACROSS documents: an anchor fault in the first loses to a span
/// fault in the second.
#[test]
fn a_later_documents_span_fault_beats_an_earlier_documents_anchor_fault() {
    let mut document = raw("documents_artifact.json");
    document["documents"][0]["tree"]["anchors"]["install"] = serde_json::json!(0);
    document["documents"][1]["tree"]["nodes"][1]["span"]["end"] = serde_json::json!(99);
    assert_preflight_wins("documents batch", &document, "span-bounds");
}

/// 11 vs 14: a set-projection fault beats a pass/snapshot fault. The closure
/// decoder used to run 14 first, so this is the exact inversion repair 3
/// named.
#[test]
fn a_set_projection_fault_beats_a_pass_snapshot_fault() {
    let mut document = raw("closure_artifact_compat.json");
    document["closure"]["pending_sources"]["explicit_use_keys"] =
        serde_json::json!(["spec://z.demo/z/z", "spec://a.demo/a/a"]);
    document["closure"]["edges"][0]["kind"] = serde_json::json!("source");
    assert_preflight_wins("closure", &document, "set-projection");
}

/// 12 vs 14: an applied absorption that still carries a pending snapshot is
/// gate 12, and it beats the `source` edge beside that snapshot (gate 14).
/// The closure decoder used to run 14 FIRST, so this is the inversion repair
/// 3 named.
#[test]
fn an_absorption_witness_fault_beats_a_pass_snapshot_fault() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["pending_sources"] = serde_json::json!({
        "discovery_order": [], "documents": {}, "expansions": {}, "explicit_use_keys": []
    });
    document["closure"]["edges"][0]["kind"] = serde_json::json!("source");
    assert_wins("closure", &document, "absorption-witness");
}

/// 13 vs 14: a miscounted link witness list beats a pass/snapshot fault. The
/// compat closure is UNPLANNED, so gate 12 cannot fire and the conflict is
/// exactly 13 against 14.
#[test]
fn a_link_witness_fault_beats_a_pass_snapshot_fault() {
    let mut document = raw("closure_artifact_compat.json");
    // Give the unplanned closure a linked result with the WRONG witness count
    // (it carries two contributions), which is gate 13's own clause.
    document["closure"]["link"] = serde_json::json!({
        "state": "linked",
        "result": {
            "mode": "plain",
            "input_digest": "0000000000000000000000000000000000000000000000000000000000000000",
            "contributions": [],
            "occurrences": []
        }
    });
    document["closure"]["edges"][0]["kind"] = serde_json::json!("source");
    assert_wins("closure", &document, "link-witness-lane");
}

/// 8 vs 14 through a RESOLVED PENDING OBSERVATION's tree: a snapshot's
/// document is a carrier tree too, so its forest is proved before the
/// pass/snapshot witness.
#[test]
fn a_pending_observations_forest_fault_beats_a_pass_snapshot_fault() {
    let mut document = raw("closure_artifact_compat.json");
    document["closure"]["pending_sources"]["documents"]["spec://demo/manual/base.md#base"]["document"]
        ["tree"]["nodes"][0]["level"] = serde_json::json!(1);
    document["closure"]["edges"][0]["kind"] = serde_json::json!("source");
    assert_preflight_wins("closure", &document, "forest");
}

/// 12 vs 13 directly: a miscounted absorption plan beats a miscounted link
/// witness list.
#[test]
fn an_absorption_witness_fault_beats_a_link_witness_fault() {
    let mut document = raw("closure_artifact.json");
    let mut plan = document["closure"]["absorption"]["plan"]["contributions"].take();
    plan.as_array_mut().unwrap().pop();
    document["closure"]["absorption"]["plan"]["contributions"] = plan;
    let mut witnesses = document["closure"]["link"]["result"]["contributions"].take();
    witnesses.as_array_mut().unwrap().pop();
    document["closure"]["link"]["result"]["contributions"] = witnesses;
    assert_wins("closure", &document, "absorption-witness");
}

/// 7 vs 11: an out-of-range edge beats a set-projection fault.
#[test]
fn an_arena_fault_beats_a_set_projection_fault() {
    let mut document = raw("closure_artifact_compat.json");
    document["closure"]["edges"][0]["from"] = serde_json::json!(9);
    document["closure"]["pending_sources"]["explicit_use_keys"] =
        serde_json::json!(["spec://z.demo/z/z", "spec://a.demo/a/a"]);
    assert_preflight_wins("closure", &document, "arena-bounds");
}

/// 10 vs 12: an anchor fault inside a closure NODE beats an absorption
/// witness fault — the tree phases cover graph documents, not just top-level
/// batches.
#[test]
fn a_closure_node_anchor_fault_beats_an_absorption_witness_fault() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["nodes"][2]["tree"]["duplicate_anchors"] = serde_json::json!(["nowhere"]);
    let mut plan = document["closure"]["absorption"]["plan"]["contributions"].take();
    plan.as_array_mut().unwrap().pop();
    document["closure"]["absorption"]["plan"]["contributions"] = plan;
    assert_preflight_wins("closure", &document, "anchor-coherence");
}

/// 8 vs 12 through a `Simple` contribution's EMBEDDED tree: it is a carrier
/// tree like any other, so its forest is proved before any witness gate.
#[test]
fn a_simple_contributions_forest_fault_beats_an_absorption_witness_fault() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][1]["document"]["tree"]["nodes"][0]["level"] =
        serde_json::json!(1);
    let mut plan = document["closure"]["absorption"]["plan"]["contributions"].take();
    plan.as_array_mut().unwrap().pop();
    document["closure"]["absorption"]["plan"]["contributions"] = plan;
    assert_preflight_wins("closure", &document, "forest");
}

/// 6 vs 7: an address drift beats an out-of-range lane node.
#[test]
fn an_address_fault_beats_a_lane_arena_fault() {
    let mut document = raw("lane_artifact.json");
    document["lane"]["contributions"][0]["chunks"][1]["node"]["requested_address"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/x.md");
    document["lane"]["contributions"][0]["chunks"][1]["node"]["node"] = serde_json::json!(9);
    assert_preflight_wins("lane", &document, "address-reparse");
}

/// 2 vs 5 vs 6 on the SAME address: a blank authority piece is an IDENTITY,
/// so it beats a bad digest elsewhere AND the raw-reparse fault it creates.
#[test]
fn a_bad_address_scalar_beats_a_digest_and_a_raw_reparse_fault() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][0]["seed_address"]["authority"]["name"] =
        serde_json::json!("  ");
    document["closure"]["contributions"][0]["seed_address"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/entry#root");
    document["closure"]["link"]["result"]["input_digest"] =
        serde_json::json!("ED73521B98C3CAB322C923AFD66C6A5ECBE81A2A24983071F3CA35DE314EA4F8");
    assert_preflight_wins("closure", &document, "scalar-ids");
}

/// The same at a document identity: a blank static-entry path beats both.
#[test]
fn a_bad_static_entry_scalar_beats_a_digest_and_a_raw_reparse_fault() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][1]["document"]["address"]["path"] = serde_json::json!(" ");
    document["closure"]["contributions"][0]["seed_address"]["raw"] =
        serde_json::json!("spec://org.demo/other/manual/entry#root");
    document["closure"]["link"]["result"]["input_digest"] =
        serde_json::json!("ED73521B98C3CAB322C923AFD66C6A5ECBE81A2A24983071F3CA35DE314EA4F8");
    assert_preflight_wins("closure", &document, "scalar-ids");
}

/// Free-form diagnostic prose is NOT an identity: a blank or MULTILINE
/// expansion `reason` is a legal carrier.
#[test]
fn a_multiline_expansion_failure_reason_is_accepted() {
    let mut document = raw("closure_artifact_compat.json");
    let expansions = document["closure"]["pending_sources"]["expansions"]
        .as_object_mut()
        .unwrap();
    for value in expansions.values_mut() {
        if value["kind"] == serde_json::json!("failed") {
            value["reason"] = serde_json::json!("line one\nline two\n\n  ");
        }
    }
    let bytes = serde_json::to_vec(&document).unwrap();
    decode(&bytes).unwrap_or_else(|error| panic!("prose is not an identity: {error}"));
}
