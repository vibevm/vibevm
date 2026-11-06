//! Actual built-in typed carriers, then hostile and explicitly mapped changes.

use super::super::{decode, encode_compact};
use super::fixture::{plan_for, world};
use crate::compiler::builtin::{compile_artifact_lane, compile_artifact_prefix};
use crate::compiler::ir::{ArtifactTarget, LaneChunk, LaneContribution, LaneIr, LaneNode};
use crate::compiler::pass::AnyIr;
use crate::compiler::verify::IrVerifier;
use crate::embed::occurrences::EmbedTape;

fn source() -> super::fixture::Map {
    let mut source = world();
    source.0.insert(
        "spec://org.demo/alpha/boot/entry#root".into(),
        "# Alpha {#root}\n#embed spec://org.demo/shared/outer#outer\n".into(),
    );
    source.0.insert(
        "spec://org.demo/shared/outer#outer".into(),
        "# Outer {#outer}\nOUTER\n#embed spec://org.demo/shared/inner#inner\nAFTER\n".into(),
    );
    source.0.insert(
        "spec://org.demo/shared/inner#inner".into(),
        "INNER\n".into(),
    );
    source
}

fn lane() -> LaneIr {
    compile_artifact_lane(plan_for(ArtifactTarget::StaticXml), &source()).unwrap()
}

fn renumber(lane: &mut LaneIr) {
    for (index, contribution) in lane.contributions.iter_mut().enumerate() {
        let chunks = match contribution {
            LaneContribution::Normal { chunks, .. } | LaneContribution::Simple { chunks, .. } => {
                chunks
            }
            _ => continue,
        };
        for chunk in chunks {
            match chunk {
                LaneChunk::NormalOpen { contribution, .. }
                | LaneChunk::NormalClose { contribution, .. }
                | LaneChunk::ForcedNewline { contribution, .. } => *contribution = index,
                LaneChunk::Node(node) => match node.as_mut() {
                    LaneNode::Normal { contribution, .. }
                    | LaneNode::Simple { contribution, .. } => *contribution = index,
                },
            }
        }
    }
}

#[test]
fn annotated_contributions_can_reorder_and_copy_known_provenance() {
    let original = compile_artifact_lane(
        super::fixture::two_root_plan(ArtifactTarget::StaticXml),
        &source(),
    )
    .unwrap();
    let witness = IrVerifier.witness(&AnyIr::Lane(original.clone())).unwrap();
    let mut reordered = original.clone();
    reordered.contributions.reverse();
    renumber(&mut reordered);
    IrVerifier.verify(&AnyIr::Lane(reordered.clone())).unwrap();
    IrVerifier
        .verify_transition(&witness, &AnyIr::Lane(reordered))
        .unwrap();
    let mut duplicated = original;
    duplicated
        .contributions
        .push(duplicated.contributions[0].clone());
    renumber(&mut duplicated);
    IrVerifier.verify(&AnyIr::Lane(duplicated.clone())).unwrap();
    IrVerifier
        .verify_transition(&witness, &AnyIr::Lane(duplicated))
        .unwrap();
}

#[test]
fn equal_owner_keys_can_reorder_and_remove_whole_annotated_carriers() {
    let single = plan_for(ArtifactTarget::StaticXml);
    let plan = crate::compiler::ir::ArtifactPlan::new(
        single.context().clone(),
        vec![
            single.contributions()[0].clone(),
            single.contributions()[0].clone(),
        ],
    )
    .unwrap();
    let original = compile_artifact_lane(plan, &source()).unwrap();
    let witness = IrVerifier.witness(&AnyIr::Lane(original.clone())).unwrap();
    let mut reordered = original;
    reordered.contributions.reverse();
    renumber(&mut reordered);
    IrVerifier.verify(&AnyIr::Lane(reordered.clone())).unwrap();
    IrVerifier
        .verify_transition(&witness, &AnyIr::Lane(reordered.clone()))
        .unwrap();
    reordered.contributions.pop();
    IrVerifier.verify(&AnyIr::Lane(reordered.clone())).unwrap();
    IrVerifier
        .verify_transition(&witness, &AnyIr::Lane(reordered.clone()))
        .unwrap();
    reordered.contributions.clear();
    IrVerifier.verify(&AnyIr::Lane(reordered.clone())).unwrap();
    IrVerifier
        .verify_transition(&witness, &AnyIr::Lane(reordered))
        .unwrap();
}

