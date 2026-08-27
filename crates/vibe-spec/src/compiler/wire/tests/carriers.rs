//! Real built-in carriers cross domain→wire→domain identically at every
//! level; the open custom-target identity round-trips; and verifier-valid
//! plugin mutations that violate the corpus producer oracles are ACCEPTED.

use specmark::verifies;
use std::path::PathBuf;

use super::super::{decode, decode_unverified, encode_compact};
use super::fixture::{plan_for, world};
use crate::compiler::builtin::{compile_artifact, compile_artifact_lane, compile_artifact_prefix};
use crate::compiler::ir::{ArtifactPlan, ArtifactTarget, Documents, SourceFormatId, SourceIr};
use crate::compiler::pass::AnyIr;
use crate::{DocTree, SpecAddress};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

fn plan() -> ArtifactPlan {
    plan_for(ArtifactTarget::StaticXml)
}

/// Domain → wire → domain, identical at every level, for carriers the real
/// built-in schedule produced end to end.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn real_builtin_carriers_round_trip_domain_wire_domain_at_every_level() {
    let world = world();
    let plan = plan();

    // Source and Document: the parse pass's own input and output for the seed.
    let source = SourceIr::new(
        crate::compiler::ir::DocumentAddress::Spec(
            SpecAddress::parse("spec://org.demo/alpha/boot/entry#root").unwrap(),
        ),
        SourceFormatId::canonical_markdown(),
        world
            .0
            .get("spec://org.demo/alpha/boot/entry#root")
            .unwrap()
            .clone(),
    );
    let document =
        crate::compiler::ir::DocumentIr::new(source.clone(), DocTree::parse(source.text()));
    let seed_source = source.clone();

    let closure = compile_artifact_prefix(plan.clone(), &world).unwrap();
    let lane = compile_artifact_lane(plan.clone(), &world).unwrap();
    let emitted = compile_artifact(plan, &world).unwrap();

    for carrier in [
        AnyIr::Source(source),
        AnyIr::Document(document),
        AnyIr::Documents(Documents::new(
            closure
                .contributions
                .iter()
                .filter_map(|contribution| match contribution {
                    crate::compiler::ir::ClosureContribution::Simple { document, .. } => {
                        Some(document.tree.clone())
                    }
                    _ => None,
                })
                .map(|tree| crate::compiler::ir::DocumentIr::new(seed_source.clone(), tree))
                .collect::<Vec<_>>(),
        )),
        AnyIr::Closure(closure),
        AnyIr::Lane(lane),
        AnyIr::Emitted(emitted),
    ] {
        let wire = encode_compact(&carrier).unwrap();
        let back = decode(&wire).unwrap();
        assert_equal(&carrier, &back);
    }
}

/// The documents-artifact carrier the real gather produced (every document
/// the worklist discovered, in worklist order).
#[test]
fn the_real_gather_batch_round_trips_as_documents_artifact() {
    let world = world();
    let plan = plan();
    let closure = compile_artifact_prefix(plan.clone(), &world).unwrap();
    // Every spec-addressed graph node's tree beside its source, i.e. a real
    // document batch: re-parse each node's lines.
    let mut documents = Vec::new();
    for node in &closure.nodes {
        let crate::compiler::ir::DocumentAddress::Spec(_) = &node.address else {
            continue;
        };
        let source = SourceIr::new(
            node.address.clone(),
            SourceFormatId::canonical_markdown(),
            node.tree.parts().3.join("\n"),
        );
        documents.push(crate::compiler::ir::DocumentIr::new(
            source,
            DocTree::parse(&node.tree.parts().3.join("\n")),
        ));
    }
    let carrier = AnyIr::Documents(Documents::new(documents));
    let wire = encode_compact(&carrier).unwrap();
    let back = decode(&wire).unwrap();
    assert_equal(&carrier, &back);
}

fn assert_equal(left: &AnyIr, right: &AnyIr) {
    use crate::compiler::ir::{ClosureIr, LaneIr};
    match (left, right) {
        (AnyIr::Source(a), AnyIr::Source(b)) => assert_eq!(a, b),
        (AnyIr::Document(a), AnyIr::Document(b)) => assert_eq!(a, b),
        (AnyIr::Documents(a), AnyIr::Documents(b)) => {
            let a: Vec<_> = a.iter().collect();
            let b: Vec<_> = b.iter().collect();
            assert_eq!(a, b);
        }
        (AnyIr::Closure(a), AnyIr::Closure(b)) => assert_eq!(a as &ClosureIr, b as &ClosureIr),
        (AnyIr::Lane(a), AnyIr::Lane(b)) => assert_eq!(a as &LaneIr, b as &LaneIr),
        (AnyIr::Emitted(a), AnyIr::Emitted(b)) => {
            assert_eq!(a.provenance(), b.provenance(), "the whole provenance");
            assert_eq!(a.bytes(), b.bytes());
        }
        _ => panic!("level mismatch across the round-trip"),
    }
}

