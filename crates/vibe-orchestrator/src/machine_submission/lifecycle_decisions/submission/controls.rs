//! Scoped authority is retained as an opaque owner plus its admitted policy.
use super::*;
use vibe_wire::generated::machine::e1::operation_stream_message::OperationStreamMessage;

/// Actual owned job with one sealed, admitted control policy.
/// Later host policy changes affect new admissions, not this admitted run.
/// ```compile_fail
/// use vibe_orchestrator::machine_submission::ScopedFixedJob;
/// fn promote(job: &ScopedFixedJob) { job.grant_control(); }
/// ```
#[derive(Clone)]
pub struct ScopedFixedJob {
    owner: HostOwnedFixedOperation,
    mode: RequestedOptionsControl,
}
impl ScopedFixedJob {
    pub fn wait_ready(&self, timeout: Duration) -> anyhow::Result<Readiness> {
        self.owner.wait_ready(timeout)
    }
    pub fn evidence(&self) -> super::super::super::lifecycle_adapter::FixedOwnerEvidenceSnapshot {
        self.owner.evidence()
    }
    pub fn inspect(&self) -> anyhow::Result<FixedOperationStatus> {
        self.owner.inspect()
    }
    pub fn attach(&self) -> anyhow::Result<ObservationHandle> {
        self.owner.attach()
    }
    pub fn control(
        &self,
        observer: &mut ObservationHandle,
        message: &OperationStreamMessage,
    ) -> anyhow::Result<OperationStreamMessage> {
        self.owner.scoped_control(observer, message, &self.mode)
    }
}
impl FixedLifecycleDecisionHost {
    /// Admit locally approved opaque evidence through the same bounded C1 owner.
    /// Deferred cancellation is unavailable before dispatch and requests only a
    /// native exit/drain/callback boundary after dispatch. It has no deadline,
    /// provider kill or rollback promise.
    pub fn submit_scoped(
        &self,
        evidence: &ValidatedFixedPlanEvidence,
        key: &str,
        observed_at: Timestamp,
    ) -> Result<ScopedFixedJob, FixedDecisionError> {
        Ok(ScopedFixedJob {
            owner: self.submit_owned(evidence, key, observed_at)?,
            mode: evidence.options.control.clone(),
        })
    }
}
#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
