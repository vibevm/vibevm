//! First admitted profile only; general C4 controls and adapters remain open.
use super::super::lifecycle_adapter::{
    FixedOperationStatus, HostOwnedFixedOperation, ObservationHandle, Readiness,
};
use super::*;
use std::time::Duration;
use vibe_wire::generated::shared::Timestamp;
pub(in crate::machine_submission) mod profile;
use profile::AdmittedProfile;

/// Owns the actual incumbent job without exposing a control grant.
/// ```compile_fail
/// use vibe_orchestrator::machine_submission::ReadonlyFixedJob;
/// fn control(job: &ReadonlyFixedJob) { job.grant_control(); }
/// ```
#[derive(Clone)]
pub struct ReadonlyFixedJob {
    owner: HostOwnedFixedOperation,
}
impl ReadonlyFixedJob {
    pub fn wait_ready(&self, timeout: Duration) -> anyhow::Result<Readiness> {
        self.owner.wait_ready(timeout)
    }
    pub fn inspect(&self) -> anyhow::Result<FixedOperationStatus> {
        self.owner.inspect()
    }
    pub fn attach_readonly(&self) -> anyhow::Result<ObservationHandle> {
        self.owner.attach()
    }
}
pub(super) struct JobEntry {
    key: String,
    request_id: String,
    plan_id: String,
    job: ReadonlyFixedJob,
}
impl FixedLifecycleDecisionHost {
    /// Consume sealed, locally approved evidence into the same C1/C2 owner.
    /// Other controls retain an explicit gap; infrastructure may be written on
    /// failures. Readiness and effect truth come exclusively from the owner.
    pub fn submit_no_control(
        &self,
        evidence: &ValidatedFixedPlanEvidence,
        key: &str,
        observed_at: Timestamp,
    ) -> Result<ReadonlyFixedJob, FixedDecisionError> {
        // Match the C1 owner's key bound before claiming a background job.
        if key.is_empty()
            || key.len() > 256
            || key.trim() != key
            || key.chars().any(char::is_control)
        {
            return Err(FixedDecisionError::InvalidData);
        }
        supported(&evidence.options)?;
        if evidence.options.control != RequestedOptionsControl::None {
            return Err(FixedDecisionError::ImplementationGap(
                "admitted submission implements only control=None; full C4 remains open".into(),
            ));
        }
        if !Arc::ptr_eq(&self.holder, &evidence.holder) {
            return Err(FixedDecisionError::ForeignScope);
        }
        let profile = AdmittedProfile::from_owned(self, evidence)?;
        profile.check_root_path(&self.root)?;
        let request_id = digest(
            profile
                .request(&evidence.base.request)
                .map_err(infrastructure)?,
        );
        let plan_id = digest(profile.plan(&evidence.base.plan).map_err(infrastructure)?);
        // The starting claim is atomic. Factory launch does no filesystem or
        // provider IO; the worker revalidates under its own protected lease.
        let mut jobs = self.jobs.lock().map_err(infrastructure)?;
        if let Some(entry) = jobs.iter().find(|entry| entry.key == key) {
            if entry.request_id != request_id || entry.plan_id != plan_id {
                return Err(FixedDecisionError::AnswerConflict);
            }
            return Ok(entry.job.clone());
        }
        if jobs.len() >= 8 {
            return Err(FixedDecisionError::Capacity);
        }
        let owner = HostOwnedFixedOperation::start_admitted(
            self.root.clone(),
            self.namespace.clone(),
            self.scope.clone(),
            key.into(),
            self.target_os,
            observed_at,
            profile,
        )
        .map_err(infrastructure)?;
        let job = ReadonlyFixedJob { owner };
        jobs.push(JobEntry {
            key: key.into(),
            request_id,
            plan_id,
            job: job.clone(),
        });
        Ok(job)
    }

    /// Cold inspection is observation, never renewed approval or dispatch.
    /// It reads the original C1/capsule/marker without reading current source.
    pub fn inspect_existing(
        &self,
        key: &str,
    ) -> Result<Option<FixedOperationStatus>, FixedDecisionError> {
        let leased = self.scope_lease()?;
        super::super::lifecycle_adapter::inspect_fixed_existing(
            &leased.retain_lease(),
            &self.namespace,
            &self.scope,
            key,
        )
        .map_err(infrastructure)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
