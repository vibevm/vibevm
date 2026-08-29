//! The witness-side byte accounting the analyzer observer reports (R4.3,
//! packages-2026-09 architecture §9) — the ONE spelling of "how many
//! emitted bytes belong to a contribution" and "how many bytes a lane
//! holds", both derived from evidence the manager already owns.
//!
//! **Witnesses, never artifact text.** Every count below reads a
//! contribution's chunk stream, its prepared XML document, or its target
//! address. None of it reads the emitted tape back, and none of it knows
//! what a generated comment marker looks like: the counts are the sizes
//! of the SAME renderings the backends write (the Markdown flatten, the
//! XML document render, the `#use` reference line), computed from the
//! inputs to those renderings rather than from their output. A fence
//! test pins this: a contribution whose own body CONTAINS text shaped
//! like a marker must still be counted by its chunks, where a
//! marker-parsing implementation would count something else.
//!
//! **Frame by subtraction, not by a second walk.** The frame — prologue,
//! markers, separators, the bytes no contribution owns — is
//! `total − Σ contributions`, with the total the artifact's own length.
//! There is deliberately no independent frame accounting to drift from
//! the backends' framing; the oracle test instead proves the
//! CONTRIBUTION counts exact by reconciling the whole rendering against
//! the real backend bytes for a lane that exercises every contribution
//! kind.
//!
//! **The lane measure.** A lane's byte content for stage deltas is the
//! untrimmed chunk stream — Σ of every contribution's flatten length,
//! backend-neutral, framing-free. Elided and hoisted contributions have
//! no chunks and contribute nothing to it; the `#use` line the backend
//! writes for a hoist is framing around a reference, not lane content.

use super::super::ir::{
    LaneChunk, LaneContribution, LaneIr, PreEmissionWitness, PreparedEmissionTarget,
};
use super::super::observer::{EmissionContribution, EmissionEvent, EmissionKind};
use super::static_md;

#[cfg(test)]
#[path = "attribution_tests.rs"]
mod tests;

/// The attribution evidence of one accepted emission: per-contribution
/// content bytes and occurrence counts, plus the frame as the complement
/// of the contributions inside the artifact's own length.
///
/// Pure over the witness and the emitted length; nothing is read back
/// from the tape.
pub(crate) fn emission_evidence(witness: &PreEmissionWitness, total_bytes: usize) -> EmissionEvent {
    let documents = match &witness.prepared_target {
        PreparedEmissionTarget::Xml { documents } => Some(documents.as_slice()),
        PreparedEmissionTarget::Markdown => None,
        #[cfg(any(test, feature = "test-support"))]
        PreparedEmissionTarget::Custom => None,
    };
    let mut contributions = Vec::with_capacity(witness.contributions.len());
    let mut content_total: usize = 0;
    for (index, contribution) in witness.contributions.iter().enumerate() {
        let document = documents
            .and_then(|docs| docs.get(index))
            .and_then(|prepared| prepared.as_ref());
        let row = contribution_row(contribution, document);
        content_total = content_total.saturating_add(row.bytes());
        contributions.push(row);
    }
    EmissionEvent::new(
        witness.context.clone(),
        contributions,
        total_bytes,
        total_bytes.saturating_sub(content_total),
    )
}

/// One contribution's row: content bytes from the witness, occurrences
/// from the chunk stream (`NormalOpen` brackets), kind from the variant.
fn contribution_row(
    contribution: &LaneContribution,
    document: Option<&vibe_specdoc::doc::SpecDoc>,
) -> EmissionContribution {
    let kind = match contribution {
        LaneContribution::Normal { .. } => EmissionKind::Normal,
        LaneContribution::Simple { .. } => EmissionKind::Simple,
        LaneContribution::Elided { .. } => EmissionKind::Elided,
        LaneContribution::Hoisted { .. } => EmissionKind::Hoisted,
    };
    let meta = match contribution {
        LaneContribution::Normal { meta, .. }
        | LaneContribution::Simple { meta, .. }
        | LaneContribution::Elided { meta }
        | LaneContribution::Hoisted { meta, .. } => meta,
    };
    let (bytes, occurrences) = match contribution {
        LaneContribution::Normal { chunks, .. } => {
            let content = match document {
                // The XML lane renders each document through the SAME
                // projection the backend holds for it.
                Some(prepared) => vibe_specdoc::to_xml(prepared).trim_end().len(),
                // The Markdown lane renders the chunk stream through the
                // one flatten the backend uses.
                None => static_md::flatten_markdown(chunks).trim_end().len(),
            };
            (content, occurrence_count(chunks))
        }
        LaneContribution::Simple { chunks, .. } => {
            // A simple contribution is one occurrence by the link law
            // (`InputOccurrence::Simple`, occurrence 0) and rides the
            // lane as its bare node — no open/close brackets to count.
            let content = match document {
                Some(prepared) => vibe_specdoc::to_xml(prepared).trim_end().len(),
                None => static_md::flatten_markdown(chunks).trim_end().len(),
            };
            (content, 1)
        }
        LaneContribution::Elided { .. } => (0, 0),
        LaneContribution::Hoisted { target, .. } => (hoisted_reference_bytes(target), 0),
    };
    EmissionContribution::new(
        kind,
        meta.origin.clone(),
        meta.path.clone(),
        bytes,
        occurrences,
    )
}

/// How many occurrences a contribution's chunk stream brackets — one
/// `NormalOpen` bracket per occurrence.
fn occurrence_count(chunks: &[LaneChunk]) -> u32 {
    chunks
        .iter()
        .filter(|chunk| matches!(chunk, LaneChunk::NormalOpen { .. }))
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

/// The `#use spec://<group>/<name>` reference line's length — the one
/// line of a hoisted contribution the backend writes on its behalf.
/// `0` for a target the backend would refuse (unreachable on an
/// accepted emission, which is the only place this runs).
fn hoisted_reference_bytes(target: &crate::SpecAddress) -> usize {
    match &target.authority {
        crate::Authority::Package {
            group,
            name,
            version: None,
        } => format!("#use spec://{group}/{name}").len(),
        _ => 0,
    }
}

/// The lane's byte content: every chunk-bearing contribution's stream,
/// untrimmed. The stage-delta measure — see the module doc.
pub(crate) fn lane_content_bytes(lane: &LaneIr) -> usize {
    lane.contributions
        .iter()
        .map(|contribution| match contribution {
            LaneContribution::Normal { chunks, .. } | LaneContribution::Simple { chunks, .. } => {
                static_md::flatten_markdown(chunks).len()
            }
            LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => 0,
        })
        .fold(0usize, usize::saturating_add)
}
