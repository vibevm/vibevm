//! Cooperating-writer publication with retained exact partial outcomes.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-RECOVERY-LAW");
use super::*;
use journal::{FileState, Phase};
use vibe_lifecycle::LifecycleLease;
use vibe_safefs::{DirectoryDurability, GuardedPublishError, PublishStage};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOutcomeState {
    Completed,
    Partial,
}
#[derive(Debug)]
pub struct EditOutcome {
    pub state: EditOutcomeState,
    pub plan_id: String,
    pub journal: String,
    pub verified: Vec<ResourceId>,
    pub uncertain: Vec<ResourceId>,
    pub pending: Vec<ResourceId>,
    pub reason: Option<EditError>,
    pub durability_reduced: bool,
    /// The incumbent lease and journal can create infrastructure. A typed
    /// refusal never asserts absence of those effects merely from Err.
    pub infrastructure_may_exist: bool,
}
impl EditBarrier {
    /// Requires explicit authority already retained in this owner-minted plan.
    /// This is guarded publication, not destination-content atomic CAS.
    pub fn apply(&self) -> Result<EditOutcome, EditError> {
        let _executing = self.applying.try_lock().map_err(|_| EditError::Busy)?;
        let invalidate = std::cell::Cell::new(false);
        let _release = ReleaseOnExit {
            barrier: self,
            invalidate: &invalidate,
        };
        if let Err(error) = self.ready() {
            return Err(error);
        };
        let lease =
            LifecycleLease::acquire(self.plan.authority.root()).map_err(|error| match error {
                vibe_lifecycle::LifecycleLeaseError::Busy { .. } => EditError::Busy,
                _ => EditError::Unavailable,
            })?;
        let project = lease.document_edit_project();
        if project
            .root_correlation_token()
            .map_err(|_| EditError::Unavailable)?
            != journal::binding(&self.plan.authority)?
        {
            return Err(EditError::AuthorityRefused);
        };
        let registry = self
            .plan
            .authority
            .host
            .participants
            .lock()
            .map_err(|_| EditError::Unavailable)?;
        if registry.barrier != Some(self.id) {
            return Err(EditError::Stale);
        };
        self.check_participants()?;
        // Revalidate ALL current expected states before journal/first file;
        // repeat immediately before each replacement through SafeFS.
        for file in self.plan.files.values() {
            if project
                .read_file_snapshot_bounded(
                    &file.path,
                    self.plan.authority.host.limits.resource_bytes,
                )
                .map_err(|_| EditError::Stale)?
                .as_ref()
                != Some(&file.before)
            {
                return Err(EditError::Stale);
            };
        }
        let mut record = match &self.plan.recovery {
            Some(record) => record.clone(),
            None => {
                if journal::load(&self.plan.authority, &self.plan.id)?.is_some() {
                    return Err(EditError::AlreadyApplied);
                };
                journal::from_plan(&self.plan)?
            }
        };
        self.answers
            .lock()
            .map_err(|_| EditError::Unavailable)?
            .consumed = true;
        if let Err(error) = journal::checkpoint(project, &record) {
            drop(registry);
            return Err(error);
        }
        let mut reason = None;
        let mut reduced = false;
        for (id, file) in &self.plan.files {
            if let Err(error) = self.check_participants() {
                reason = Some(error);
                break;
            };
            record.phase = Phase::Publishing;
            if let Err(error) = journal::checkpoint(project, &record) {
                reason = Some(error);
                break;
            };
            if let Err(error) = self.check_participants() {
                reason = Some(error);
                break;
            };
            let row = record
                .files
                .iter_mut()
                .find(|r| r.path == file.path)
                .ok_or(EditError::InvalidJournal)?;
            // Recovery can observe a prior landed desired state. It never
            // republishes those bytes or substitutes old dirty-buffer intent.
            if file.before.bytes.as_slice() == file.desired.as_ref() {
                row.state = FileState::Verified;
                row.after_identity = Some(file.before.identity.observation_correlation_token());
            } else {
                match project.write_snapshot_guarded(&file.path, &file.before, &file.desired) {
                    Ok(write) => {
                        invalidate.set(true);
                        reduced |= write.parent != DirectoryDurability::Synced;
                        match project.read_file_snapshot_bounded(
                            &file.path,
                            self.plan.authority.host.limits.resource_bytes,
                        ) {
                            Ok(Some(s))
                                if s.bytes.as_slice() == file.desired.as_ref()
                                    && s.unix_mode == file.before.unix_mode =>
                            {
                                row.state = FileState::Verified;
                                row.after_identity =
                                    Some(s.identity.observation_correlation_token());
                            }
                            _ => {
                                row.state = FileState::Uncertain;
                                reason = Some(EditError::RecoveryRequired);
                            }
                        }
                        if !write.file_synced
                            || matches!(write.parent, DirectoryDurability::Failed(_))
                        {
                            reason = Some(EditError::RecoveryRequired)
                        };
                    }
                    Err(GuardedPublishError::Changed | GuardedPublishError::Unavailable) => {
                        reason = Some(EditError::Stale);
                    }
                    Err(GuardedPublishError::Publication(error)) => {
                        if error.stage == PublishStage::PossiblyPublished {
                            row.state = FileState::Uncertain;
                            invalidate.set(true);
                        }
                        reason = Some(EditError::RecoveryRequired);
                    }
                }
            }
            let _ = id;
            if reason.is_none() {
                if let Err(error) = self.check_participants() {
                    reason = Some(error);
                }
            }
            if let Err(error) = journal::checkpoint(project, &record) {
                reason = Some(error);
                break;
            };
            if reason.is_some() {
                break;
            };
        }
        record.phase = if reason.is_none() {
            Phase::Completed
        } else {
            Phase::Recovery
        };
        if let Err(error) = journal::checkpoint(project, &record) {
            reason = Some(error)
        };
        let mut verified = vec![];
        let mut uncertain = vec![];
        let mut pending = vec![];
        for row in &record.files {
            let id = *self
                .plan
                .files
                .iter()
                .find(|(_, f)| f.path == row.path)
                .ok_or(EditError::InvalidJournal)?
                .0;
            match row.state {
                FileState::Verified => verified.push(id),
                FileState::Uncertain => uncertain.push(id),
                FileState::Pending => pending.push(id),
            }
        }
        drop(registry);
        Ok(EditOutcome {
            state: if reason.is_none() {
                EditOutcomeState::Completed
            } else {
                EditOutcomeState::Partial
            },
            plan_id: self.plan.id.clone(),
            journal: journal::path(&self.plan.id)?,
            verified,
            uncertain,
            pending,
            reason,
            durability_reduced: reduced,
            infrastructure_may_exist: true,
        })
    }
    fn release(&self, invalidate: bool) {
        super::fence::release(
            &self.plan.authority,
            &self.plan.participants,
            self.id,
            invalidate,
        )
    }
}

struct ReleaseOnExit<'a> {
    barrier: &'a EditBarrier,
    invalidate: &'a std::cell::Cell<bool>,
}
impl Drop for ReleaseOnExit<'_> {
    fn drop(&mut self) {
        self.barrier.release(self.invalidate.get());
    }
}
