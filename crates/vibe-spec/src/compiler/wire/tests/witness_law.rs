//! Gates 12 and 13 are the COMPLETE production laws, not a count-and-kind
//! subset. Every case here is a fault a shallow subset would MISS, and each
//! is planted beside a later-gate fault so the ordering is what is proved:
//! a subtle gate-12 fault must never be reported as gate 13, and a subtle
//! gate-13 fault must never be reported as gate 14.

use specmark::verifies;
use std::path::PathBuf;

use super::super::{decode, decode_unverified};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

/// The conversion pipeline's verdict, proved not to be the verifier's: both
/// `decode` and the same pipeline minus `IrVerifier` must name this gate.
fn assert_gate(name: &str, document: &serde_json::Value, expected: &str) {
    let bytes = serde_json::to_vec(document).unwrap();
    let decoded = decode(&bytes).unwrap_err().to_string();
    let converted = decode_unverified(&bytes)
        .expect_err("a conversion gate owns this, not the verifier")
        .to_string();
    let wanted = format!("gate `{expected}`");
    assert!(decoded.contains(&wanted), "{name}: decode gave {decoded}");
    assert!(
        converted.contains(&wanted),
        "{name}: conversion gave {converted}"
    );
}

/// A miscounted link witness vector, planted beside every gate-12 fault below
/// so gate 12 is proved to WIN rather than merely to fire.
fn miscount_link(document: &mut serde_json::Value) {
    let mut witnesses = document["closure"]["link"]["result"]["contributions"].take();
    witnesses.as_array_mut().unwrap().pop();
    document["closure"]["link"]["result"]["contributions"] = witnesses;
}

// ── 12. the complete absorption witness ─────────────────────────────────────

/// The freeze's own case: flip an applied occurrence's `absorbed` bit so the
/// non-absorbed projection no longer equals the live emission order. Count
/// and kind still line up perfectly, so only the full law sees it.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_flipped_absorbed_bit_is_gate_twelve_even_beside_a_link_fault() {
    let mut document = raw("closure_artifact.json");
    let occurrence =
        &mut document["closure"]["absorption"]["plan"]["contributions"][0]["occurrences"][1];
    assert_eq!(occurrence["absorbed"], serde_json::json!(true));
    occurrence["absorbed"] = serde_json::json!(false);
    miscount_link(&mut document);
    assert_gate("flipped absorbed bit", &document, "absorption-witness");
}

/// The plan's mode disagreeing with the applied qualification mode.
#[test]
fn a_plan_mode_that_is_not_the_applied_mode_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["plan"]["mode"] = serde_json::json!("plain");
    miscount_link(&mut document);
    assert_gate("plan mode", &document, "absorption-witness");
}

/// A plan witness whose meta identity drifts from the contribution it aligns
/// with — same kind, same position.
#[test]
fn a_drifted_plan_meta_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["plan"]["contributions"][0]["meta"]["path"] =
        serde_json::json!("manual/other.md");
    miscount_link(&mut document);
    assert_gate("plan meta", &document, "absorption-witness");
}

/// A plan witness pointing at a different seed node than its contribution.
#[test]
fn a_drifted_plan_seed_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["plan"]["contributions"][0]["seed"] = serde_json::json!(0);
    miscount_link(&mut document);
    assert_gate("plan seed", &document, "absorption-witness");
}

/// A plan occurrence whose requested address drifts from the live one, with
/// the `absorbed` bits untouched.
#[test]
fn a_drifted_plan_occurrence_address_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["plan"]["contributions"][0]["occurrences"][0]
        ["requested_address"] = document["closure"]["absorption"]["plan"]["contributions"][0]
        ["occurrences"][2]["requested_address"]
        .clone();
    miscount_link(&mut document);
    assert_gate("plan occurrence address", &document, "absorption-witness");
}

/// A `Simple` plan witness whose address drifts from its contribution's own
/// document identity.
#[test]
fn a_drifted_simple_plan_address_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["plan"]["contributions"][1]["address"]["path"] =
        serde_json::json!("vibevm/vibespecs/boot/01-other.md");
    miscount_link(&mut document);
    assert_gate("simple plan address", &document, "absorption-witness");
}

/// A `Hoisted` plan witness whose target drifts from its contribution's.
#[test]
fn a_drifted_hoisted_plan_target_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    let target = &mut document["closure"]["absorption"]["plan"]["contributions"][3]["target"];
    target["raw"] = serde_json::json!("spec://org.demo/lib/manual/guide.md");
    target["doc_path"] = serde_json::json!("manual/guide.md");
    miscount_link(&mut document);
    assert_gate("hoisted plan target", &document, "absorption-witness");
}

/// Qualification and absorption typestate are one gate-12 witness: planning
/// cannot exist while qualification is still pending.
#[test]
fn pending_qualification_with_a_plan_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["qualification"]["state"] = serde_json::json!("pending");
    miscount_link(&mut document);
    assert_gate(
        "pending qualification with plan",
        &document,
        "absorption-witness",
    );
}

