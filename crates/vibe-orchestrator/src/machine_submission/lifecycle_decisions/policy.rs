use super::*;

/// Trusted embedding facts, never decoded from an answer or wire policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPolicyFacts {
    pub approve_captured_script: bool,
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
    pub revision: u64,
    pub counter: u64,
    pub sequences: Vec<Sequence>,
}
impl State {
    pub fn generation(&self, issuer: &str) -> String {
        format!("{issuer}:policy:{}", self.revision)
    }
    pub fn question_id(&mut self, issuer: &str) -> Result<String, FixedDecisionError> {
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(FixedDecisionError::Capacity)?;
        Ok(format!("{issuer}:question:{}", self.counter))
    }
}
