use super::*;

impl<'a, S, F, V, I> Engine<'a, S, F, V, I>
where
    S: TransactionStore,
    F: TransactionFilesystem,
    V: TransactionVerifier,
    I: FaultInjector,
{
    pub(super) fn execute_in_place(
        &mut self,
        journal: &mut Journal,
        plan: &InPlacePlan,
        snapshots: &[Snapshot],
    ) -> Result<TransactionReport, TransactionError> {
        let before = match self.verify(
            journal,
            VerificationPhase::Before,
            VerificationRootKind::Source,
            &journal.project_display_root.clone(),
            &plan.before_tree,
            plan.before_same_display_path,
            None,
        ) {
            Ok(evidence) => evidence,
            Err(error) if is_fault(&error) => return Err(error),
            Err(error) => return self.refuse_unmutated(journal, error.to_string()),
        };
        if !before.accepted {
            return self.refuse_unmutated(journal, before.summary);
        }
        self.transition(journal, TransactionState::BeforePassed)?;
        let quarantine = format!(".vibe-scrape-quarantine-{}", journal.transaction_id.0);
        let owner = ownership_token(journal, "quarantine");
        journal.quarantine_name = Some(quarantine.clone());
        journal.owned_tree_token = Some(owner.clone());
        self.persist_same_state(journal, DurableBoundary::QuarantineNamePersisted)?;
        self.hit(DurableBoundary::OwnershipPersisted)?;
        self.mark_apply_intent(journal, "in-place/quarantine", 0)?;
        match self
            .filesystem
            .create_quarantine(plan, &quarantine, &owner)?
        {
            ExclusiveTreeCreation::Owned => {}
            ExclusiveTreeCreation::NotCreated { detail } => {
                journal
                    .events
                    .push(format!("quarantine not created: {detail}"));
                return self.refuse_unmutated(journal, detail);
            }
            ExclusiveTreeCreation::CreatedNotReopened { detail } => {
                journal
                    .events
                    .push(format!("quarantine ownership pending: {detail}"));
                self.persist_journal(journal)?;
                return Err(TransactionError::Filesystem(detail));
            }
        }
        journal.owned_tree_seal = Some(self.filesystem.owned_tree_seal(&quarantine, &owner)?);
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::MutationCompleted {
            label: "quarantine-create".to_owned(),
        })?;
        self.mark_applied(journal, "in-place/quarantine", 0)?;
        self.transition(journal, TransactionState::Mutating)?;

        for (index, step) in plan.steps.iter().enumerate() {
            if let Err(error) =
                self.apply_in_place_step(journal, plan, step, index, &quarantine, &owner, snapshots)
            {
                if is_fault(&error) {
                    return Err(error);
                }
                return self.rollback_in_place_with_event(journal, plan, error.to_string());
            }
        }
        let exemption = match plan.contract_step.kind {
            MutationKind::ContractDeleteLast => plan
                .contract_step
                .transitions
                .iter()
                .find(|transition| transition.location == Location::Project)
                .map(|transition| transition.path.as_str()),
            MutationKind::ContractExternalPreserve => None,
            _ => unreachable!("validated contract step"),
        };
        match self.verify(
            journal,
            VerificationPhase::PreContractResidual,
            VerificationRootKind::InPlaceView,
            &journal.project_display_root.clone(),
            &plan.pre_contract_tree,
            plan.after_same_display_path,
            exemption,
        ) {
            Ok(evidence) if evidence.accepted => {}
            Ok(evidence) => {
                return self.rollback_in_place_with_event(journal, plan, evidence.summary);
            }
            Err(error) if is_fault(&error) => return Err(error),
            Err(error) => {
                return self.rollback_in_place_with_event(journal, plan, error.to_string());
            }
        }
        let contract_index = plan.steps.len();
        if plan.contract_step.kind == MutationKind::ContractExternalPreserve {
            set_progress_status(journal, &plan.contract_step.id, MutationStatus::NoMutation)?;
            journal.completed_steps = contract_index + 1;
            self.persist_journal(journal)?;
            self.hit(DurableBoundary::StepCompletionPersisted {
                index: contract_index,
                id: plan.contract_step.id.clone(),
            })?;
        } else if let Err(error) = self.apply_in_place_step(
            journal,
            plan,
            &plan.contract_step,
            contract_index,
            &quarantine,
            &owner,
            snapshots,
        ) {
            if is_fault(&error) {
                return Err(error);
            }
            return self.rollback_in_place_with_event(journal, plan, error.to_string());
        }
        let boundary = match plan.contract_step.kind {
            MutationKind::ContractDeleteLast => ContractBoundaryAction::DeleteLastMoved,
            MutationKind::ContractExternalPreserve => ContractBoundaryAction::ExternalPreserved,
            _ => unreachable!("validated contract step"),
        };
        self.transition(journal, TransactionState::ContractBoundary(boundary))?;
        if let Some(cleanup_step) = &plan.contract_cleanup_step {
            let cleanup_index = contract_index + 1;
            if let Err(error) = self.apply_in_place_step(
                journal,
                plan,
                cleanup_step,
                cleanup_index,
                &quarantine,
                &owner,
                snapshots,
            ) {
                if is_fault(&error) {
                    return Err(error);
                }
                return self.rollback_in_place_with_event(journal, plan, error.to_string());
            }
        }
        for phase in [
            VerificationPhase::FinalResidual,
            VerificationPhase::AfterHealth,
        ] {
            match self.verify(
                journal,
                phase,
                VerificationRootKind::InPlaceView,
                &journal.project_display_root.clone(),
                &plan.after_tree,
                plan.after_same_display_path,
                None,
            ) {
                Ok(evidence) if evidence.accepted => {}
                Ok(evidence) => {
                    return self.rollback_in_place_with_event(journal, plan, evidence.summary);
                }
                Err(error) if is_fault(&error) => return Err(error),
                Err(error) => {
                    return self.rollback_in_place_with_event(journal, plan, error.to_string());
                }
            }
        }
        if let Err(error) = self.reprove_and_record(
            journal,
            VerificationPhase::FinalTree,
            VerificationRootKind::InPlaceView,
            &journal.project_display_root.clone(),
            &plan.after_tree,
            true,
        ) {
            if is_fault(&error) {
                return Err(error);
            }
            return self.rollback_in_place_with_event(journal, plan, error.to_string());
        }
        self.transition(journal, TransactionState::Verified)?;
        self.finish_verified(journal, CleanupPlan::InPlace(plan))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_in_place_step(
        &mut self,
        journal: &mut Journal,
        plan: &InPlacePlan,
        step: &MutationStep,
        index: usize,
        quarantine: &str,
        owner: &str,
        snapshots: &[Snapshot],
    ) -> Result<(), TransactionError> {
        journal.active_step = Some(index);
        set_progress_status(journal, &step.id, MutationStatus::ApplyIntent)?;
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::StepIntentPersisted {
            index,
            id: step.id.clone(),
        })?;
        match self
            .filesystem
            .observe_step(plan, quarantine, owner, step)?
        {
            SealedObservation::Before => {}
            SealedObservation::After => {
                // A retry after a post-mutation crash. Do not execute twice.
            }
            SealedObservation::Third { detail } => {
                return Err(TransactionError::ThirdState(format!(
                    "step `{}` before apply: {detail}",
                    step.id
                )));
            }
        }
        if self
            .filesystem
            .observe_step(plan, quarantine, owner, step)?
            == SealedObservation::Before
        {
            let after = prepared_after_for_step(step, snapshots)?;
            self.filesystem
                .apply_step(plan, quarantine, owner, step, after)?;
            self.hit(DurableBoundary::OwnedTreeMutationBeforeReseal {
                label: format!("in-place-step-{index}"),
            })?;
            self.refresh_owned_seal(journal, quarantine, owner)?;
            self.hit(DurableBoundary::MutationCompleted {
                label: format!("in-place-step-{index}"),
            })?;
        }
        match self
            .filesystem
            .observe_step(plan, quarantine, owner, step)?
        {
            SealedObservation::After => {}
            SealedObservation::Before => {
                return Err(TransactionError::Filesystem(format!(
                    "step `{}` reported success but remained in before state",
                    step.id
                )));
            }
            SealedObservation::Third { detail } => {
                return Err(TransactionError::ThirdState(format!(
                    "step `{}` after apply: {detail}",
                    step.id
                )));
            }
        }
        journal.completed_steps = index + 1;
        journal.active_step = None;
        record_actual(
            journal,
            &step.id,
            MutationDirection::Apply,
            MutationStatus::Applied,
            self.origin,
        )?;
        set_progress_status(journal, &step.id, MutationStatus::Applied)?;
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::StepCompletionPersisted {
            index,
            id: step.id.clone(),
        })
    }
}
