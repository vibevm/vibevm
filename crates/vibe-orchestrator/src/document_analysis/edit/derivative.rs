//! Upper composition of pure rebase selection with registered M16 editing.
//! Result publication is a separate create-new effect after fences are released.
use super::derivative_journal as dj;
use super::*;
use std::time::Duration;
use vibe_wire::generated::derivative_rebase_journal::DerivativeRebaseJournal as Record;
use vibe_workspace::derivatives::{RebaseHandoff, capture_retained};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivativeEffect {
    Completed,
    Pending,
    Unknown,
}
#[derive(Debug)]
pub struct DerivativeOutcome {
    pub transaction: String,
    pub authored: DerivativeEffect,
    pub result: DerivativeEffect,
    pub edit: Option<EditOutcome>,
    pub reason: Option<EditError>,
    pub durability_reduced: bool,
}
#[derive(Debug)]
pub struct DerivativeRebasePlan {
    edit: EditPlan,
    record: Record,
}
#[derive(Debug)]
pub struct DerivativeRebaseBarrier {
    edit: EditBarrier,
    record: Record,
}
fn selected_output(
    authority: &DocumentEditAuthority,
    output: &Project,
) -> Result<String, EditError> {
    if Project::open(output.root_path())
        .and_then(|p| p.root_identity())
        .map_err(|_| EditError::Unavailable)?
        != output.root_identity().map_err(|_| EditError::Unavailable)?
    {
        return Err(EditError::Stale);
    }
    // Existing capability owner proves physical as well as lexical disjointness.
    vibe_safefs::ExternalStore::open_existing_disjoint(output.root_path(), &authority.host.project)
        .map_err(|_| EditError::AuthorityRefused)?
        .ok_or(EditError::AuthorityRefused)?;
    output
        .root_correlation_token()
        .map_err(|_| EditError::Unavailable)
}
fn recapture(
    r: &Record,
    base: &Project,
    local: &Project,
    output: &Project,
) -> Result<(), EditError> {
    if output
        .root_correlation_token()
        .map_err(|_| EditError::Unavailable)?
        != r.result_root
    {
        return Err(EditError::AuthorityRefused);
    }
    for source in [base, local] {
        vibe_safefs::ExternalStore::open_existing_disjoint(output.root_path(), source)
            .map_err(|_| EditError::AuthorityRefused)?
            .ok_or(EditError::AuthorityRefused)?;
    }
    let binding = dj::result(&r.desired_result)?
        .checked_binding()
        .map_err(|_| EditError::JournalIntegrity)?;
    capture_retained(base, binding.base()).map_err(|_| EditError::Stale)?;
    capture_retained(local, binding.local()).map_err(|_| EditError::Stale)?;
    Ok(())
}
impl DocumentEditAuthority {
    /// Read the retained before/desired results and prior transaction link.
    /// These are observed provenance, not a grant or a production capture proof.
    pub fn inspect_derivative_rebase_intent(&self, id: &str) -> Result<Record, EditError> {
        let record = dj::load(self, id)?;
        dj::chain(self, &record)?;
        Ok(record)
    }

