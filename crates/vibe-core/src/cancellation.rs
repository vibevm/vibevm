//! Cooperative cancellation shared by command and action owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

/// A cancellation request is distinct from acknowledgement by a checkpoint.
/// ```
/// use vibe_core::cancellation::CancellationToken;
/// let token = CancellationToken::new();
/// token.cancel();
/// assert!(token.checkpoint().is_err());
/// assert!(token.acknowledged());
/// ```
#[derive(Clone, Default)]
pub struct CancellationToken {
    state: Arc<State>,
    poll: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    controlled: bool,
}
#[derive(Default)]
struct State {
    requested: AtomicBool,
    acknowledged: AtomicBool,
    publication: AtomicU8,
    unowned_effects: AtomicBool,
}
impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken")
            .field("requested", &self.state.requested.load(Ordering::SeqCst))
            .finish()
    }
}
/// Stop acknowledged at a cooperative boundary, not proof of rollback.
/// ```
/// assert_eq!(vibe_core::cancellation::Cancelled.to_string(), "operation cancelled at a cooperative checkpoint");
/// ```
#[derive(Debug, thiserror::Error)]
#[error("operation cancelled at a cooperative checkpoint")]
pub struct Cancelled;
impl CancellationToken {
    pub fn new() -> Self {
        Self {
            controlled: true,
            ..Self::default()
        }
    }
    /// Install the composition root's portable stop observation.
    pub fn with_poll(mut self, poll: Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
        self.controlled = true;
        self.poll = Some(poll);
        self
    }
    pub fn cancel(&self) {
        self.state.requested.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        if self.poll.as_ref().is_some_and(|poll| poll()) {
            self.cancel();
        }
        self.state.requested.load(Ordering::SeqCst)
    }
    pub fn checkpoint(&self) -> Result<(), Cancelled> {
        if self.is_cancelled() {
            self.state.acknowledged.store(true, Ordering::SeqCst);
            Err(Cancelled)
        } else {
            Ok(())
        }
    }
    pub fn has_control(&self) -> bool {
        self.controlled || self.state.requested.load(Ordering::SeqCst)
    }
    pub fn acknowledged(&self) -> bool {
        self.state.acknowledged.load(Ordering::SeqCst)
    }
    /// Mark the first owned local effect; cancellation does not infer recovery.
    /// Build scripts may write outside the publication receipt. Never label those restored.
    pub fn mark_unowned_effects(&self) {
        self.state.unowned_effects.store(true, Ordering::SeqCst);
    }
    pub fn publishing(&self) {
        self.state.publication.fetch_max(1, Ordering::SeqCst);
    }
    pub fn restored(&self) {
        let _ = self
            .state
            .publication
            .compare_exchange(1, 3, Ordering::SeqCst, Ordering::SeqCst);
    }
    pub fn committed(&self) {
        self.state.publication.fetch_max(2, Ordering::SeqCst);
    }
    /// Conservative classification until the recovery owner reports more.
    pub fn publication_state(&self) -> &'static str {
        let state = self.state.publication.load(Ordering::SeqCst);
        if state != 2 && self.state.unowned_effects.load(Ordering::SeqCst) {
            return "recovery-required";
        }
        match state {
            0 => "not-started",
            1 => "recovery-required",
            2 => "committed",
            _ => "restored",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_restore_does_not_claim_arbitrary_build_writes_were_undone() {
        let token = CancellationToken::new();
        token.publishing();
        token.mark_unowned_effects();
        token.cancel();
        assert!(token.checkpoint().is_err());
        token.restored();
        assert_eq!(token.publication_state(), "recovery-required");
    }
}
