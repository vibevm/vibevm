//! Trusted shared-policy integration; questions and approvals remain local.
use super::super::{PolicyError, PolicyOwner};
use super::*;

pub(super) fn policy_error(error: PolicyError) -> FixedDecisionError {
    match error {
        PolicyError::ForeignScope => FixedDecisionError::ForeignScope,
        other => infrastructure(other),
    }
}

impl FixedLifecycleDecisionHost {
    /// Attach existing owned policy and an existing namespace. This never
    /// initializes policy, overwrites a record or issues a namespace.
    pub fn from_shared_owner(
        root: PathBuf,
        namespace: SubmissionNamespace,
        scope: SubmissionScope,
        target_os: TargetOs,
        owner: Arc<PolicyOwner>,
    ) -> Result<Self, FixedDecisionError> {
        if owner.scope() != &scope {
            return Err(FixedDecisionError::ForeignScope);
        }
        let project = Arc::new(Project::open(&root).map_err(infrastructure)?);
        let issuer = propose_run_id(&root).map_err(infrastructure)?;
        let mut host = Self {
            root,
            project,
            namespace,
            scope,
            target_os,
            issuer,
            shared_owner: Some(owner.clone()),
            policy_access: Arc::new(Mutex::new(())),
            #[cfg(test)]
            after_shared_read: None,
            #[cfg(test)]
            before_shared_refresh_access: None,
            holder: Arc::new(()),
            jobs: Mutex::new(Vec::new()),
            state: Arc::new(Mutex::new(State {
                facts: HostPolicyFacts {
                    approve_captured_script: false,
                    allow_detach: false,
                    allow_cancel_pre_dispatch: false,
                    allow_cancel_deferred_boundary: false,
                },
                snapshot: None,
                revision: 1,
                counter: 0,
                sequences: vec![],
            })),
        };
        let leased = host.scope_lease()?;
        let snapshot = owner
            .current_under_lease(&leased.retain_lease())
            .map_err(policy_error)?;
        host.state = Arc::new(Mutex::new(State {
            facts: snapshot.facts().clone(),
            snapshot: Some(snapshot),
            revision: 1,
            counter: 0,
            sequences: vec![],
        }));
        Ok(host)
    }

    /// Observe outside the cell without contending with an active provider.
    /// This only refreshes comparisons; the actual guard rereads under C1's
    /// existing writer lease before admitting any effects.
    pub(super) fn observe_shared(&self) -> Result<(), FixedDecisionError> {
        let Some(owner) = &self.shared_owner else {
            return Ok(());
        };
        // Readonly paths never acquire a writer lease while holding access.
        let _access = self.policy_access.lock().map_err(infrastructure)?;
        let snapshot = owner.observe_readonly().map_err(policy_error)?;
        #[cfg(test)]
        if let Some(hook) = &self.after_shared_read {
            hook(&snapshot);
        }
        self.state
            .lock()
            .map_err(infrastructure)?
            .observe_snapshot(snapshot)
    }

    /// Read outside the cell, then invalidate local records with a checked
    /// revision. Return the very same outer lease for fresh source capture.
    pub(super) fn refresh_shared(
        &self,
    ) -> Result<Option<LeasedDefaultLifecycle>, FixedDecisionError> {
        let Some(owner) = &self.shared_owner else {
            return Ok(None);
        };
        let leased = self.scope_lease()?;
        #[cfg(test)]
        if let Some(hook) = &self.before_shared_refresh_access {
            hook(&leased.retain_lease());
        }
        // Writer lease precedes access; State never spans filesystem reads.
        let _access = self.policy_access.lock().map_err(infrastructure)?;
        let snapshot = owner
            .current_under_lease(&leased.retain_lease())
            .map_err(policy_error)?;
        let mut state = self.state.lock().map_err(infrastructure)?;
        state.observe_snapshot(snapshot)?;
        drop(state);
        Ok(Some(leased))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests;
#[cfg(test)]
mod tests;
