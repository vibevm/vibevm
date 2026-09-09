//! The evidence-provider seam (PROP-043 §6).
//!
//! The core knows only this trait; vibevm wires specmap into it from the
//! adapter side. A project without any provider runs with empty evidence —
//! the separability law in action.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#evidence");

use crate::model::{ArtifactKind, Marker, Stage, State};
use crate::terminal::ProviderArtifactEvidence;
use serde::{Deserialize, Serialize};

/// External facts about one unit, whatever the provider can supply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// e.g. specmap `implements` edge count.
    pub implements: usize,
    /// e.g. specmap `verifies` edge count.
    pub verifies: usize,
    /// Free-form provenance strings ("crates/x/src/y.rs:12").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// The closed per-artifact result exposed by reports and state projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactState {
    Satisfied,
    Missing,
    Unavailable,
}

/// One declared artifact observed against the current provider snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactObservation {
    pub kind: ArtifactKind,
    pub state: ArtifactState,
    /// `None` means the provider could not count; `Some(0)` is a known zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Independent of count: `None` means locators are unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locators: Option<Vec<String>>,
}

/// The closed overall result for one fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerminalOutcome {
    Unclassified,
    Pending,
    Terminal,
}

/// Aggregate terminality, deliberately separate from stage/status rollup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCounts {
    pub classified: usize,
    pub terminal: usize,
    pub pending: usize,
    pub unclassified: usize,
}

impl TerminalCounts {
    pub fn observe(&mut self, outcome: TerminalOutcome) {
        match outcome {
            TerminalOutcome::Unclassified => self.unclassified += 1,
            TerminalOutcome::Pending => {
                self.classified += 1;
                self.pending += 1;
            }
            TerminalOutcome::Terminal => {
                self.classified += 1;
                self.terminal += 1;
            }
        }
    }
}

/// Given a unit address (`spec://…#anchor` or `path#anchor`), return facts.
///
/// Canonical use — the adapter wires a real provider (specmap in vibevm);
/// a bare project runs the null one:
///
/// ```
/// use progress_core::evidence::{Evidence, EvidenceProvider, NoEvidence};
///
/// struct Fixed;
/// impl EvidenceProvider for Fixed {
///     fn evidence_for(&self, _unit: &str) -> Option<Evidence> {
///         Some(Evidence { implements: 1, verifies: 2, refs: vec![] })
///     }
/// }
///
/// let wired: &dyn EvidenceProvider = &Fixed;
/// assert_eq!(wired.evidence_for("spec://x/y#z").expect("facts").verifies, 2);
/// let bare: &dyn EvidenceProvider = &NoEvidence;
/// assert!(bare.evidence_for("spec://x/y#z").is_none());
/// ```
pub trait EvidenceProvider {
    fn evidence_for(&self, unit_addr: &str) -> Option<Evidence>;

    /// Observe one typed terminal artifact without collapsing provider
    /// absence into a known zero. Existing providers need implement only
    /// [`EvidenceProvider::evidence_for`]; this adapter preserves their
    /// legacy behaviour for implementation and verification evidence.
    fn artifact_evidence_for(
        &self,
        unit_addr: &str,
        artifact: ArtifactKind,
        _reference: Option<&str>,
    ) -> ProviderArtifactEvidence {
        let Some(evidence) = self.evidence_for(unit_addr) else {
            return ProviderArtifactEvidence::Unavailable;
        };
        let count = match artifact {
            ArtifactKind::Implementation => evidence.implements,
            ArtifactKind::Verification => evidence.verifies,
            _ => return ProviderArtifactEvidence::Unavailable,
        };
        ProviderArtifactEvidence::Known {
            count,
            // Legacy refs merge implements and verifies locators. Reusing
            // that bag for either typed artifact would forge attribution;
            // typed providers override this method when they can separate it.
            locators: None,
        }
    }
}

/// The null provider: always empty.
pub struct NoEvidence;

impl EvidenceProvider for NoEvidence {
    fn evidence_for(&self, _unit_addr: &str) -> Option<Evidence> {
        None
    }
}

/// A markup-vs-reality mismatch worth flagging (PROP-043 §6):
/// the marker claims more than the evidence shows.
pub fn mismatch(marker: &Marker, ev: &Evidence) -> Option<String> {
    match (marker.stage, marker.state) {
        (Stage::Test, State::Done) if ev.verifies == 0 => {
            Some("marked test/done but no verifying evidence (0 `verifies` edges)".into())
        }
        (Stage::Freeze, _) if ev.implements == 0 => {
            Some("marked freeze but no implementing evidence (0 `implements` edges)".into())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Granularity, MarkerForm};

    fn marker(stage: Stage, state: State) -> Marker {
        Marker {
            stage,
            state,
            action: None,
            actionstage: None,
            audience: Vec::new(),
            comment: None,
            r#ref: None,
            form: MarkerForm::Point,
            granularity: Granularity::Section,
            line: 1,
        }
    }

    #[test]
    fn flags_claims_without_evidence() {
        let ev = Evidence::default();
        assert!(mismatch(&marker(Stage::Test, State::Done), &ev).is_some());
        assert!(mismatch(&marker(Stage::Freeze, State::Done), &ev).is_some());
        assert!(mismatch(&marker(Stage::Impl, State::Work), &ev).is_none());
        let proven = Evidence {
            implements: 2,
            verifies: 3,
            refs: vec![],
        };
        assert!(mismatch(&marker(Stage::Test, State::Done), &proven).is_none());
    }

    #[test]
    fn legacy_adapter_never_attributes_the_merged_ref_bag_to_one_verb() {
        struct Legacy;
        impl EvidenceProvider for Legacy {
            fn evidence_for(&self, _unit_addr: &str) -> Option<Evidence> {
                Some(Evidence {
                    implements: 1,
                    verifies: 1,
                    refs: vec!["implementation.rs:1".into(), "verification.rs:2".into()],
                })
            }
        }
        for kind in [ArtifactKind::Implementation, ArtifactKind::Verification] {
            assert_eq!(
                Legacy.artifact_evidence_for("a.md#A", kind, None),
                ProviderArtifactEvidence::Known {
                    count: 1,
                    locators: None,
                }
            );
        }
    }
}
