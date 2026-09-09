//! Derived terminal-artifact observations.
//!
//! Authored requirements and live evidence stay separate: this module is a
//! pure resolver over one exact fact address, its whole-unit marker and one
//! provider snapshot. Nothing here mutates source, cache or campaign verdicts.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts");

use crate::doc::{Fact, ParsedDoc};
use crate::evidence::EvidenceProvider;
use crate::model::{ArtifactKind, ArtifactRequirements, Marker, Stage, State};

pub use crate::evidence::{ArtifactObservation, ArtifactState, TerminalCounts, TerminalOutcome};

/// A provider's typed answer before terminal semantics are applied.
///
/// `Known { count: 0, .. }` is observed absence; `Unavailable` means the
/// provider cannot answer. Locator availability is independent of a positive
/// count: a provider can know that evidence exists without exposing where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderArtifactEvidence {
    Unavailable,
    Known {
        count: usize,
        locators: Option<Vec<String>>,
    },
}

/// One complete, ephemeral terminal observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalObservation {
    pub outcome: TerminalOutcome,
    pub artifacts: Vec<ArtifactObservation>,
}

/// Exact local address of an explicitly anchored fact.
pub fn fact_address(doc: &ParsedDoc, fact: &Fact) -> Option<String> {
    fact.id.as_ref().map(|id| format!("{}#{id}", doc.path))
}

/// Resolve the exact whole-unit marker captured for `fact` during parsing.
/// Missing or stale sidecar association remains unknown; it is never repaired
/// by matching a line number.
pub fn fact_marker<'a>(doc: &'a ParsedDoc, fact: &Fact) -> Option<&'a Marker> {
    fact.marker_index.and_then(|index| doc.markers.get(index))
}

/// Derive terminality from authored requirements and current evidence.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#terminal-artifacts"
)]
pub fn resolve_terminal(
    address: &str,
    marker: &Marker,
    requirements: Option<&ArtifactRequirements>,
    provider: &dyn EvidenceProvider,
) -> TerminalObservation {
    let Some(requirements) = requirements else {
        return TerminalObservation {
            outcome: TerminalOutcome::Unclassified,
            artifacts: Vec::new(),
        };
    };

    let artifacts: Vec<ArtifactObservation> = requirements
        .iter()
        .map(|kind| observe_artifact(address, marker, kind, provider))
        .collect();
    let terminal = marker.state == State::Done
        && marker.action.is_none()
        && artifacts
            .iter()
            .all(|artifact| artifact.state == ArtifactState::Satisfied);
    TerminalObservation {
        outcome: if terminal {
            TerminalOutcome::Terminal
        } else {
            TerminalOutcome::Pending
        },
        artifacts,
    }
}

fn observe_artifact(
    address: &str,
    marker: &Marker,
    kind: ArtifactKind,
    provider: &dyn EvidenceProvider,
) -> ArtifactObservation {
    match kind {
        ArtifactKind::Specification
        | ArtifactKind::Decision
        | ArtifactKind::Research
        | ArtifactKind::Plan
        | ArtifactKind::Disposition => self_carried(address, marker.state == State::Done, kind),
        ArtifactKind::Documentation
            if marker.stage == Stage::Doc && marker.state == State::Done =>
        {
            self_carried(address, true, kind)
        }
        ArtifactKind::Implementation | ArtifactKind::Verification | ArtifactKind::Documentation => {
            from_provider(
                kind,
                provider.artifact_evidence_for(address, kind, None),
                None,
            )
        }
        ArtifactKind::External => {
            let reference = marker.r#ref.as_deref();
            from_provider(
                kind,
                provider.artifact_evidence_for(address, kind, reference),
                reference,
            )
        }
    }
}

fn self_carried(address: &str, complete: bool, kind: ArtifactKind) -> ArtifactObservation {
    ArtifactObservation {
        kind,
        state: if complete {
            ArtifactState::Satisfied
        } else {
            ArtifactState::Missing
        },
        count: Some(usize::from(complete)),
        // The fact body is the artifact's locator even while its authored
        // state says that artifact is not complete yet.
        locators: Some(vec![address.to_string()]),
    }
}

fn from_provider(
    kind: ArtifactKind,
    evidence: ProviderArtifactEvidence,
    authored_reference: Option<&str>,
) -> ArtifactObservation {
    match evidence {
        ProviderArtifactEvidence::Unavailable => ArtifactObservation {
            kind,
            state: ArtifactState::Unavailable,
            count: None,
            locators: authored_reference.map(|value| vec![value.to_string()]),
        },
        ProviderArtifactEvidence::Known {
            count,
            mut locators,
        } => {
            if let Some(reference) = authored_reference {
                let values = locators.get_or_insert_with(Vec::new);
                if !values.iter().any(|value| value == reference) {
                    values.insert(0, reference.to_string());
                }
            }
            if let Some(values) = &mut locators {
                values.sort();
                values.dedup();
            }
            ArtifactObservation {
                kind,
                state: if count > 0 {
                    ArtifactState::Satisfied
                } else {
                    ArtifactState::Missing
                },
                count: Some(count),
                locators,
            }
        }
    }
}

/// Count exact fact-marker observations for one valid parsed document.
/// Invalid input is omitted by callers instead of being forced into a fourth
/// terminal state.
pub fn counts_for_doc(doc: &ParsedDoc, provider: &dyn EvidenceProvider) -> TerminalCounts {
    let mut counts = TerminalCounts::default();
    for fact in doc.blocks.iter().flat_map(|block| &block.facts) {
        let Some(address) = fact_address(doc, fact) else {
            continue;
        };
        match &fact.requirements {
            None => counts.observe(TerminalOutcome::Unclassified),
            Some(requirements) => {
                let Some(marker) = fact_marker(doc, fact) else {
                    // An old sidecar can carry no exact marker association.
                    // Decline to invent a classified observation from a line.
                    continue;
                };
                counts.observe(
                    resolve_terminal(&address, marker, Some(requirements), provider).outcome,
                );
            }
        }
    }
    counts
}

#[cfg(test)]
#[path = "terminal/tests.rs"]
mod tests;