fn annotated(lane: &mut LaneIr) -> (&mut String, &mut EmbedTape) {
    for contribution in &mut lane.contributions {
        let chunks = match contribution {
            LaneContribution::Normal { chunks, .. } | LaneContribution::Simple { chunks, .. } => {
                chunks
            }
            _ => continue,
        };
        for chunk in chunks {
            if let LaneChunk::Node(node) = chunk {
                let (body, tape) = match node.as_mut() {
                    LaneNode::Normal { body, embeds, .. }
                    | LaneNode::Simple { body, embeds, .. } => (body, embeds),
                };
                if !tape.occurrences.is_empty() {
                    return (body, tape);
                }
            }
        }
    }
    panic!("real compile must produce annotated body")
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-054#IR-EMBED-PROVENANCE-EPOCH"
)]
fn real_recursive_closure_and_lane_roundtrip_with_required_e2_tapes() {
    let closure = compile_artifact_prefix(plan_for(ArtifactTarget::StaticXml), &source()).unwrap();
    for original in [AnyIr::Closure(closure), AnyIr::Lane(lane())] {
        let encoded = encode_compact(&original).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(value["ir_schema"], 2);
        let back = decode(&encoded).unwrap();
        assert_eq!(encode_compact(&back).unwrap(), encoded);
    }
}

#[test]
fn stale_body_and_crossed_ranges_refuse_before_execution() {
    let mut stale = lane();
    annotated(&mut stale).0.push_str("changed");
    assert!(IrVerifier.verify(&AnyIr::Lane(stale.clone())).is_err());
    assert!(decode(&encode_compact(&AnyIr::Lane(stale)).unwrap()).is_err());
    let mut crossed = lane();
    let (body, tape) = annotated(&mut crossed);
    tape.occurrences[1].end = body.lines().count() + 1;
    assert!(decode(&encode_compact(&AnyIr::Lane(crossed)).unwrap()).is_err());
}

#[test]
fn transform_cannot_strip_forge_or_suppress_live_provenance_with_unchanged_body() {
    let original = lane();
    let witness = IrVerifier.witness(&AnyIr::Lane(original.clone())).unwrap();
    let mut stripped = original.clone();
    annotated(&mut stripped).1.occurrences.clear();
    assert!(
        IrVerifier
            .verify_transition(&witness, &AnyIr::Lane(stripped))
            .is_err()
    );
    let mut forged = original.clone();
    annotated(&mut forged).1.occurrences[0].target =
        crate::SpecAddress::parse("spec://org.demo/other/doc#x").unwrap();
    assert!(
        IrVerifier
            .verify_transition(&witness, &AnyIr::Lane(forged))
            .is_err()
    );
    let mut hidden = original;
    for row in &mut annotated(&mut hidden).1.occurrences {
        row.suppressed = true;
        row.end = row.start;
    }
    IrVerifier.verify(&AnyIr::Lane(hidden.clone())).unwrap();
    assert!(
        IrVerifier
            .verify_transition(&witness, &AnyIr::Lane(hidden))
            .is_err()
    );
}

#[test]
fn explicit_mapped_transform_and_payload_removal_keep_the_original_occurrences() {
    let original = lane();
    let witness = IrVerifier.witness(&AnyIr::Lane(original.clone())).unwrap();
    let mut rewritten = original.clone();
    let (body, tape) = annotated(&mut rewritten);
    let next = body.replace("OUTER", "OUTER-EDIT");
    *tape = tape.rewrite_lines(body, &next).unwrap();
    *body = next;
    IrVerifier.verify(&AnyIr::Lane(rewritten.clone())).unwrap();
    IrVerifier
        .verify_transition(&witness, &AnyIr::Lane(rewritten.clone()))
        .unwrap();
    decode(&encode_compact(&AnyIr::Lane(rewritten)).unwrap()).unwrap();
    let mut removed = original;
    let (body, tape) = annotated(&mut removed);
    *tape = tape
        .retain_lines(
            body,
            "",
            &vec![false; body.lines().count()],
            &Default::default(),
        )
        .unwrap();
    body.clear();
    assert_eq!(tape.occurrences.len(), 2);
    assert!(tape.boundaries().is_empty());
    IrVerifier.verify(&AnyIr::Lane(removed.clone())).unwrap();
    IrVerifier
        .verify_transition(&witness, &AnyIr::Lane(removed.clone()))
        .unwrap();
    decode(&encode_compact(&AnyIr::Lane(removed)).unwrap()).unwrap();
}
