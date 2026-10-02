//! Bounded memory tables retain actual opaque owner objects. No independent journal.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use vibe_orchestrator::machine_submission::{
    FixedDecisionError, FixedLifecycleDecisionHost, FixedOwnerEvidenceSnapshot, ScopedFixedJob,
    ValidatedFixedPlanEvidence,
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
pub(crate) struct Bridge {
    pub(crate) host: Arc<FixedLifecycleDecisionHost>,
    tables: Mutex<Tables>,
    pub(crate) observers: super::observers::Observers,
}
impl Bridge {
    pub(crate) fn new(host: Arc<FixedLifecycleDecisionHost>) -> Self {
        Self {
            host,
            tables: Mutex::new(Tables::default()),
            observers: Default::default(),
        }
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
    ) -> Result<ScopedFixedJob, FixedDecisionError> {
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
            let slot = t.jobs.get_mut(key).expect("reserved key");
            slot.claims -= 1;
            if let Ok(job) = &result {
                slot.job = Some(job.clone());
            }
            if slot.claims == 0 && slot.job.is_none() {
                t.jobs.remove(key);
            }
        }
        result
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
            Some(j) => Ok(Some(j.evidence())),
            None => self.host.inspect_existing_evidence(key),
        }
    }
}
