//! Cooperative hints expire freshness; they never run domain effects.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-VIEW-INVALIDATION");
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeHint {
    Disk(ResourceId),
    Lock(ResourceId),
    Policy,
    Target,
    Overlay(ResourceId),
    Gap,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalidation {
    pub sequence: u64,
    pub affected: Vec<ResourceId>,
    pub resnapshot_required: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewFreshness {
    UninvalidatedHint,
    Historical,
    ResnapshotRequired,
}
impl DocumentClient {
    /// The host reports sequence numbers in this client's watch context.
    /// No watcher or provider is created; loss remains sticky until resnapshot.
    pub fn notify(
        &self,
        sequence: u64,
        hint: ChangeHint,
    ) -> Result<Invalidation, DocumentAnalysisError> {
        if let ChangeHint::Disk(id) | ChangeHint::Lock(id) | ChangeHint::Overlay(id) = hint {
            if !self.host.declarations.contains_key(&id) {
                return Err(DocumentAnalysisError::UnknownResource);
            }
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        if sequence == 0 || sequence <= state.watch_sequence {
            return Err(DocumentAnalysisError::StaleVersion);
        }
        let expected = state
            .watch_sequence
            .checked_add(1)
            .ok_or(DocumentAnalysisError::Unavailable)?;
        if sequence != expected || hint == ChangeHint::Gap {
            state.watch_gap = true;
        }
        state.watch_sequence = sequence;
        state.advance()?;
        // A source can participate in embeds/configuration; conservatively
        // invalidate the admitted set, rather than claim uncomputed edges.
        Ok(Invalidation {
            sequence,
            affected: self.host.declarations.keys().copied().collect(),
            resnapshot_required: state.watch_gap,
        })
    }
    pub fn freshness(
        &self,
        capture: &AnalysisCapture,
    ) -> Result<ViewFreshness, DocumentAnalysisError> {
        if capture.context != self.host.host || capture.client != self.client {
            return Err(DocumentAnalysisError::InvalidDeclaration);
        }
        let state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        Ok(if state.watch_gap {
            ViewFreshness::ResnapshotRequired
        } else if state.generation != capture.generation {
            ViewFreshness::Historical
        } else {
            ViewFreshness::UninvalidatedHint
        })
    }
    /// Explicit fresh bounded disk capture. Does not save buffers or install.
    pub fn resnapshot(&self) -> Result<AnalysisCapture, DocumentAnalysisError> {
        let mut capture = self.freeze()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DocumentAnalysisError::Unavailable)?;
        if state.generation != capture.generation {
            return Err(DocumentAnalysisError::StaleVersion);
        };
        state.advance()?;
        capture.generation = state.generation;
        state.watch_gap = false;
        Ok(capture)
    }
}
