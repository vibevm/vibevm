use super::*;

impl<'a, S, F, V, I> Engine<'a, S, F, V, I>
where
    S: TransactionStore,
    F: TransactionFilesystem,
    V: TransactionVerifier,
    I: FaultInjector,
{
    pub(super) fn transition(
        &mut self,
        journal: &mut Journal,
        next: TransactionState,
    ) -> Result<(), TransactionError> {
        validate_transition(journal.mode, &journal.state, &next)?;
        journal.state = next.clone();
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::JournalPersisted(next))
    }

    pub(super) fn persist_same_state(
        &mut self,
        journal: &mut Journal,
        boundary: DurableBoundary,
    ) -> Result<(), TransactionError> {
        self.persist_journal(journal)?;
        self.hit(boundary)
    }

    pub(super) fn hit(&mut self, boundary: DurableBoundary) -> Result<(), TransactionError> {
        self.faults.boundary(boundary)
    }

    pub(super) fn persist_journal(
        &mut self,
        journal: &mut Journal,
    ) -> Result<(), TransactionError> {
        let previous = journal.revision;
        journal.revision = previous
            .checked_add(1)
            .ok_or_else(|| TransactionError::Store("journal revision overflow".to_owned()))?;
        match self.store.persist_journal(journal) {
            Ok(()) => Ok(()),
            Err(error) => {
                // The store accepts a same-revision retry only when its bytes
                // are identical, covering both pre-write and post-write error
                // reports without skipping a generation.
                journal.revision = previous;
                Err(error)
            }
        }
    }

    pub(super) fn persist_report(
        &mut self,
        journal: &Journal,
        report: &TransactionReport,
    ) -> Result<(), TransactionError> {
        let plan: vibe_wire::generated::scrape::e1::plan::Plan =
            serde_json::from_slice(&journal.canonical_plan).map_err(|error| {
                TransactionError::Store(format!(
                    "embedded canonical plan failed strict generated decode: {error}"
                ))
            })?;
        let wire = super::report::report_to_wire_plan(report, &plan)?;
        let canonical = serde_json::to_vec(&wire).map_err(|error| {
            TransactionError::Store(format!("canonical report serialization failed: {error}"))
        })?;
        self.store.persist_report(report, &canonical)
    }

    pub(super) fn retire_transaction(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.verifier.release_verification_workspace();
        self.store.retire_transaction(journal)
    }

    pub(super) fn mark_apply_intent(
        &mut self,
        journal: &mut Journal,
        id: &str,
        index: usize,
    ) -> Result<(), TransactionError> {
        set_progress_status(journal, id, MutationStatus::ApplyIntent)?;
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::StepIntentPersisted {
            index,
            id: id.to_owned(),
        })
    }

    pub(super) fn refresh_owned_seal(
        &mut self,
        journal: &mut Journal,
        name: &str,
        owner: &str,
    ) -> Result<(), TransactionError> {
        journal.owned_tree_seal = Some(self.filesystem.owned_tree_seal(name, owner)?);
        self.persist_journal(journal)
    }

    pub(super) fn cleanup_owned_tree(
        &mut self,
        journal: &mut Journal,
        name: &str,
        owner: &str,
    ) -> Result<(), TransactionError> {
        if journal.cleanup_wal.is_none() {
            let seal = journal.owned_tree_seal.as_ref().ok_or_else(|| {
                TransactionError::ThirdState(
                    "owned-tree cleanup has no durable identity/manifest seal".to_owned(),
                )
            })?;
            journal.cleanup_wal = Some(OwnedTreeCleanupWal {
                name: name.to_owned(),
                directory_identity: seal.directory_identity.clone(),
                manifest_digest: seal.manifest_digest.clone(),
                completed: Vec::new(),
                active: None,
            });
            self.persist_journal(journal)?;
            self.hit(DurableBoundary::CleanupStarted)?;
        }

        loop {
            let wal = journal.cleanup_wal.as_ref().ok_or_else(|| {
                TransactionError::Store("cleanup WAL disappeared before completion".to_owned())
            })?;
            let seal = journal.owned_tree_seal.clone().ok_or_else(|| {
                TransactionError::Store("cleanup WAL has no owned-tree seal".to_owned())
            })?;
            if wal.name != name
                || wal.directory_identity != seal.directory_identity
                || wal.manifest_digest != seal.manifest_digest
            {
                return Err(TransactionError::Store(
                    "cleanup WAL binding differs from the requested owned tree".to_owned(),
                ));
            }

            if wal.active.is_none() {
                let completed = wal.completed.clone();
                match self
                    .filesystem
                    .prepare_owned_tree_cleanup(journal, name, owner, &seal, &completed)?
                {
                    OwnedTreeCleanupPreparation::Complete => {
                        // Keep the completed manifest-bound WAL evidence in
                        // the terminal journal until that journal itself is
                        // retired. The final root completion was already
                        // persisted; clearing the only durable explanation
                        // before a fault here would make root absence
                        // ambiguous on recovery.
                        self.hit(DurableBoundary::CleanupCompleted)?;
                        return Ok(());
                    }
                    OwnedTreeCleanupPreparation::Intent(intent) => {
                        let progress_key = intent.progress_key.clone();
                        journal
                            .cleanup_wal
                            .as_mut()
                            .expect("cleanup WAL was checked")
                            .active = Some(intent);
                        self.persist_journal(journal)?;
                        self.hit(DurableBoundary::CleanupIntentPersisted { progress_key })?;
                    }
                }
            }

            let wal = journal.cleanup_wal.as_ref().expect("cleanup WAL is live");
            let completed = wal.completed.clone();
            let intent = wal.active.clone().ok_or_else(|| {
                TransactionError::Store("cleanup intent disappeared before execution".to_owned())
            })?;
            let completion = self
                .filesystem
                .execute_owned_tree_cleanup(journal, name, owner, &seal, &completed, &intent)?;
            if completion.progress_key != intent.progress_key {
                return Err(TransactionError::Store(
                    "cleanup completion differs from its durable intent".to_owned(),
                ));
            }
            let progress_key = completion.progress_key;
            self.hit(DurableBoundary::CleanupMutationCompleted {
                progress_key: progress_key.clone(),
            })?;
            let wal = journal.cleanup_wal.as_mut().expect("cleanup WAL is live");
            wal.completed.push(progress_key.clone());
            wal.active = None;
            if completion.recovered_after_syscall {
                journal.events.push(format!(
                    "cleanup recovered completed syscall for `{progress_key}`"
                ));
            }
            self.persist_journal(journal)?;
            self.hit(DurableBoundary::CleanupStepCompletionPersisted { progress_key })?;
        }
    }

    pub(super) fn mark_applied(
        &mut self,
        journal: &mut Journal,
        id: &str,
        index: usize,
    ) -> Result<(), TransactionError> {
        set_progress_status(journal, id, MutationStatus::Applied)?;
        record_actual(
            journal,
            id,
            MutationDirection::Apply,
            MutationStatus::Applied,
            self.origin,
        )?;
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::StepCompletionPersisted {
            index,
            id: id.to_owned(),
        })
    }

    pub(super) fn mark_rollback_intent(
        &mut self,
        journal: &mut Journal,
        id: &str,
        index: usize,
    ) -> Result<(), TransactionError> {
        set_progress_status(journal, id, MutationStatus::RollbackIntent)?;
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::StepRollbackIntentPersisted {
            index,
            id: id.to_owned(),
        })
    }

    pub(super) fn mark_rolled_back(
        &mut self,
        journal: &mut Journal,
        id: &str,
        index: usize,
    ) -> Result<(), TransactionError> {
        set_progress_status(journal, id, MutationStatus::RolledBack)?;
        record_actual(
            journal,
            id,
            MutationDirection::Rollback,
            MutationStatus::RolledBack,
            self.origin,
        )?;
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::StepRollbackCompletionPersisted {
            index,
            id: id.to_owned(),
        })
    }

    pub(super) fn mark_export_tree_rolled_back(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
        actual: &TreeManifest,
    ) -> Result<(), TransactionError> {
        self.mark_export_count_rolled_back(journal, plan, actual.entries.len())
    }

    pub(super) fn mark_export_count_rolled_back(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
        count: usize,
    ) -> Result<(), TransactionError> {
        for (index, entry) in plan.entries.iter().enumerate().take(count).rev() {
            self.mark_rolled_back(journal, &export_entry_id(index, entry), index)?;
        }
        self.mark_rolled_back(journal, "export/candidate", 0)
    }

    pub(super) fn mark_absent_export_rollbacks(
        &mut self,
        journal: &mut Journal,
    ) -> Result<(), TransactionError> {
        let applied = journal
            .actual_mutations
            .iter()
            .filter(|actual| {
                actual.direction == MutationDirection::Apply
                    && !journal.actual_mutations.iter().any(|other| {
                        other.id == actual.id && other.direction == MutationDirection::Rollback
                    })
            })
            .map(|actual| actual.id.clone())
            .collect::<Vec<_>>();
        for (index, id) in applied.into_iter().enumerate() {
            self.mark_rolled_back(journal, &id, index)?;
        }
        Ok(())
    }
}
