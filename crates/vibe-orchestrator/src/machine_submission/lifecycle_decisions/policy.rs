use super::*;

/// Trusted embedding facts, never decoded from an answer or wire policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPolicyFacts {
    pub approve_captured_script: bool,
    /// Separate trusted permission for an exact local Install. Existing script
    /// approval never implies this permission.
    pub approve_exact_local_install: bool,
    pub allow_detach: bool,
    pub allow_cancel_pre_dispatch: bool,
    pub allow_cancel_deferred_boundary: bool,
}
impl HostPolicyFacts {
    pub(super) fn permits(&self, control: &RequestedOptionsControl) -> bool {
        match control {
            RequestedOptionsControl::None => true,
            RequestedOptionsControl::Detach => self.allow_detach,
            RequestedOptionsControl::CancelPreDispatch => self.allow_cancel_pre_dispatch,
            RequestedOptionsControl::CancelDeferredBoundary => self.allow_cancel_deferred_boundary,
        }
    }
    /// The existing fixed Validate profile stays byte-compatible. Its
    /// generation changes when a shared policy enables Install separately.
    pub(super) fn identity(&self) -> [bool; 4] {
        [
            self.approve_captured_script,
            self.allow_detach,
            self.allow_cancel_pre_dispatch,
            self.allow_cancel_deferred_boundary,
        ]
    }
}
pub(super) struct State {
    pub facts: HostPolicyFacts,
    pub snapshot: Option<super::super::PolicySnapshot>,
    pub revision: u64,
    pub counter: u64,
    pub sequences: Vec<Sequence>,
}
impl State {
    pub fn observe_snapshot(
        &mut self,
        snapshot: super::super::PolicySnapshot,
    ) -> Result<(), FixedDecisionError> {
        if self.snapshot.as_ref() != Some(&snapshot) {
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(FixedDecisionError::Capacity)?;
            self.facts = snapshot.facts().clone();
            self.snapshot = Some(snapshot);
            self.sequences.clear();
        }
        Ok(())
    }

    pub fn generation(&self, issuer: &str) -> String {
        self.snapshot.as_ref().map_or_else(
            || format!("{issuer}:policy:{}", self.revision),
            |snapshot| snapshot.generation().to_owned(),
        )
    }
    pub fn question_id(&mut self, issuer: &str) -> Result<String, FixedDecisionError> {
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(FixedDecisionError::Capacity)?;
        Ok(format!("{issuer}:question:{}", self.counter))
    }
}
