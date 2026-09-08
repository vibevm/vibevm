use super::*;

impl<'a, S, F, V, I> Engine<'a, S, F, V, I>
where
    S: TransactionStore,
    F: TransactionFilesystem,
    V: TransactionVerifier,
    I: FaultInjector,
{
    pub(super) fn execute_export(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
        snapshots: &[Snapshot],
    ) -> Result<TransactionReport, TransactionError> {
        let before = match self.verify(
            journal,
            VerificationPhase::Before,
            VerificationRootKind::Source,
            &journal.project_display_root.clone(),
            &plan.source_tree,
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
        let candidate = format!(".vibe-scrape-candidate-{}", journal.transaction_id.0);
        let owner = ownership_token(journal, "export");
        journal.candidate_name = Some(candidate.clone());
        journal.owned_tree_token = Some(owner.clone());
        self.persist_same_state(journal, DurableBoundary::CandidateNamePersisted)?;
        self.hit(DurableBoundary::OwnershipPersisted)?;
        self.mark_apply_intent(journal, "export/candidate", 0)?;
        match self
            .filesystem
            .create_export_candidate(plan, &candidate, &owner)
        {
            Ok(ExclusiveTreeCreation::Owned) => {}
            Ok(ExclusiveTreeCreation::NotCreated { detail }) => {
                journal
                    .events
                    .push(format!("candidate not created: {detail}"));
                return self.refuse_unmutated(journal, detail);
            }
            Ok(ExclusiveTreeCreation::CreatedNotReopened { detail }) => {
                journal
                    .events
                    .push(format!("candidate ownership pending: {detail}"));
                self.persist_journal(journal)?;
                return Err(TransactionError::Filesystem(detail));
            }
            Err(error) => return Err(error),
        }
        journal.owned_tree_seal = Some(self.filesystem.owned_tree_seal(&candidate, &owner)?);
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::MutationCompleted {
            label: "export-candidate-create".to_owned(),
        })?;
        self.mark_applied(journal, "export/candidate", 0)?;

        for (index, entry) in plan.entries.iter().enumerate() {
            let progress_id = export_entry_id(index, entry);
            journal.active_step = Some(index);
            set_progress_status(journal, &progress_id, MutationStatus::ApplyIntent)?;
            self.persist_journal(journal)?;
            self.hit(DurableBoundary::StepIntentPersisted {
                index,
                id: progress_id.clone(),
            })?;
            let after = prepared_after_for_export(entry, snapshots)?;
            if let Err(error) = self
                .filesystem
                .apply_export_entry(plan, &candidate, &owner, entry, after)
            {
                if is_fault(&error) {
                    return Err(error);
                }
                return self.rollback_export_with_event(journal, plan, error.to_string());
            }
            self.hit(DurableBoundary::OwnedTreeMutationBeforeReseal {
                label: format!("export-entry-{index}"),
            })?;
            self.refresh_owned_seal(journal, &candidate, &owner)?;
            self.hit(DurableBoundary::MutationCompleted {
                label: format!("export-entry-{index}"),
            })?;
            journal.completed_steps = index + 1;
            journal.active_step = None;
            record_actual(
                journal,
                &progress_id,
                MutationDirection::Apply,
                MutationStatus::Applied,
                self.origin,
            )?;
            set_progress_status(journal, &progress_id, MutationStatus::Applied)?;
            self.persist_journal(journal)?;
            self.hit(DurableBoundary::StepCompletionPersisted {
                index,
                id: progress_id,
            })?;
        }
        self.transition(journal, TransactionState::Candidate)?;
        if let Err(error) = require_exact_tree(
            self.filesystem.observe_export_tree(
                plan,
                ExportTreeSlot::Candidate,
                &candidate,
                &owner,
            )?,
            &plan.final_manifest,
            "candidate before publication",
        ) {
            return self.rollback_failed(journal, error.to_string());
        }
        self.mark_apply_intent(journal, "export/publish", plan.entries.len() + 1)?;
        match self
            .filesystem
            .publish_export_noreplace(plan, &candidate, &owner)
        {
            Ok(()) => {}
            Err(TransactionError::OutputRace(detail)) => {
                return self.refuse_export_race(journal, plan, detail);
            }
            Err(TransactionError::AtomicNoReplaceUnsupported) => {
                return self.refuse_export_race(
                    journal,
                    plan,
                    "platform cannot guarantee no-replace publication".to_owned(),
                );
            }
            Err(error) if is_fault(&error) => return Err(error),
            Err(error) => {
                return self.rollback_export_with_event(journal, plan, error.to_string());
            }
        }
        self.hit(DurableBoundary::OwnedTreeMutationBeforeReseal {
            label: "export-publish".to_owned(),
        })?;
        self.refresh_owned_seal(journal, &candidate, &owner)?;
        self.hit(DurableBoundary::MutationCompleted {
            label: "export-publish".to_owned(),
        })?;
        self.mark_applied(journal, "export/publish", plan.entries.len() + 1)?;
        self.transition(journal, TransactionState::PublishedPendingVerify)?;
        if let Err(error) = require_exact_tree(
            self.filesystem.observe_export_tree(
                plan,
                ExportTreeSlot::Output,
                &candidate,
                &owner,
            )?,
            &plan.final_manifest,
            "published output",
        ) {
            return self.rollback_failed(journal, error.to_string());
        }
        for phase in [
            VerificationPhase::FinalResidual,
            VerificationPhase::AfterHealth,
        ] {
            match self.verify(
                journal,
                phase,
                VerificationRootKind::ExportFinal,
                &plan.output_display_path,
                &plan.final_manifest,
                plan.after_same_display_path,
                None,
            ) {
                Ok(evidence) if evidence.accepted => {}
                Ok(evidence) => {
                    return self.rollback_export_with_event(journal, plan, evidence.summary);
                }
                Err(error) if is_fault(&error) => return Err(error),
                Err(error) => {
                    return self.rollback_export_with_event(journal, plan, error.to_string());
                }
            }
        }
        if let Err(error) = self.reprove_and_record(
            journal,
            VerificationPhase::FinalTree,
            VerificationRootKind::ExportFinal,
            &plan.output_display_path,
            &plan.final_manifest,
            true,
        ) {
            if is_fault(&error) {
                return Err(error);
            }
            return self.rollback_export_with_event(journal, plan, error.to_string());
        }
        if let Err(error) = self.reprove_and_record(
            journal,
            VerificationPhase::SourceUnchanged,
            VerificationRootKind::Source,
            &journal.project_display_root.clone(),
            &plan.source_tree,
            false,
        ) {
            if is_fault(&error) {
                return Err(error);
            }
            return self.rollback_export_with_event(journal, plan, error.to_string());
        }
        self.transition(journal, TransactionState::Verified)?;
        self.finish_verified(journal, CleanupPlan::Export(plan))
    }

    pub(super) fn rollback_export_with_event(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
        event: String,
    ) -> Result<TransactionReport, TransactionError> {
        journal.events.push(event);
        self.rollback_export(journal, plan)
    }

    pub(super) fn rollback_export(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
    ) -> Result<TransactionReport, TransactionError> {
        self.transition(journal, TransactionState::RollingBack)?;
        let Some(candidate) = journal.candidate_name.clone() else {
            return self.complete_rollback(journal, plan.source_tree.digest.clone());
        };
        let owner = journal.owned_tree_token.clone().ok_or_else(|| {
            TransactionError::ThirdState("owned export name has no durable ownership token".into())
        })?;
        if journal.cleanup_wal.is_some() {
            let count = journal
                .owned_tree_seal
                .as_ref()
                .map_or(0, |seal| seal.entries.len());
            if let Err(error) = self.cleanup_owned_tree(journal, &candidate, &owner) {
                return self.rollback_error(journal, error);
            }
            self.mark_export_count_rolled_back(journal, plan, count)?;
            return if journal.settlement_intent == Some(Outcome::Refused) {
                let report = self.base_report(journal, Outcome::Refused, Cleanup::Complete);
                self.finish_terminal_without_product(journal, report)
            } else {
                self.complete_rollback(journal, plan.source_tree.digest.clone())
            };
        }
        let expected = expected_export_manifests(journal, plan);
        let candidate_observed = match self.filesystem.observe_export_tree(
            plan,
            ExportTreeSlot::Candidate,
            &candidate,
            &owner,
        ) {
            Ok(observed) => observed,
            Err(error) => return self.rollback_error(journal, error),
        };
        let output_observed = match self.filesystem.observe_export_tree(
            plan,
            ExportTreeSlot::Output,
            &candidate,
            &owner,
        ) {
            Ok(observed) => observed,
            Err(error) => return self.rollback_error(journal, error),
        };
        match (candidate_observed, output_observed) {
            (OwnedTreeObservation::Absent, OwnedTreeObservation::Absent) => {
                if !both_absent_is_explained(journal) {
                    return self.rollback_failed(
                        journal,
                        "candidate and output are both absent without pre-create or durable rollback evidence"
                            .to_owned(),
                    );
                }
                self.mark_absent_export_rollbacks(journal)?;
            }
            (OwnedTreeObservation::Exact(actual), OwnedTreeObservation::Absent)
                if expected.contains(&actual) =>
            {
                infer_export_apply_progress(journal, plan, &actual, self.origin)?;
                self.persist_journal(journal)?;
                for (index, entry) in plan
                    .entries
                    .iter()
                    .enumerate()
                    .take(actual.entries.len())
                    .rev()
                {
                    self.mark_rollback_intent(journal, &export_entry_id(index, entry), index)?;
                }
                self.mark_rollback_intent(journal, "export/candidate", 0)?;
                self.refresh_owned_seal(journal, &candidate, &owner)?;
                if let Err(error) = self.cleanup_owned_tree(journal, &candidate, &owner) {
                    return self.rollback_error(journal, error);
                }
                self.hit(DurableBoundary::MutationCompleted {
                    label: "export-candidate-remove".to_owned(),
                })?;
                self.mark_export_tree_rolled_back(journal, plan, &actual)?;
            }
            (OwnedTreeObservation::Absent, OwnedTreeObservation::Exact(actual))
                if actual == plan.final_manifest =>
            {
                infer_export_apply_progress(journal, plan, &actual, self.origin)?;
                infer_actual(
                    journal,
                    "export/publish",
                    MutationDirection::Apply,
                    self.origin,
                )?;
                self.persist_journal(journal)?;
                self.mark_rollback_intent(journal, "export/publish", plan.entries.len() + 1)?;
                if let Err(error) = self.filesystem.unpublish_export(plan, &candidate, &owner) {
                    return self.rollback_error(journal, error);
                }
                self.hit(DurableBoundary::OwnedTreeMutationBeforeReseal {
                    label: "export-unpublish".to_owned(),
                })?;
                self.refresh_owned_seal(journal, &candidate, &owner)?;
                self.hit(DurableBoundary::MutationCompleted {
                    label: "export-unpublish".to_owned(),
                })?;
                self.mark_rolled_back(journal, "export/publish", plan.entries.len() + 1)?;
                if let Err(error) = require_exact_tree(
                    self.filesystem.observe_export_tree(
                        plan,
                        ExportTreeSlot::Candidate,
                        &candidate,
                        &owner,
                    )?,
                    &plan.final_manifest,
                    "rolled-back export candidate",
                ) {
                    return self.rollback_error(journal, error);
                }
                for (index, entry) in plan.entries.iter().enumerate().rev() {
                    self.mark_rollback_intent(journal, &export_entry_id(index, entry), index)?;
                }
                self.mark_rollback_intent(journal, "export/candidate", 0)?;
                self.refresh_owned_seal(journal, &candidate, &owner)?;
                if let Err(error) = self.cleanup_owned_tree(journal, &candidate, &owner) {
                    return self.rollback_error(journal, error);
                }
                self.hit(DurableBoundary::MutationCompleted {
                    label: "export-candidate-remove".to_owned(),
                })?;
                self.mark_export_tree_rolled_back(journal, plan, &plan.final_manifest)?;
            }
            (OwnedTreeObservation::Third { detail }, _)
            | (_, OwnedTreeObservation::Third { detail }) => {
                return self.rollback_failed(journal, format!("export descendant set: {detail}"));
            }
            (left, right) => {
                return self.rollback_failed(
                    journal,
                    format!("export ownership is ambiguous: candidate={left:?}, output={right:?}"),
                );
            }
        }
        if journal.settlement_intent == Some(Outcome::Refused) {
            let report = self.base_report(journal, Outcome::Refused, Cleanup::Complete);
            self.finish_terminal_without_product(journal, report)
        } else {
            self.complete_rollback(journal, plan.source_tree.digest.clone())
        }
    }

    pub(super) fn refuse_export_race(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
        detail: String,
    ) -> Result<TransactionReport, TransactionError> {
        let candidate = journal.candidate_name.clone().ok_or_else(|| {
            TransactionError::Store("output race journal has no candidate name".to_owned())
        })?;
        let owner = journal.owned_tree_token.clone().ok_or_else(|| {
            TransactionError::Store("output race journal has no ownership token".to_owned())
        })?;
        journal.settlement_intent = Some(Outcome::Refused);
        journal
            .events
            .push(format!("output publication refused: {detail}"));
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::RefusalIntentPersisted)?;
        let observed = match self.filesystem.observe_export_tree(
            plan,
            ExportTreeSlot::Candidate,
            &candidate,
            &owner,
        ) {
            Ok(observed) => observed,
            Err(error) => return self.rollback_error(journal, error),
        };
        if let Err(error) = require_exact_tree(
            observed,
            &plan.final_manifest,
            "candidate after output race",
        ) {
            return self.rollback_failed(journal, error.to_string());
        }
        for (index, entry) in plan.entries.iter().enumerate().rev() {
            self.mark_rollback_intent(journal, &export_entry_id(index, entry), index)?;
        }
        self.mark_rollback_intent(journal, "export/candidate", 0)?;
        self.refresh_owned_seal(journal, &candidate, &owner)?;
        if let Err(error) = self.cleanup_owned_tree(journal, &candidate, &owner) {
            return self.rollback_error(journal, error);
        }
        self.hit(DurableBoundary::MutationCompleted {
            label: "export-candidate-remove".to_owned(),
        })?;
        self.mark_export_tree_rolled_back(journal, plan, &plan.final_manifest)?;
        let report = self.base_report(journal, Outcome::Refused, Cleanup::Complete);
        self.finish_terminal_without_product(journal, report)
    }
}
