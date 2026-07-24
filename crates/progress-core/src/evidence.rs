//! The evidence-provider seam (PROP-043 §6).
//!
//! The core knows only this trait; vibevm wires specmap into it from the
//! adapter side. A project without any provider runs with empty evidence —
//! the separability law in action.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#evidence");

use crate::model::{Marker, Stage, State};
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
}