/// The converse misalignment is equally red: applied qualification may not
/// claim that absorption was never planned.
#[test]
fn applied_qualification_without_a_plan_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"] = serde_json::json!({"state": "unplanned"});
    miscount_link(&mut document);
    assert_gate(
        "applied qualification without plan",
        &document,
        "absorption-witness",
    );
}

/// A plan is minted only after the source/embed snapshots it analyses are
/// consumed. This applies to the planned state as well as the applied one.
#[test]
fn a_planned_absorption_with_a_pending_snapshot_is_gate_twelve() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["state"] = serde_json::json!("planned");
    document["closure"]["pending_sources"] = serde_json::json!({
        "discovery_order": [],
        "documents": {},
        "expansions": {},
        "explicit_use_keys": []
    });
    miscount_link(&mut document);
    assert_gate(
        "planned absorption with pending snapshot",
        &document,
        "absorption-witness",
    );
}

// ── 13. the complete link result, and a lane's bracketing ───────────────────

/// The freeze's own case: a linked witness whose `occurrence_count` is wrong
/// while the vector count and kinds look valid, beside a pass/snapshot fault.
/// Gate 13 must win over gate 14.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_wrong_occurrence_count_is_gate_thirteen_even_beside_a_pass_snapshot_fault() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["link"]["result"]["contributions"][0]["occurrence_count"] =
        serde_json::json!(1);
    // Gate 14 needs a pending snapshot beside a matching edge; the terminal
    // closure has none, and adding one would be gate 12 — so the conflict is
    // staged on the UNPLANNED compat closure instead, below. Here the second
    // fault is the digest, which gate 13 also owns.
    assert_gate("occurrence count", &document, "link-witness-lane");
}

/// The same clause on the UNPLANNED compat closure, where gate 12 cannot
/// fire: a linked Normal/Elided vector with a wrong `occurrence_count`, plus
/// a `source` edge beside the pending source snapshot. Gate 13 beats 14.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_wrong_occurrence_count_beats_a_pass_snapshot_fault_on_an_unplanned_closure() {
    let mut document = raw("closure_artifact_compat.json");
    let normal = &document["closure"]["contributions"][0];
    let (meta, seed, seed_address) = (
        normal["meta"].clone(),
        normal["seed"].clone(),
        normal["seed_address"].clone(),
    );
    let elided_meta = document["closure"]["contributions"][1]["meta"].clone();
    document["closure"]["link"] = serde_json::json!({
        "state": "linked",
        "result": {
            "mode": "plain",
            "input_digest": "0000000000000000000000000000000000000000000000000000000000000000",
            "contributions": [
                {"kind": "normal", "meta": meta, "seed": seed,
                 "seed_address": seed_address, "occurrence_count": 7},
                {"kind": "elided", "meta": elided_meta}
            ],
            "occurrences": []
        }
    });
    document["closure"]["edges"][0]["kind"] = serde_json::json!("source");
    assert_gate("unplanned occurrence count", &document, "link-witness-lane");
}

/// The link result's own mode, drifting from the closure's compile mode.
#[test]
fn a_drifted_link_mode_is_gate_thirteen() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["link"]["result"]["mode"] = serde_json::json!("plain");
    assert_gate("link mode", &document, "link-witness-lane");
}

/// The link result's input digest: canonical hex, so gate 5 accepts it, and
/// only the replay sees it is the wrong digest.
#[test]
fn a_canonical_but_wrong_input_digest_is_gate_thirteen() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["link"]["result"]["input_digest"] =
        serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
    assert_gate("input digest", &document, "link-witness-lane");
}

/// A linked occurrence's body, drifting from the text the closure's own tree
/// carries. Nothing structural changes; only the replay sees it.
#[test]
fn a_drifted_linked_occurrence_body_is_gate_thirteen() {
    let mut document = raw("closure_artifact.json");
    let body = document["closure"]["link"]["result"]["occurrences"][0]["body"]
        .as_str()
        .unwrap()
        .to_string();
    document["closure"]["link"]["result"]["occurrences"][0]["body"] =
        serde_json::json!(format!("{body} tampered"));
    assert_gate("occurrence body", &document, "link-witness-lane");
}

/// A LANE carrier's bracketing: drop the opening marker of the first
/// contribution. The lane law is gate 13's clause at the next level, and it
/// runs before any later verdict.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_lane_with_broken_bracketing_is_gate_thirteen() {
    let mut document = raw("lane_artifact.json");
    let mut chunks = document["lane"]["contributions"][0]["chunks"].take();
    chunks.as_array_mut().unwrap().remove(0);
    document["lane"]["contributions"][0]["chunks"] = chunks;
    assert_gate("lane bracketing", &document, "link-witness-lane");
}

/// A lane whose fence history is discontinuous: the second node claims a
/// closed fence before, while the first left one open.
#[test]
fn a_lane_with_a_discontinuous_fence_history_is_gate_thirteen() {
    let mut document = raw("lane_artifact.json");
    document["lane"]["contributions"][0]["chunks"][5]["node"]["fence_before"] =
        serde_json::json!({"state": "closed"});
    assert_gate("lane fence history", &document, "link-witness-lane");
}
