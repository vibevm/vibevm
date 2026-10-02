//! Sealed owned profile, never caller-provided canonical bytes.
use super::*;
use vibe_lifecycle::LifecycleLease;

pub(in crate::machine_submission) struct AdmittedProfile {
    evidence: ValidatedFixedPlanEvidence,
    state: Arc<Mutex<State>>,
    root: Arc<Project>,
    root_token: String,
    scope: SubmissionScope,
    facts: HostPolicyFacts,
    generation: String,
    #[cfg(test)]
    pub(in crate::machine_submission) after_ready: Option<Arc<dyn Fn() + Send + Sync>>,
    #[cfg(test)]
    pub(super) before_guard: Option<Arc<dyn Fn() + Send + Sync>>,
}
impl AdmittedProfile {
    pub(super) fn from_owned(
        host: &FixedLifecycleDecisionHost,
        evidence: &ValidatedFixedPlanEvidence,
    ) -> Result<Self, FixedDecisionError> {
        let (facts, generation) = {
            let state = host.state.lock().map_err(infrastructure)?;
            Self::check_record(&state, evidence)?;
            (state.facts.clone(), state.generation(&host.issuer))
        };
        let root_token = host
            .project
            .root_correlation_token()
            .map_err(infrastructure)?;
        let root = host.project.clone();
        if root.root_correlation_token().map_err(infrastructure)? != root_token {
            return Err(FixedDecisionError::ForeignScope);
        }
        Ok(Self {
            evidence: evidence.clone(),
            state: host.state.clone(),
            root,
            root_token,
            scope: host.scope.clone(),
            facts,
            generation,
            #[cfg(test)]
            before_guard: None,
            #[cfg(test)]
            after_ready: None,
        })
    }
    pub(in crate::machine_submission) fn control_mode(&self) -> RequestedOptionsControl {
        self.evidence.options.control.clone()
    }
    fn check_record(
        state: &State,
        evidence: &ValidatedFixedPlanEvidence,
    ) -> Result<(), FixedDecisionError> {
        if state.revision != evidence.revision {
            return Err(FixedDecisionError::StalePolicy);
        }
        if !state.facts.approve_captured_script || !state.facts.permits(&evidence.options.control) {
            return Err(FixedDecisionError::PolicyDenied);
        }
        let sequence = state
            .sequences
            .iter()
            .find(|s| {
                Arc::ptr_eq(&s.base, &evidence.base)
                    && s.approved == Some(true)
                    && s.approval
                        .as_ref()
                        .unwrap_or(&s.original)
                        .question_id
                        .as_deref()
                        == Some(&evidence.question_id)
            })
            .ok_or(FixedDecisionError::UnknownQuestion)?;
        let q = sequence.approval.as_ref().unwrap_or(&sequence.original);
        if q.plan_id.as_deref() != Some(&evidence.identity)
            || q.requested_options != evidence.options
        {
            return Err(FixedDecisionError::InvalidData);
        }
        Ok(())
    }
    pub(in crate::machine_submission) fn check_root_path(
        &self,
        path: &std::path::Path,
    ) -> Result<(), FixedDecisionError> {
        if self.root.root_correlation_token().map_err(infrastructure)? != self.root_token
            || Project::open(path)
                .map_err(infrastructure)?
                .root_correlation_token()
                .map_err(infrastructure)?
                != self.root_token
        {
            return Err(FixedDecisionError::ForeignScope);
        }
        Ok(())
    }
    pub(in crate::machine_submission) fn guard(
        &self,
        lease: &LifecycleLease,
        request: &[u8],
        plan: &[u8],
        source: &vibe_lifecycle::process::ScriptSourceWitness,
    ) -> anyhow::Result<()> {
        #[cfg(test)]
        if let Some(hook) = &self.before_guard {
            hook();
        }
        self.check_root_path(lease.root())?;
        anyhow::ensure!(
            lease
                .publication_journal_project()
                .root_correlation_token()?
                == self.root_token,
            "admitted lease root changed"
        );
        anyhow::ensure!(
            request == self.evidence.base.request && plan == self.evidence.base.plan,
            "approved v4 base changed before C1 admission"
        );
        // Linearization: the current owned approval is consumed here under the
        // protected writer lease, immediately before reservation. A later policy
        // update affects later admissions, not this already admitted dispatch.
        // This is admission-time validation, not a revocable prepared token:
        // policy changes after this check do not stop this run after Ready.
        // The policy cell is released before journal/filesystem/provider IO.
        vibe_lifecycle::process::recheck_script_source_witness(source)?;
        let state = self.state.lock().map_err(infrastructure)?;
        Self::check_record(&state, &self.evidence)?;
        Ok(())
    }
    fn wrap(&self, kind: &str, base: &[u8]) -> anyhow::Result<Vec<u8>> {
        let base: serde_json::Value = serde_json::from_slice(base)?;
        let value = serde_json::json!({
            "format": kind, "base_v4": base, "base_plan_sha256": self.evidence.base.plan_id,
            "accepted_plan": self.evidence.identity, "requested_options": self.evidence.options,
            "policy_generation": self.generation, "host_policy": self.facts.identity(),
            "scope": { "context": self.scope.context, "target": self.scope.target, "principal": self.scope.principal }
        });
        let mut out = Vec::new();
        super::super::super::lifecycle_plan::write_canonical(&value, &mut out)?;
        anyhow::ensure!(out.len() <= CANONICAL_CAP, "admitted canonical capacity");
        Ok(out)
    }
    pub(in crate::machine_submission) fn request(&self, base: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.wrap("vibevm.lifecycle.admitted-no-control-request/1", base)
    }
    pub(in crate::machine_submission) fn plan(&self, base: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.wrap("vibevm.lifecycle.admitted-no-control-plan/1", base)
    }
}
