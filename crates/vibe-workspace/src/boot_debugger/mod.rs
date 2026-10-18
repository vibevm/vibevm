//! Captured boot evidence, independent of display and live compilation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#CAPTURED-GRAPH");

use serde::Serialize;
use vibe_spec::{
    BootGraph, GraphContribution, GraphOccurrence, InclusionParent, ReferenceState, SourceIdentity,
};
pub mod dynamic_index;
pub mod hypothetical;
#[cfg(test)]
mod m_06_d_2_cases;
pub mod measurement;
pub mod preparation;
pub mod protocol;
pub mod retained;
mod static_summary;
pub mod viewer;
pub use static_summary::{StaticDecompileError, capture_static, decompile_static};

/// Existing CLI summary vocabulary, now shared without changing its JSON.
#[derive(Debug, Clone, Serialize)]
pub struct StaticContribution {
    pub order: u64,
    pub origin: String,
    pub source_path: String,
    pub bytes: u64,
    pub lines: u64,
    pub embeds: Vec<EmbedSpan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbedSpan {
    pub address: String,
    pub start_line: u64,
    pub end_line: u64,
}

#[derive(Debug, Clone)]
pub struct SummaryCapture {
    /// Opaque caller identity of these exact captured summary inputs.
    pub capture_id: String,
    pub graph: BootGraph,
}

/// Project ordered committed summaries; no filesystem reads or resolution.
pub fn from_static_summary(capture_id: String, rows: &[StaticContribution]) -> SummaryCapture {
    let mut graph = BootGraph::default();
    for row in rows {
        let source = graph.source(
            SourceIdentity::StaticEntry {
                origin: row.origin.clone(),
                path: row.source_path.clone(),
            },
            None,
        );
        let mut occurrences = vec![GraphOccurrence {
            source,
            requested: None,
            state: ReferenceState::Unknown,
            parent: InclusionParent::Root,
            artifact_lines: None,
        }];
        // Parents are strictly containing recorded spans; choose the smallest
        // container, not address equality (the same address may recur).
        for (i, span) in row.embeds.iter().enumerate() {
            let parent = row
                .embeds
                .iter()
                .enumerate()
                .filter(|(j, p)| {
                    *j != i && p.start_line < span.start_line && span.end_line < p.end_line
                })
                .min_by_key(|(_, p)| p.end_line - p.start_line)
                .map(|(j, _)| j + 1)
                .unwrap_or(0);
            let source = graph.source(SourceIdentity::Spec(span.address.clone()), None);
            occurrences.push(GraphOccurrence {
                source,
                requested: Some(span.address.clone()),
                state: ReferenceState::Unknown,
                parent: InclusionParent::Occurrence(parent),
                artifact_lines: Some((span.start_line, span.end_line)),
            });
        }
        graph.contributions.push(GraphContribution {
            origin: row.origin.clone(),
            path: row.source_path.clone(),
            kind: "committed-static".into(),
            seed: Some(source),
            occurrences,
        });
    }
    graph.gaps.push("Committed markers record artifact lines, not authored source positions or checked reference resolution.".into());
    SummaryCapture { capture_id, graph }
}

/// Exact recorded nesting from contribution root to the selected fragment.
/// Invalid/cyclic caller-created graphs are refused rather than truncated.
pub fn why_included(row: &GraphContribution, occurrence: usize) -> Option<Vec<usize>> {
    let mut chain = Vec::new();
    let mut current = Some(occurrence);
    while let Some(i) = current {
        if chain.contains(&i) {
            return None;
        }
        let node = row.occurrences.get(i)?;
        chain.push(i);
        current = match node.parent {
            InclusionParent::Unknown => return None,
            InclusionParent::Root => None,
            InclusionParent::Occurrence(parent) => Some(parent),
        };
    }
    chain.reverse();
    Some(chain)
}

#[cfg(test)]
mod m_06_b_1_cases;

/// Project observer counts without inventing node identities or inclusion
/// paths absent from the emission witness. Each count remains an occurrence.
pub fn from_emission(capture_id: String, event: &vibe_spec::EmissionEvent) -> SummaryCapture {
    let mut graph = BootGraph::default();
    for row in event.contributions() {
        let source = graph.source(
            SourceIdentity::StaticEntry {
                origin: row.origin().into(),
                path: row.path().into(),
            },
            None,
        );
        graph.contributions.push(GraphContribution {
            origin: row.origin().into(),
            path: row.path().into(),
            kind: format!("{:?}", row.kind()),
            seed: Some(source),
            occurrences: (0..row.occurrences())
                .map(|_| GraphOccurrence {
                    source,
                    requested: None,
                    state: ReferenceState::Unknown,
                    parent: InclusionParent::Unknown,
                    artifact_lines: None,
                })
                .collect(),
        });
    }
    graph.gaps.push("Emission counts identify contribution occurrences only; per-node source, position and inclusion chains are unavailable in this observer event.".into());
    SummaryCapture { capture_id, graph }
}

#[cfg(test)]
mod m_06_e_1_cases;

#[cfg(test)]
mod m_06_e_2_cases;
pub mod scenario;

#[cfg(test)]
mod m_10_d_1_cases;
