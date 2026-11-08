//! Cooperative cancellation shared by command and action owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
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
    resources: Arc<Resources>,
}
#[derive(Default)]
struct State {
    requested: AtomicBool,
    acknowledged: AtomicBool,
    publication: AtomicU8,
    unowned_effects: AtomicBool,
    llm_in_flight: AtomicUsize,
    llm_unknown: AtomicBool,
    llm_completed: AtomicBool,
    /// Completed local effects the operation owns until it commits, oldest
    /// first — see [`CancellationToken::adopt_completed_effect`].
    completed: std::sync::Mutex<Vec<Box<dyn CompletedEffect>>>,
}

/// One completed local effect of a step that finished inside an operation —
/// for example the declared outputs an agent step published — which the
/// operation's pre-commit rollback must undo when a LATER stop rolls the
/// operation back.
///
/// The undo is the effect owner's checked inverse: it compares each name
/// against the receipt of what the step wrote before touching it, puts back
/// only this step's own objects, and leaves foreign state untouched. `Err`
/// names what was preserved instead, and the operation then reports
/// `recovery-required` (spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW).
/// ```
/// use vibe_core::cancellation::{CancellationToken, CompletedEffect};
/// /// A published file a stop must take back, unless someone replaced it.
/// struct Published { replaced: bool }
/// impl CompletedEffect for Published {
///     fn undo(self: Box<Self>) -> Result<(), String> {
///         if self.replaced {
///             Err("docs/out.md: changed replacement preserved".into())
///         } else {
///             Ok(())
///         }
///     }
/// }
/// let token = CancellationToken::new();
/// token.publishing();
/// token.adopt_completed_effect(Box::new(Published { replaced: true }));
/// token.cancel();
/// assert!(token.checkpoint().is_err());
/// let reason = token.undo_completed_effects().unwrap_err();
/// assert!(reason.contains("docs/out.md"));
/// token.restored();
/// assert_eq!(token.publication_state(), "recovery-required");
/// ```
pub trait CompletedEffect: Send {
    /// Undo the effect. `Err` carries the actionable account of every name
    /// left in place.
    fn undo(self: Box<Self>) -> Result<(), String>;
}