/// A valid custom-target closure (domain) round-trips through the wire with
/// its owned backend id verbatim — no refusal, no registry consultation.
#[test]
fn a_valid_custom_target_round_trips_with_its_owned_backend_id() {
    let bytes = std::fs::read(corpus().join("valid").join("closure_artifact_compat.json")).unwrap();
    let first = decode(&bytes).unwrap();
    let wire = encode_compact(&first).unwrap();
    let second = decode(&wire).unwrap();
    let (AnyIr::Closure(a), AnyIr::Closure(b)) = (&first, &second) else {
        panic!("the compat corpus document is a closure");
    };
    assert_eq!(a, b);
    assert_eq!(a.context().target(), b.context().target());
    let round: serde_json::Value = serde_json::from_slice(&wire).unwrap();
    assert_eq!(round["closure"]["context"]["target"], "demo-backend");
}

// ── Verifier-valid plugin mutations the producer oracles would reject ───────

/// CLOSE ORDER binds only what THIS corpus's builtin close emitted. A plugin
/// returning the same graph with a different (still verifier-valid) emission
/// order decodes.
#[test]
fn a_non_builtin_close_order_is_accepted() {
    let mut document = raw("closure_artifact_compat.json");
    let order = document["closure"]["contributions"][0]["emission_order"].clone();
    let mut reversed = order.as_array().unwrap().clone();
    reversed.reverse();
    document["closure"]["contributions"][0]["emission_order"] = serde_json::json!(reversed);
    let bytes = serde_json::to_vec(&document).unwrap();
    decode(&bytes)
        .unwrap_or_else(|error| panic!("a plugin's order is not the decoder's law: {error}"));
}

/// QUALIFY SPELLING binds only visited nodes. An authored slug a plugin kept
/// on a node the builtin qualify would have spelled differently is content,
/// not a decode rule.
#[test]
fn an_authored_slug_on_a_skipped_node_is_accepted() {
    let mut document = raw("closure_artifact_compat.json");
    document["closure"]["nodes"][1]["tree"]["nodes"][1]["id"] =
        serde_json::json!("org-demo--lib--manual-notes-md--api");
    let anchors = &mut document["closure"]["nodes"][1]["tree"]["anchors"];
    *anchors = serde_json::json!({"org-demo--lib--manual-notes-md--api": 1, "DUP": 2, "DUP2": 3});
    let bytes = serde_json::to_vec(&document).unwrap();
    decode(&bytes)
        .unwrap_or_else(|error| panic!("authored anchors are not the decoder's law: {error}"));
}

/// OPAQUE TAPE pins this corpus's golden three bytes. Any tape whose EMIT
/// IDENTITY coherence holds is a valid carrier.
#[test]
fn a_coherent_non_golden_opaque_tape_is_accepted() {
    use sha2::{Digest, Sha256};
    let mut document = raw("emitted_artifact.json");
    let bytes: Vec<u8> = vec![0x01, 0x02, 0x03];
    let framed = {
        let domain = b"vibe-spec/emitted-bytes/v1";
        let mut stream = Vec::new();
        stream.extend_from_slice(&(domain.len() as u64).to_le_bytes());
        stream.extend_from_slice(domain);
        stream.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        stream.extend_from_slice(&bytes);
        stream
    };
    let digest: String = Sha256::digest(&framed)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    document["emitted"]["bytes_b64"] =
        serde_json::json!(super::super::emitted::encode_base64(&bytes));
    document["emitted"]["provenance"]["bytes_digest"] = serde_json::json!(digest);
    let encoded = serde_json::to_vec(&document).unwrap();
    decode(&encoded).unwrap_or_else(|error| panic!("a coherent tape is a valid carrier: {error}"));
}

/// The typed conversion (before the verifier) accepts the corpus's duplicate
/// A coherent duplicate-anchor record is constructible through the typed
/// conversion (the wire carries the record as `DuplicateId` verdict input),
/// and production `decode` still hands it to the FULL verifier, which refuses
/// it — the verdict is the compiler gate's, never silently normalized.
#[test]
fn a_carried_duplicate_anchor_record_constructs_then_the_full_verifier_refuses() {
    // Re-introduce the corpus's retired duplicate: the second DUP fact folds
    // back onto the first, coherently (id, anchors, duplicate record).
    let mut document = raw("closure_artifact_compat.json");
    let tree = &mut document["closure"]["nodes"][1]["tree"];
    tree["nodes"][3]["id"] = serde_json::json!("DUP");
    tree["anchors"] = serde_json::json!({"api": 1, "DUP": 2});
    tree["duplicate_anchors"] = serde_json::json!(["DUP"]);
    tree["lines"][9] = serde_json::json!("##DUP Repeated claim.");
    let bytes = serde_json::to_vec(&document).unwrap();

    let typed = decode_unverified(&bytes).expect("the typed conversion carries the record");
    let AnyIr::Closure(closure) = &typed else {
        panic!("the mutated document is a closure");
    };
    assert!(
        closure
            .nodes
            .iter()
            .any(|node| !node.tree.duplicate_anchors().is_empty()),
        "the record is constructed, not normalized away"
    );

    // The refusal names the verifier's typed FAMILY. It is rendered through
    // the bounded `Debug` sink (repair 4), so it is the variant that is
    // asserted, never a prose sentence a hostile carrier could pad.
    let error = decode(&bytes).unwrap_err();
    let rendered = error.to_string();
    assert!(
        matches!(error, super::super::IrWireError::Verification(_)),
        "production decode hands the carrier to the full verifier: {rendered}"
    );
    assert!(
        rendered.contains("DuplicateId"),
        "production decode returns the full verifier's DuplicateId refusal: {rendered}"
    );
}