    /// Output is explicitly selected, disjoint and initially absent. This does
    /// not widen authored resource eligibility to infrastructure directories.
    pub fn prepare_derivative_rebase(
        self: &Arc<Self>,
        capture: &AnalysisCapture,
        resource: ResourceId,
        handoff: RebaseHandoff,
        output: &Project,
        name: &str,
        prior: Option<&str>,
    ) -> Result<DerivativeRebasePlan, EditError> {
        vibe_safefs::ensure_safe_component(name).map_err(|_| EditError::AuthorityRefused)?;
        let result_root = selected_output(self, output)?;
        if output
            .read_file_bounded(name, dj::CAP)
            .map_err(|_| EditError::AuthorityRefused)?
            .is_some()
        {
            return Err(EditError::ThirdState);
        }
        let saved = capture
            .resource(resource)
            .and_then(|r| r.disk_observation())
            .ok_or(EditError::SaveRequired)?;
        if saved.bytes != handoff.previous().record().binding_toml.as_bytes() {
            return Err(EditError::Stale);
        }
        let edit = self.prepare(
            capture,
            vec![DocumentReplacement {
                resource,
                bytes: Arc::from(handoff.selected().record().binding_toml.as_bytes()),
            }],
        )?;
        let mut record = Record {
            schema: 1,
            recipe: "derivative-rebase-journal/1".into(),
            id: String::new(),
            edit_intent: serde_json::to_string(&journal::from_plan(&edit)?)
                .map_err(|_| EditError::InvalidJournal)?,
            previous_result: serde_json::to_string(handoff.previous().record())
                .map_err(|_| EditError::InvalidJournal)?,
            desired_result: serde_json::to_string(handoff.selected().record())
                .map_err(|_| EditError::InvalidJournal)?,
            result_root,
            result_name: name.into(),
            prior: prior.map(str::to_owned),
        };
        record.id = dj::identity(&record)?;
        dj::validate(self, &record)?;
        dj::chain(self, &record)?;
        Ok(DerivativeRebasePlan { edit, record })
    }
    /// Cold recovery loads registered intent, then recaptures saved state and
    /// uses the same before/desired/third-state rules and participant owner.
    pub fn recover_derivative_rebase(
        self: &Arc<Self>,
        capture: &AnalysisCapture,
        id: &str,
        base: &Project,
        local: &Project,
        output: &Project,
    ) -> Result<DerivativeRebasePlan, EditError> {
        let record = dj::load(self, id)?;
        dj::chain(self, &record)?;
        selected_output(self, output)?;
        recapture(&record, base, local, output)?;
        if result_state(&record, output)? == DerivativeEffect::Unknown {
            return Err(EditError::ThirdState);
        }
        let intent = dj::validate(self, &record)?;
        let actual = journal::load(self, &intent.id)?.unwrap_or(intent);
        let edit = self.prepare_recovery_record(capture, actual)?;
        Ok(DerivativeRebasePlan { edit, record })
    }
    pub fn inspect_derivative_rebase(
        &self,
        id: &str,
        output: &Project,
    ) -> Result<DerivativeOutcome, EditError> {
        let record = dj::load(self, id)?;
        dj::chain(self, &record)?;
        if selected_output(self, output)? != record.result_root {
            return Err(EditError::AuthorityRefused);
        }
        let intent = dj::validate(self, &record)?;
        let actual = journal::load(self, &intent.id)?;
        let row = &intent.files[0];
        let saved = self
            .host
            .project
            .read_file_snapshot_bounded(&row.path, self.host.limits.resource_bytes)
            .map_err(|_| EditError::Unavailable)?;
        let authored = match saved {
            Some(s)
                if s.unix_mode == row.unix_mode
                    && s.bytes == row.before
                    && s.identity.observation_correlation_token() == row.before_identity =>
            {
                DerivativeEffect::Pending
            }
            Some(s)
                if s.unix_mode == row.unix_mode
                    && s.bytes == row.after
                    && actual.as_ref().is_some_and(|j| {
                        j.files[0]
                            .after_identity
                            .as_ref()
                            .is_none_or(|id| id == &s.identity.observation_correlation_token())
                    }) =>
            {
                DerivativeEffect::Completed
            }
            _ => DerivativeEffect::Unknown,
        };
        let result = result_state(&record, output)?;
        let reason = (authored == DerivativeEffect::Unknown || result == DerivativeEffect::Unknown)
            .then_some(EditError::ThirdState);
        Ok(DerivativeOutcome {
            transaction: id.into(),
            authored,
            result,
            edit: None,
            reason,
            durability_reduced: true,
        })
    }
}
fn result_state(record: &Record, output: &Project) -> Result<DerivativeEffect, EditError> {
    match output
        .read_file_bounded(&record.result_name, dj::CAP)
        .map_err(|_| EditError::ThirdState)?
    {
        None => Ok(DerivativeEffect::Pending),
        Some(b) if b == record.desired_result.as_bytes() => Ok(DerivativeEffect::Completed),
        Some(_) => Ok(DerivativeEffect::Unknown),
    }
}
impl DerivativeRebasePlan {
    pub fn id(&self) -> &str {
        &self.record.id
    }
    pub fn barrier(self, ttl: Duration) -> Result<DerivativeRebaseBarrier, EditError> {
        Ok(DerivativeRebaseBarrier {
            edit: self.edit.barrier(ttl)?,
            record: self.record,
        })
    }
}
impl DerivativeRebaseBarrier {
    pub fn edit_barrier(&self) -> &EditBarrier {
        &self.edit
    }
    pub fn id(&self) -> &str {
        &self.record.id
    }
    /// Errors may leave journal infrastructure; only the returned per-effect
    /// observations describe payload completion. No nested lifecycle lease.
    pub fn apply(
        &self,
        base: &Project,
        local: &Project,
        output: &Project,
    ) -> Result<DerivativeOutcome, EditError> {
        self.edit.ready()?;
        selected_output(&self.edit.plan.authority, output)?;
        recapture(&self.record, base, local, output)?;
        if result_state(&self.record, output)? == DerivativeEffect::Unknown {
            return Err(EditError::ThirdState);
        }
        let journal_reduced = dj::retain(&self.edit.plan.authority, &self.record)?;
        let edit = self.edit.apply()?;
        let authored = if edit.state == EditOutcomeState::Completed {
            DerivativeEffect::Completed
        } else if edit.uncertain.is_empty() && edit.verified.is_empty() {
            DerivativeEffect::Pending
        } else {
            DerivativeEffect::Unknown
        };
        let mut outcome = DerivativeOutcome {
            transaction: self.record.id.clone(),
            authored,
            result: result_state(&self.record, output).unwrap_or(DerivativeEffect::Unknown),
            reason: edit.reason.clone(),
            durability_reduced: edit.durability_reduced || journal_reduced,
            edit: Some(edit),
        };
        if authored != DerivativeEffect::Completed {
            return Ok(outcome);
        }
        // The barrier and lifecycle lease have ended. This separate effect is
        // immutable create-new, never an overwrite of a third-state occupant.
        if let Err(e) = recapture(&self.record, base, local, output) {
            outcome.reason = Some(e);
            return Ok(outcome);
        }
        let current = self
            .edit
            .plan
            .authority
            .inspect_derivative_rebase(&self.record.id, output);
        if !matches!(current,Ok(ref observed) if observed.authored==DerivativeEffect::Completed) {
            outcome.authored = DerivativeEffect::Unknown;
            outcome.reason = Some(EditError::ThirdState);
            return Ok(outcome);
        }
        match result_state(&self.record, output) {
            Ok(DerivativeEffect::Completed) => outcome.result = DerivativeEffect::Completed,
            Ok(DerivativeEffect::Pending) => {
                let dir = match output.root_dir() {
                    Ok(dir) => dir,
                    Err(_) => {
                        outcome.result = DerivativeEffect::Unknown;
                        outcome.reason = Some(EditError::Unavailable);
                        return Ok(outcome);
                    }
                };
                match output.publish_new_in(
                    &dir,
                    &self.record.result_name,
                    self.record.desired_result.as_bytes(),
                ) {
                    Ok(()) => {
                        let durability = dir.sync_directory();
                        outcome.durability_reduced |=
                            durability != vibe_safefs::DirectoryDurability::Synced;
                        if matches!(durability, vibe_safefs::DirectoryDurability::Failed(_)) {
                            outcome.result = DerivativeEffect::Unknown;
                            outcome.reason = Some(EditError::RecoveryRequired);
                            return Ok(outcome);
                        }
                        outcome.result =
                            result_state(&self.record, output).unwrap_or(DerivativeEffect::Unknown);
                    }
                    Err(e) => {
                        outcome.result = if e.stage == vibe_safefs::PublishStage::PossiblyPublished
                        {
                            DerivativeEffect::Unknown
                        } else {
                            DerivativeEffect::Pending
                        };
                        outcome.reason = Some(EditError::RecoveryRequired);
                    }
                }
            }
            _ => {
                outcome.result = DerivativeEffect::Unknown;
                outcome.reason = Some(EditError::ThirdState);
            }
        }
        if outcome.result == DerivativeEffect::Completed {
            // Confirm durability of an exact replay without replacing its inode.
            let confirmed = (|| {
                let store = vibe_safefs::ExternalStore::open_existing_disjoint(
                    output.root_path(),
                    &self.edit.plan.authority.host.project,
                )
                .map_err(|_| EditError::Unavailable)?
                .ok_or(EditError::Unavailable)?;
                let saved = output
                    .read_file_snapshot_bounded(&self.record.result_name, dj::CAP)
                    .map_err(|_| EditError::Unavailable)?
                    .ok_or(EditError::ThirdState)?;
                if saved.bytes != self.record.desired_result.as_bytes() {
                    return Err(EditError::ThirdState);
                }
                store
                    .sync_stable_file_bounded(&self.record.result_name, dj::CAP, &saved)
                    .map_err(|_| EditError::RecoveryRequired)
            })();
            match confirmed {
                Ok(vibe_safefs::DirectoryDurability::Failed(_)) | Err(_) => {
                    outcome.result = DerivativeEffect::Unknown;
                    outcome.reason = Some(EditError::RecoveryRequired);
                }
                Ok(d) => {
                    outcome.durability_reduced |= d != vibe_safefs::DirectoryDurability::Synced
                }
            }
        }
        if outcome.result == DerivativeEffect::Unknown && outcome.reason.is_none() {
            outcome.reason = Some(EditError::RecoveryRequired)
        }
        Ok(outcome)
    }
}
