//! Trusted fixed Validate route shared by explicit library and adapter grants.
//! Bounded memory tables retain opaque owner objects, never a second journal.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");
mod contract;
mod dispatch;
mod envelope;
mod error;
pub use envelope::{DOMAIN_OWNER, OPERATION_ID};
mod observers;
pub use contract::{PUBLICATION_CAP, REQUEST_CAP};
pub use error::FixedLifecycleRouteError;
#[cfg(all(test, target_os = "linux"))]
mod native_tests;
use crate::machine_submission::{
    FixedDecisionError, FixedLifecycleDecisionHost, FixedOwnerEvidenceSnapshot, ScopedFixedJob,
    ValidatedFixedPlanEvidence,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
struct Slot {
    job: Option<ScopedFixedJob>,
    claims: usize,
}
#[derive(Default)]
struct Tables {
    accepted: BTreeMap<String, ValidatedFixedPlanEvidence>,
    jobs: BTreeMap<String, Slot>,
}
fn stopped_without_run(snapshot: &FixedOwnerEvidenceSnapshot) -> bool {
    snapshot.bound_run_id().is_none()
        && matches!(
            snapshot.status(),
            Some(
                crate::machine_submission::FixedOperationStatus::RecoveryRequired {
                    run_id: None,
                    ..
                }
            )
        )
}
/// Trusted fixed Validate action route with bounded connection-local references.
/// Construction retains the embedding's existing host; it initializes no namespace,
/// policy or provider. Every dispatch rechecks the embedding's physical root.
/// The host remains the sole decision, C1/C2 journal and control owner.
///
/// ```no_run
/// use std::{path::Path, sync::Arc};
/// use vibe_orchestrator::{machine_operation::fixed_lifecycle::FixedLifecycleRoute,
///     machine_submission::FixedLifecycleDecisionHost};
/// fn inspect(host: Arc<FixedLifecycleDecisionHost>, root: &Path) {
///     let route = FixedLifecycleRoute::new(host);
///     let bytes = br#"{"schema":1,"action":{"kind":"inspect","key":"once"}}"#;
///     let result = FixedLifecycleRoute::dispatch(Some(&route), bytes, root);
///     if let Err(refusal) = result {
///         // An error cannot establish absence of effects.
///         let _ = (refusal.error, refusal.effects_may_have_begun);
///     }
/// }
/// ```
/// Data alone cannot construct the route:
/// ```compile_fail
/// use vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute;
/// let route = FixedLifecycleRoute::new(serde_json::json!({"grant":true}));
/// ```
pub struct FixedLifecycleRoute {
    host: Arc<FixedLifecycleDecisionHost>,
    tables: Mutex<Tables>,
    observers: observers::Observers,
}
impl FixedLifecycleRoute {
    /// Retain an explicit trusted host grant. See the type example for canonical use.
    #[must_use]
    pub fn new(host: Arc<FixedLifecycleDecisionHost>) -> Self {
        Self {
            host,
            tables: Mutex::new(Tables::default()),
            observers: Default::default(),
        }
    }
    /// Read-only physical identity comparison; see the type's dispatch example.
    pub fn compare_context_root(&self, root: &std::path::Path) -> Result<(), FixedDecisionError> {
        self.host.compare_context_root(root)
    }
    pub(crate) fn retain(
        &self,
        id: String,
        e: ValidatedFixedPlanEvidence,
    ) -> Result<(), FixedDecisionError> {
        let mut t = self
            .tables
            .lock()
            .map_err(|_| FixedDecisionError::Capacity)?;
        if !t.accepted.contains_key(&id) && t.accepted.len() >= 4 {
            return Err(FixedDecisionError::Capacity);
        }
        t.accepted.insert(id, e);
        Ok(())
    }
    pub(crate) fn accepted(&self, id: &str) -> Option<ValidatedFixedPlanEvidence> {
        self.tables.lock().ok()?.accepted.get(id).cloned()
    }
    pub(crate) fn submit(
        &self,
        e: &ValidatedFixedPlanEvidence,
        key: &str,
    ) -> Result<FixedOwnerEvidenceSnapshot, FixedDecisionError> {
        {
            let mut t = self
                .tables
                .lock()
                .map_err(|_| FixedDecisionError::Capacity)?;
            if !t.jobs.contains_key(key) && t.jobs.len() >= 8 {
                return Err(FixedDecisionError::Capacity);
            }
            t.jobs
                .entry(key.into())
                .or_insert(Slot {
                    job: None,
                    claims: 0,
                })
                .claims += 1;
        }
        // Revalidate with the actual owner even for an existing key; never attach by string.
        let result = self.host.submit_scoped(
            e,
            key,
            chrono::Utc::now()
                .to_rfc3339()
                .parse()
                .map_err(|_| FixedDecisionError::InvalidData)?,
        );
        {
            let mut t = self
                .tables
                .lock()
                .map_err(|_| FixedDecisionError::Capacity)?;
            let slot = t.jobs.get_mut(key).ok_or(FixedDecisionError::Capacity)?;
            slot.claims -= 1;
            if let Ok(job) = &result {
                slot.job = Some(job.clone());
            }
            if slot.claims == 0 && slot.job.is_none() {
                t.jobs.remove(key);
            }
        }
        let job = result?;
        let live = job.evidence();
        if stopped_without_run(&live) {
            if let Some(retained) = self.host.matching_existing_evidence(e, key)? {
                return Ok(retained);
            }
        }
        Ok(live)
    }
    pub(crate) fn retained_job(
        &self,
        key: &str,
    ) -> Result<Option<ScopedFixedJob>, FixedDecisionError> {
        Ok(self
            .tables
            .lock()
            .map_err(|_| FixedDecisionError::Capacity)?
            .jobs
            .get(key)
            .and_then(|s| s.job.clone()))
    }
    pub(crate) fn inspect(
        &self,
        key: &str,
    ) -> Result<Option<FixedOwnerEvidenceSnapshot>, FixedDecisionError> {
        let job = {
            self.tables
                .lock()
                .map_err(|_| FixedDecisionError::Capacity)?
                .jobs
                .get(key)
                .and_then(|s| s.job.clone())
        };
        match job {
            Some(j) => {
                let live = j.evidence();
                // A concurrent surface can win the shared C1 key while this
                // connection's worker stops on the incumbent nonblocking
                // lease. Its local no-run failure is not the key's retained
                // truth. Prefer an independently validated C1/capsule/outcome
                // observation once the winning owner releases the lease.
                if stopped_without_run(&live) {
                    match self.host.inspect_existing_evidence(key) {
                        Ok(Some(retained)) => return Ok(Some(retained)),
                        Ok(None) => {}
                        Err(error) => return Err(error),
                    }
                }
                Ok(Some(live))
            }
            None => self.host.inspect_existing_evidence(key),
        }
    }
}
