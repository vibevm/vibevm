//! Finite cooperative document editing, separate from read analysis.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-BARRIER-LAW");
use super::*;
mod apply;
mod barrier;
mod journal;
pub use apply::{EditOutcome, EditOutcomeState};
mod plan;
mod proposal;
pub use barrier::EditBarrier;
use plan::Participant;
pub use plan::{DocumentEditAuthority, DocumentReplacement, EditError, EditPlan};
pub use proposal::BufferProposal;

#[derive(Debug, Default)]
pub(super) struct ParticipantRegistry {
    pub(super) clients: std::collections::BTreeMap<u64, std::sync::Weak<super::RegisteredClient>>,
    pub(super) barrier: Option<u64>,
    pub(super) next_barrier: u64,
}

#[cfg(test)]
mod m_16_d_3_cases;
