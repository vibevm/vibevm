//! The generated-wire preflight: the schema's GLOBAL gate order, enforced as
//! ordered phases over the whole carrier.
//!
//! Every phase runs over the COMPLETE carrier before the next one starts —
//! all scalar identities, then all contexts, then all origin relations, all
//! digest spellings, all addresses and fences, all arena indices, all
//! forests, all spans, all anchor coherence, all set projections, then the
//! set projections. A fault in an earlier phase always beats a fault in a
//! later one, wherever in the carrier each lives and whichever nested decoder
//! would have reached one first. Construction re-uses these same pure
//! validators as belts; it is never the first owner of a lower-numbered
//! fault.
//!
//! Gates 12, 13 and 14 are replays of production laws over the DECODED value,
//! so they are staged in `wire::staged` right after construction; gate 15
//! (EMIT IDENTITY and backend framing) needs the decoded provenance too and
//! rides construction itself. Nothing they could mask comes before them:
//! construction owns no gate grammar of its own.

mod addresses;
mod coherence;
mod digests;
mod inventory;
mod scalars;
mod structure;
mod witness;

use std::collections::BTreeMap;

use super::{IrWireError, wire};

pub(super) use witness::check_set;

/// Run every ordered phase over one generated carrier.
pub(super) fn run(ir: &wire::Ir) -> Result<(), IrWireError> {
    schema(ir)?; // 1
    scalars::run(ir)?; // 2
    coherence::contexts(ir)?; // 3
    coherence::origins(ir)?; // 4
    digests::run(ir)?; // 5
    addresses::run(ir)?; // 6
    structure::bounds(ir)?; // 7
    structure::forests(ir)?; // 8
    structure::spans(ir)?; // 9
    structure::anchors(ir)?; // 10
    witness::sets(ir)?; // 11
    // 12, 13 and 14 are replays over the CONSTRUCTED value; they live in
    // `wire::staged`, which runs immediately after construction and before
    // the immutable verifier.
    Ok(())
}

/// Gate 1: `ir_schema == 1` and the level/cardinality belt of the riding
/// variant, before anything else looks at the value.
fn schema(ir: &wire::Ir) -> Result<(), IrWireError> {
    let (value, level, cardinality) = match ir {
        wire::Ir::SourceDocument(arm) => (
            arm.ir_schema,
            matches!(arm.level, wire::LevelSource::Source),
            matches!(arm.cardinality, wire::CardinalityDocument::Document),
        ),
        wire::Ir::DocumentDocument(arm) => (
            arm.ir_schema,
            matches!(arm.level, wire::LevelDocument::Document),
            matches!(arm.cardinality, wire::CardinalityDocument::Document),
        ),
        wire::Ir::DocumentsArtifact(arm) => (
            arm.ir_schema,
            matches!(arm.level, wire::LevelDocument::Document),
            matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
        ),
        wire::Ir::ClosureArtifact(arm) => (
            arm.ir_schema,
            matches!(arm.level, wire::LevelClosure::Closure),
            matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
        ),
        wire::Ir::LaneArtifact(arm) => (
            arm.ir_schema,
            matches!(arm.level, wire::LevelLane::Lane),
            matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
        ),
        wire::Ir::EmittedArtifact(arm) => (
            arm.ir_schema,
            matches!(arm.level, wire::LevelEmitted::Emitted),
            matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
        ),
    };
    if value != 1 {
        return Err(super::IrWireError::Schema(value));
    }
    debug_assert!(
        level && cardinality,
        "the strict reader guarantees the level/cardinality belt"
    );
    Ok(())
}

// ── arm accessors shared by every phase ─────────────────────────────────────

pub(super) fn documents(ir: &wire::Ir) -> Vec<&'_ wire::DocumentIr> {
    match ir {
        wire::Ir::DocumentDocument(arm) => vec![&arm.doc],
        wire::Ir::DocumentsArtifact(arm) => arm.documents.iter().collect(),
        _ => Vec::new(),
    }
}

pub(super) fn closure(ir: &wire::Ir) -> Option<&'_ wire::ClosureIr> {
    match ir {
        wire::Ir::ClosureArtifact(arm) => Some(&arm.closure),
        _ => None,
    }
}

pub(super) fn lane(ir: &wire::Ir) -> Option<&'_ wire::LaneIr> {
    match ir {
        wire::Ir::LaneArtifact(arm) => Some(&arm.lane),
        _ => None,
    }
}

pub(super) fn emitted(ir: &wire::Ir) -> Option<&'_ wire::EmittedArtifact> {
    match ir {
        wire::Ir::EmittedArtifact(arm) => Some(&arm.emitted),
        _ => None,
    }
}

/// The absorption plan a closure carries, whatever its typestate.
pub(super) fn plan_of(value: &wire::ClosureIr) -> Option<&'_ wire::AbsorptionPlan> {
    match &value.absorption {
        wire::AbsorptionState::Planned(arm) => Some(&arm.plan),
        wire::AbsorptionState::Applied(arm) => Some(&arm.plan),
        wire::AbsorptionState::Unplanned(_) => None,
    }
}

/// Source documents of every document-level carrier (also the pending
/// snapshots' observations).
pub(super) fn source_docs(ir: &wire::Ir) -> Vec<&'_ wire::SourceDoc> {
    if let wire::Ir::SourceDocument(arm) = ir {
        return vec![&arm.doc];
    }
    let mut out: Vec<&wire::SourceDoc> = documents(ir)
        .iter()
        .map(|document| &document.source)
        .collect();
    if let Some(closure) = closure(ir) {
        if let Some(snapshot) = &closure.pending_sources {
            push_resolved(&mut out, &snapshot.documents);
        }
        if let Some(snapshot) = &closure.pending_embeds {
            push_resolved(&mut out, &snapshot.documents);
        }
    }
    out
}

fn push_resolved<'a>(
    out: &mut Vec<&'a wire::SourceDoc>,
    documents: &'a BTreeMap<String, wire::DocumentObservation>,
) {
    for observation in documents.values() {
        if let wire::DocumentObservation::Resolved(resolved) = observation {
            out.push(&resolved.document.source);
        }
    }
}