/// One dispatched paid LLM request, held by the transport that owns it.
///
/// The remote effect is kept apart from local publication: an interrupted
/// request may already have been accepted and billed by the provider, and no
/// local restore can undo that. Dropping the handle unresolved — a stop, a
/// timeout, a lost connection — records `unknown`, which no later success
/// erases.
/// ```
/// let token = vibe_core::cancellation::CancellationToken::new();
/// assert_eq!(token.llm_effect(), "not-started");
/// let request = token.llm_request_started();
/// assert_eq!(token.llm_effect(), "unknown");
/// drop(request);
/// token.llm_request_started().completed();
/// assert_eq!(token.llm_effect(), "unknown");
/// assert_eq!(token.publication_state(), "not-started");
/// ```
#[must_use = "an unresolved request handle records an unknown provider-side effect"]
pub struct LlmRequest {
    state: Arc<State>,
    resolved: bool,
}
impl LlmRequest {
    /// The provider answered: the exchange completed on the remote side.
    pub fn completed(mut self) {
        self.state.llm_completed.store(true, Ordering::SeqCst);
        self.resolve();
    }
    /// Proven never delivered (the connection was never established).
    pub fn not_delivered(mut self) {
        self.resolve();
    }
    fn resolve(&mut self) {
        if !self.resolved {
            self.resolved = true;
            self.state.llm_in_flight.fetch_sub(1, Ordering::SeqCst);
        }
    }
}
impl Drop for LlmRequest {
    fn drop(&mut self) {
        if !self.resolved {
            self.state.llm_unknown.store(true, Ordering::SeqCst);
            self.resolve();
        }
    }
}
impl std::fmt::Debug for LlmRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmRequest")
            .field("resolved", &self.resolved)
            .finish()
    }
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
    /// Hand a completed step's local effect to the operation's pre-commit
    /// rollback: [`Self::undo_completed_effects`] undoes it if a later stop
    /// rolls the operation back, and [`Self::committed`] releases it. An
    /// effect adopted after the operation committed is already part of the
    /// committed state and is released at once.
    /// ```
    /// use vibe_core::cancellation::{CancellationToken, CompletedEffect};
    /// struct Noted(std::sync::Arc<std::sync::atomic::AtomicBool>);
    /// impl CompletedEffect for Noted {
    ///     fn undo(self: Box<Self>) -> Result<(), String> {
    ///         self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    ///         Ok(())
    ///     }
    /// }
    /// let undone = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    /// let token = CancellationToken::new();
    /// token.publishing();
    /// token.adopt_completed_effect(Box::new(Noted(undone.clone())));
    /// token.cancel();
    /// assert!(token.checkpoint().is_err());
    /// assert_eq!(token.publication_state(), "recovery-required");
    /// assert!(token.undo_completed_effects().is_ok());
    /// assert!(undone.load(std::sync::atomic::Ordering::SeqCst));
    /// token.restored();
    /// assert_eq!(token.publication_state(), "restored");
    /// ```
    pub fn adopt_completed_effect(&self, effect: Box<dyn CompletedEffect>) {
        if self.state.publication.load(Ordering::SeqCst) == 2 {
            return;
        }
        self.completed_effects().push(effect);
    }
    /// Undo every adopted completed effect, newest first. Each undo runs even
    /// when an earlier one failed; a failure marks the operation's local
    /// effects unowned (so it can never be reported `restored`) and every
    /// failure's account is returned, joined.
    pub fn undo_completed_effects(&self) -> Result<(), String> {
        let effects = std::mem::take(&mut *self.completed_effects());
        let failures: Vec<String> = effects
            .into_iter()
            .rev()
            .filter_map(|effect| effect.undo().err())
            .collect();
        if failures.is_empty() {
            Ok(())
        } else {
            self.mark_unowned_effects();
            Err(failures.join("; "))
        }
    }
    /// Whether any adopted completed effect is still waiting for the
    /// operation's commit or rollback.
    pub fn has_completed_effects(&self) -> bool {
        !self.completed_effects().is_empty()
    }
    fn completed_effects(&self) -> std::sync::MutexGuard<'_, Vec<Box<dyn CompletedEffect>>> {
        self.state
            .completed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// Record one paid LLM request about to be dispatched. Remote effects
    /// never mark local publication as needing recovery.
    pub fn llm_request_started(&self) -> LlmRequest {
        self.state.llm_in_flight.fetch_add(1, Ordering::SeqCst);
        LlmRequest {
            state: self.state.clone(),
            resolved: false,
        }
    }
    /// Aggregate provider-side effect, in the cancellation report's closed
    /// `effect` vocabulary: `unknown` while any request is unresolved or was
    /// interrupted, `completed` when every dispatched request was answered.
    pub fn llm_effect(&self) -> &'static str {
        if self.state.llm_unknown.load(Ordering::SeqCst)
            || self.state.llm_in_flight.load(Ordering::SeqCst) > 0
        {
            "unknown"
        } else if self.state.llm_completed.load(Ordering::SeqCst) {
            "completed"
        } else {
            "not-started"
        }
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
    /// The operation's publication is durable: adopted completed effects are
    /// part of it now and are released, never undone.
    pub fn committed(&self) {
        self.state.publication.fetch_max(2, Ordering::SeqCst);
        self.completed_effects().clear();
    }
    /// Conservative classification until the recovery owner reports more.
    pub fn publication_state(&self) -> &'static str {
        let state = self.state.publication.load(Ordering::SeqCst);
        // A completed effect no rollback owner undid is still on disk.
        if state != 2
            && (self.state.unowned_effects.load(Ordering::SeqCst) || self.has_completed_effects())
        {
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

/// Resources owned by one operation, released with its last token clone.
#[derive(Default)]
struct Resources {
    slots: std::sync::Mutex<Vec<Arc<dyn std::any::Any + Send + Sync>>>,
}

/// A non-owning view of one operation's control, held by resources that the
/// operation itself owns so they never keep that operation alive.
/// ```
/// let token = vibe_core::cancellation::CancellationToken::new();
/// let weak = token.downgrade();
/// assert!(weak.upgrade().is_some());
/// drop(token);
/// assert!(weak.upgrade().is_none());
/// ```
#[derive(Clone)]
pub struct WeakCancellationToken {
    state: std::sync::Weak<State>,
    resources: std::sync::Weak<Resources>,
    poll: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    controlled: bool,
}
impl WeakCancellationToken {
    /// The operation's token while any owning clone is still alive.
    pub fn upgrade(&self) -> Option<CancellationToken> {
        Some(CancellationToken {
            state: self.state.upgrade()?,
            poll: self.poll.clone(),
            controlled: self.controlled,
            resources: self.resources.upgrade()?,
        })
    }
}
impl std::fmt::Debug for WeakCancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakCancellationToken")
            .field("controlled", &self.controlled)
            .finish_non_exhaustive()
    }
}
impl CancellationToken {
    /// A view that observes this operation without owning it.
    pub fn downgrade(&self) -> WeakCancellationToken {
        WeakCancellationToken {
            state: Arc::downgrade(&self.state),
            resources: Arc::downgrade(&self.resources),
            poll: self.poll.clone(),
            controlled: self.controlled,
        }
    }
    /// The one `T` this operation owns, created by `init` on first use and
    /// dropped with the operation's last token clone. A resource that must
    /// observe the operation holds [`Self::downgrade`], never a clone.
    /// ```
    /// let token = vibe_core::cancellation::CancellationToken::new();
    /// let first = token.operation_resource(|| 7_u32);
    /// let second = token.clone().operation_resource(|| 9_u32);
    /// assert_eq!((*first, *second), (7, 7));
    /// assert_eq!(*vibe_core::cancellation::CancellationToken::new().operation_resource(|| 9_u32), 9);
    /// ```
    pub fn operation_resource<T: std::any::Any + Send + Sync>(
        &self,
        init: impl FnOnce() -> T,
    ) -> Arc<T> {
        let mut slots = self
            .resources
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = slots
            .iter()
            .find_map(|slot| Arc::clone(slot).downcast::<T>().ok())
        {
            return existing;
        }
        let created = Arc::new(init());
        slots.push(created.clone());
        created
    }
    /// The operation's `T` only when an earlier call already created it.
    pub fn existing_operation_resource<T: std::any::Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.resources
            .slots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .find_map(|slot| Arc::clone(slot).downcast::<T>().ok())
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
    struct Effect {
        log: Arc<std::sync::Mutex<Vec<&'static str>>>,
        name: &'static str,
        fails: bool,
    }
    impl CompletedEffect for Effect {
        fn undo(self: Box<Self>) -> Result<(), String> {
            self.log
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(self.name);
            if self.fails {
                Err(format!("{}: changed replacement preserved", self.name))
            } else {
                Ok(())
            }
        }
    }
    fn effect(
        log: &Arc<std::sync::Mutex<Vec<&'static str>>>,
        name: &'static str,
        fails: bool,
    ) -> Box<dyn CompletedEffect> {
        Box::new(Effect {
            log: log.clone(),
            name,
            fails,
        })
    }
    #[test]
    fn completed_effects_are_undone_newest_first_and_a_conflict_is_never_restored() {
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let token = CancellationToken::new();
        token.publishing();
        token.adopt_completed_effect(effect(&log, "first", false));
        token
            .clone()
            .adopt_completed_effect(effect(&log, "second", true));
        token.adopt_completed_effect(effect(&log, "third", false));
        token.cancel();
        assert!(token.checkpoint().is_err());
        assert_eq!(token.publication_state(), "recovery-required");
        let reason = token.undo_completed_effects().unwrap_err();
        assert!(reason.contains("second"), "{reason}");
        assert_eq!(*log.lock().unwrap(), ["third", "second", "first"]);
        assert!(!token.has_completed_effects());
        token.restored();
        assert_eq!(
            token.publication_state(),
            "recovery-required",
            "a preserved foreign replacement is never reported restored"
        );
    }
    #[test]
    fn committed_effects_are_released_and_never_undone() {
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let token = CancellationToken::new();
        token.publishing();
        token.adopt_completed_effect(effect(&log, "before", false));
        token.committed();
        token.adopt_completed_effect(effect(&log, "after", false));
        assert!(!token.has_completed_effects());
        assert!(token.undo_completed_effects().is_ok());
        assert!(log.lock().unwrap().is_empty());
        assert_eq!(token.publication_state(), "committed");
    }
    #[test]
    fn interrupted_llm_request_stays_unknown_without_touching_publication() {
        let token = CancellationToken::new();
        let first = token.llm_request_started();
        token.llm_request_started().not_delivered();
        assert_eq!(token.llm_effect(), "unknown");
        token.cancel();
        assert!(token.checkpoint().is_err());
        drop(first);
        token.llm_request_started().completed();
        assert_eq!(token.llm_effect(), "unknown");
        assert_eq!(token.publication_state(), "not-started");
        let answered = CancellationToken::new();
        answered.llm_request_started().not_delivered();
        assert_eq!(answered.llm_effect(), "not-started");
        answered.llm_request_started().completed();
        assert_eq!(answered.llm_effect(), "completed");
    }
}
