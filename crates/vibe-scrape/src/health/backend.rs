//! Sealed OS-enforcement boundary. The portable core never calls `Command`.

use super::model::{
    Applicability, BackendCapabilities, BackendCommandRequest, CommandExecution, HealthBlocker,
    HealthError, PhaseContext, PreparedHealth,
};
use super::tree::TreeSeal;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// An implementation is trusted only after adversarial platform tests prove
/// every advertised capability. The trait is sealed to this crate.
pub trait HealthBackend: sealed::Sealed {
    fn capabilities(&self) -> BackendCapabilities;

    fn execute(
        &mut self,
        request: BackendCommandRequest<'_>,
    ) -> Result<CommandExecution, HealthError>;

    /// Re-observe the real source/delivered tree, not merely the disposable
    /// phase view. Returning the expected value without observation is a
    /// backend defect.
    fn reprove_tree(&mut self, context: &PhaseContext) -> Result<TreeSeal, HealthError>;
}

#[must_use]
pub fn capability_blockers(
    prepared: &PreparedHealth,
    capabilities: BackendCapabilities,
    same_display_path_required: bool,
) -> Vec<HealthBlocker> {
    let mut blockers = Vec::new();
    for check in prepared
        .checks
        .iter()
        .filter(|check| check.applicability == Applicability::Applicable)
    {
        let required = check.sandbox;
        for (needed, present, code, label) in [
            (
                required.exact_executable_identity,
                capabilities.exact_executable_identity,
                "health-exact-exec-unavailable",
                "exact executable identity",
            ),
            (
                required.filesystem_isolation,
                capabilities.filesystem_isolation,
                "health-filesystem-isolation-unavailable",
                "filesystem isolation",
            ),
            (
                required.read_policy_enforcement,
                capabilities.read_policy_enforcement,
                "health-read-policy-unavailable",
                "read policy enforcement",
            ),
            (
                required.process_tree_containment,
                capabilities.process_tree_containment,
                "health-process-tree-unavailable",
                "process-tree containment",
            ),
            (
                required.graceful_termination,
                capabilities.graceful_termination,
                "health-graceful-termination-unavailable",
                "graceful-then-forced process-tree termination",
            ),
            (
                required.spawn_prevention,
                capabilities.spawn_prevention,
                "health-spawn-prevention-unavailable",
                "spawn prevention",
            ),
            (
                required.network_deny,
                capabilities.network_deny,
                "health-network-deny-unavailable",
                "OS-enforced network denial",
            ),
            (
                required.bounded_output,
                capabilities.bounded_output,
                "health-output-bound-unavailable",
                "bounded concurrent output",
            ),
            (
                required.atomic_result,
                capabilities.atomic_result,
                "health-atomic-result-unavailable",
                "an atomic structured-result channel",
            ),
            (
                required.bundle_materialization,
                capabilities.bundle_materialization,
                "health-bundle-materialization-unavailable",
                "capability-relative verified verifier-bundle publication",
            ),
            (
                same_display_path_required,
                capabilities.same_display_path_view,
                "health-same-path-view-unavailable",
                "same-display-path isolated view",
            ),
        ] {
            if needed && !present {
                blockers.push(HealthBlocker {
                    code: code.to_owned(),
                    check_id: Some(check.id.clone()),
                    message: format!("healthcheck `{}` requires {label}", check.id),
                });
            }
        }
    }
    blockers.sort_by(|left, right| {
        (&left.code, &left.check_id, &left.message).cmp(&(
            &right.code,
            &right.check_id,
            &right.message,
        ))
    });
    blockers.dedup();
    blockers
}

/// Default for a platform without a proven enforcement implementation.
pub struct UnsupportedBackend {
    reason: String,
}

impl UnsupportedBackend {
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl sealed::Sealed for UnsupportedBackend {}

impl HealthBackend for UnsupportedBackend {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    fn execute(
        &mut self,
        _request: BackendCommandRequest<'_>,
    ) -> Result<CommandExecution, HealthError> {
        Err(HealthError::Unsupported(self.reason.clone()))
    }

    fn reprove_tree(&mut self, _context: &PhaseContext) -> Result<TreeSeal, HealthError> {
        Err(HealthError::Unsupported(self.reason.clone()))
    }
}
