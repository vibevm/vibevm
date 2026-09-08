use super::*;

impl<'a, S, F, V, I> Engine<'a, S, F, V, I>
where
    S: TransactionStore,
    F: TransactionFilesystem,
    V: TransactionVerifier,
    I: FaultInjector,
{
    pub(super) fn rollback_in_place_with_event(
        &mut self,
        journal: &mut Journal,
        plan: &InPlacePlan,
        event: String,
    ) -> Result<TransactionReport, TransactionError> {
        journal.events.push(event);
        self.rollback_in_place(journal, plan)
    }

    pub(super) fn rollback_in_place(
        &mut self,
        journal: &mut Journal,
        plan: &InPlacePlan,
    ) -> Result<TransactionReport, TransactionError> {
        self.transition(journal, TransactionState::RollingBack)?;
        let Some(quarantine) = journal.quarantine_name.clone() else {
            return self.complete_rollback(journal, plan.before_tree.digest.clone());
        };
        let owner = journal.owned_tree_token.clone().ok_or_else(|| {
            TransactionError::ThirdState(
                "owned quarantine name has no durable ownership token".into(),
            )
        })?;
        if journal.cleanup_wal.is_some() {
            if let Err(error) = self.cleanup_owned_tree(journal, &quarantine, &owner) {
                return self.rollback_error(journal, error);
            }
            self.mark_rolled_back(journal, "in-place/quarantine", 0)?;
            return self.complete_rollback(journal, plan.before_tree.digest.clone());
        }
        let quarantine_owned =
            match self
                .filesystem
                .observe_quarantine_root(plan, &quarantine, &owner)?
            {
                OwnedRootObservation::ExactOwned => {
                    if matches!(
                        progress_status(journal, "in-place/quarantine"),
                        Some(MutationStatus::Planned | MutationStatus::ApplyIntent)
                    ) {
                        infer_actual(
                            journal,
                            "in-place/quarantine",
                            MutationDirection::Apply,
                            self.origin,
                        )?;
                        self.persist_journal(journal)?;
                    }
                    true
                }
                OwnedRootObservation::Absent => {
                    let applied_step = journal.actual_mutations.iter().any(|actual| {
                        actual.direction == MutationDirection::Apply
                            && actual.id != "in-place/quarantine"
                    });
                    match progress_status(journal, "in-place/quarantine") {
                        Some(MutationStatus::RollbackIntent | MutationStatus::RolledBack) => false,
                        Some(MutationStatus::Planned | MutationStatus::ApplyIntent)
                            if !applied_step =>
                        {
                            false
                        }
                        _ => {
                            return self.rollback_failed(
                                journal,
                                "quarantine root is absent despite applied mutation evidence"
                                    .to_owned(),
                            );
                        }
                    }
                }
                OwnedRootObservation::Third { detail } => {
                    return self.rollback_failed(
                        journal,
                        format!("quarantine root is a third state: {detail}"),
                    );
                }
            };
        let mut steps = plan.steps.iter().enumerate().collect::<Vec<_>>();
        if plan.contract_step.kind == MutationKind::ContractDeleteLast {
            steps.push((plan.steps.len(), &plan.contract_step));
        }
        if let Some(cleanup_step) = &plan.contract_cleanup_step {
            steps.push((plan.steps.len() + 1, cleanup_step));
        }
        for (index, step) in steps.into_iter().rev() {
            let observed = match self
                .filesystem
                .observe_step(plan, &quarantine, &owner, step)
            {
                Ok(observed) => observed,
                Err(error) => return self.rollback_error(journal, error),
            };
            match observed {
                SealedObservation::Before => {
                    if progress_status(journal, &step.id) == Some(MutationStatus::ApplyIntent)
                        && let Err(error) = self.filesystem.cleanup_unpublished_step_stage(
                            plan,
                            &quarantine,
                            &owner,
                            step,
                        )
                    {
                        return self.rollback_error(journal, error);
                    }
                    if progress_status(journal, &step.id) == Some(MutationStatus::RollbackIntent) {
                        self.mark_rolled_back(journal, &step.id, index)?;
                    }
                    continue;
                }
                SealedObservation::After => {
                    infer_actual(journal, &step.id, MutationDirection::Apply, self.origin)?;
                    self.persist_journal(journal)?;
                    self.mark_rollback_intent(journal, &step.id, index)?;
                    if let Err(error) =
                        self.filesystem
                            .rollback_step(plan, &quarantine, &owner, step)
                    {
                        return self.rollback_error(journal, error);
                    }
                    self.hit(DurableBoundary::OwnedTreeMutationBeforeReseal {
                        label: format!("rollback-{}", step.id),
                    })?;
                    self.refresh_owned_seal(journal, &quarantine, &owner)?;
                    self.hit(DurableBoundary::MutationCompleted {
                        label: format!("rollback-{}", step.id),
                    })?;
                    let restored =
                        match self
                            .filesystem
                            .observe_step(plan, &quarantine, &owner, step)
                        {
                            Ok(observed) => observed,
                            Err(error) => return self.rollback_error(journal, error),
                        };
                    match restored {
                        SealedObservation::Before => {
                            self.mark_rolled_back(journal, &step.id, index)?;
                        }
                        SealedObservation::After => {
                            return self.rollback_failed(
                                journal,
                                format!("step `{}` remained after rollback", step.id),
                            );
                        }
                        SealedObservation::Third { detail } => {
                            return self.rollback_failed(
                                journal,
                                format!("step `{}` became third state: {detail}", step.id),
                            );
                        }
                    }
                }
                SealedObservation::Third { detail } => {
                    return self.rollback_failed(
                        journal,
                        format!("step `{}` is a third state: {detail}", step.id),
                    );
                }
            }
        }
        if quarantine_owned {
            self.mark_rollback_intent(journal, "in-place/quarantine", 0)?;
            self.refresh_owned_seal(journal, &quarantine, &owner)?;
            if let Err(error) = self.cleanup_owned_tree(journal, &quarantine, &owner) {
                return self.rollback_error(journal, error);
            }
            self.hit(DurableBoundary::MutationCompleted {
                label: "quarantine-remove".to_owned(),
            })?;
            self.mark_rolled_back(journal, "in-place/quarantine", 0)?;
        } else if progress_status(journal, "in-place/quarantine")
            == Some(MutationStatus::RollbackIntent)
        {
            self.mark_rolled_back(journal, "in-place/quarantine", 0)?;
        }
        self.complete_rollback(journal, plan.before_tree.digest.clone())
    }
}
