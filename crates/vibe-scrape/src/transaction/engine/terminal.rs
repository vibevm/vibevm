use super::*;

impl<'a, S, F, V, I> Engine<'a, S, F, V, I>
where
    S: TransactionStore,
    F: TransactionFilesystem,
    V: TransactionVerifier,
    I: FaultInjector,
{
    pub(super) fn finish_verified(
        &mut self,
        journal: &mut Journal,
        cleanup: CleanupPlan<'_>,
    ) -> Result<TransactionReport, TransactionError> {
        if journal.state == TransactionState::Complete {
            let report = journal.report.clone().ok_or_else(|| {
                TransactionError::Store("complete journal has no report".to_owned())
            })?;
            self.retire_transaction(journal)?;
            return Ok(report);
        }
        let gates_event = "all declared final gates accepted".to_owned();
        if !journal.events.contains(&gates_event) {
            journal.events.push(gates_event);
        }
        let mut report = journal
            .report
            .clone()
            .unwrap_or_else(|| self.base_report(journal, Outcome::Verified, Cleanup::Pending));
        report.outcome = Outcome::Verified;
        report.cleanup = Cleanup::Pending;
        report.events = journal.events.clone();
        journal.report = Some(report.clone());
        self.transition(journal, TransactionState::CleanupPending)?;
        self.persist_report(journal, &report)?;
        self.hit(DurableBoundary::ReportPersisted(report.cleanup))?;
        let cleanup_result = match cleanup {
            CleanupPlan::Export(plan) => {
                let candidate = journal.candidate_name.clone().unwrap_or_default();
                let owner = journal.owned_tree_token.clone().unwrap_or_default();
                // A verified export owns the final path as the product, not as
                // cleanup payload. Only a still-present candidate is removed.
                match self.filesystem.observe_export_tree(
                    plan,
                    ExportTreeSlot::Candidate,
                    &candidate,
                    &owner,
                ) {
                    Ok(OwnedTreeObservation::Absent) => Ok(()),
                    Ok(OwnedTreeObservation::Exact(actual)) if actual == plan.final_manifest => {
                        self.refresh_owned_seal(journal, &candidate, &owner)?;
                        self.cleanup_owned_tree(journal, &candidate, &owner)
                    }
                    Ok(OwnedTreeObservation::Third { detail }) => {
                        Err(TransactionError::ThirdState(detail))
                    }
                    Ok(other) => Err(TransactionError::ThirdState(format!(
                        "verified export candidate cleanup saw {other:?}"
                    ))),
                    Err(error) => Err(error),
                }
            }
            CleanupPlan::InPlace(_plan) => {
                let name = journal.quarantine_name.clone().unwrap_or_default();
                let owner = journal.owned_tree_token.clone().unwrap_or_default();
                self.refresh_owned_seal(journal, &name, &owner)?;
                self.cleanup_owned_tree(journal, &name, &owner)
            }
        };
        if let Err(error) = cleanup_result {
            if is_fault(&error) {
                return Err(error);
            }
            let event = format!("cleanup pending: {error}");
            if !journal.events.contains(&event) {
                journal.events.push(event);
            }
            report.events = journal.events.clone();
            journal.report = Some(report.clone());
            self.persist_journal(journal)?;
            self.persist_report(journal, &report)?;
            self.hit(DurableBoundary::ReportPersisted(report.cleanup))?;
            return Ok(report);
        }
        self.hit(DurableBoundary::CleanupCompleted)?;
        report.cleanup = Cleanup::Complete;
        report.events = journal.events.clone();
        journal.report = Some(report.clone());
        self.transition(journal, TransactionState::Complete)?;
        self.persist_report(journal, &report)?;
        self.hit(DurableBoundary::ReportPersisted(report.cleanup))?;
        // A failure here is a benign complete-journal residue; recovery only
        // retires it and never reopens product mutation.
        self.retire_transaction(journal)?;
        Ok(report)
    }

    pub(super) fn finish_rolled_back(
        &mut self,
        journal: &mut Journal,
    ) -> Result<TransactionReport, TransactionError> {
        let report = journal.report.clone().unwrap_or_else(|| {
            let restored = match &journal.execution {
                PreparedMode::Export(plan) => plan.source_tree.digest.clone(),
                PreparedMode::InPlace(plan) => plan.before_tree.digest.clone(),
            };
            self.rolled_back_report(journal, restored)
        });
        self.finish_terminal_without_product(journal, report)
    }

    pub(super) fn finish_complete(
        &mut self,
        journal: &Journal,
    ) -> Result<TransactionReport, TransactionError> {
        let report = journal
            .report
            .clone()
            .ok_or_else(|| TransactionError::Store("complete journal has no report".to_owned()))?;
        self.persist_report(journal, &report)?;
        self.retire_transaction(journal)?;
        Ok(report)
    }

    pub(super) fn finish_rollback_failed(
        &mut self,
        journal: &Journal,
    ) -> Result<TransactionReport, TransactionError> {
        let report = journal.report.clone().ok_or_else(|| {
            TransactionError::Store("rollback-failed journal has no embedded report".to_owned())
        })?;
        self.persist_report(journal, &report)?;
        Err(TransactionError::ThirdState(
            "rollback-failed journal requires operator resolution; embedded report was republished"
                .to_owned(),
        ))
    }

    pub(super) fn refuse_unmutated(
        &mut self,
        journal: &mut Journal,
        detail: String,
    ) -> Result<TransactionReport, TransactionError> {
        let mut report = self.base_report(journal, Outcome::Refused, Cleanup::Complete);
        report.events.push(detail);
        self.finish_terminal_without_product(journal, report)
    }

    pub(super) fn finish_terminal_without_product(
        &mut self,
        journal: &mut Journal,
        report: TransactionReport,
    ) -> Result<TransactionReport, TransactionError> {
        journal.state = TransactionState::Complete;
        journal.report = Some(report.clone());
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::JournalPersisted(
            TransactionState::Complete,
        ))?;
        self.persist_report(journal, &report)?;
        self.hit(DurableBoundary::ReportPersisted(report.cleanup))?;
        self.retire_transaction(journal)?;
        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn verify(
        &mut self,
        journal: &mut Journal,
        phase: VerificationPhase,
        root_kind: VerificationRootKind,
        root: &str,
        expected_tree: &TreeManifest,
        same_display_path_required: bool,
        exemption: Option<&str>,
    ) -> Result<VerificationEvidence, TransactionError> {
        let workspace = self.verification_workspace.clone().ok_or_else(|| {
            TransactionError::Store(
                "transaction has no identity-owned verification workspace".to_owned(),
            )
        })?;
        let context = VerificationContext {
            phase,
            root_kind,
            root_display: root,
            expected_tree,
            same_display_path_required,
            contract_exemption: exemption,
            workspace: &workspace,
        };
        let observed = self.verifier.observe_phase_view(journal, &context)?;
        if observed != *expected_tree {
            return Err(TransactionError::Verification(format!(
                "phase view for {phase:?} does not equal the sealed expected manifest"
            )));
        }
        self.hit(DurableBoundary::PhaseViewProved(phase))?;
        let evidence = self.verifier.execute_verification(journal, context)?;
        journal.verification.push(VerificationRecord {
            phase,
            evidence_sha256: digest(&evidence.canonical_evidence),
            evidence: evidence.clone(),
        });
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::VerificationCompleted(phase))?;
        Ok(evidence)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn reprove_and_record(
        &mut self,
        journal: &mut Journal,
        phase: VerificationPhase,
        root_kind: VerificationRootKind,
        root: &str,
        expected_tree: &TreeManifest,
        delivered: bool,
    ) -> Result<(), TransactionError> {
        let observed = self.verifier.reprove_real_tree(journal, root_kind, root)?;
        if observed != *expected_tree {
            return Err(TransactionError::Verification(
                "real protected tree differs from its sealed expected manifest".to_owned(),
            ));
        }
        self.hit(DurableBoundary::PhaseViewProved(phase))?;
        let canonical_evidence = format!(
            "real-tree-proof/e1\nphase={phase:?}\nroot={root}\ntree={}\n",
            observed.digest.0
        )
        .into_bytes();
        let evidence = VerificationEvidence {
            accepted: true,
            assurance: Assurance::Full,
            summary: format!("real tree proof accepted for {phase:?}"),
            canonical_evidence,
        };
        journal.verification.push(VerificationRecord {
            phase,
            evidence_sha256: digest(&evidence.canonical_evidence),
            evidence,
        });
        if delivered {
            journal.delivered_tree = Some(observed.digest);
        }
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::VerificationCompleted(phase))
    }

    pub(super) fn base_report(
        &self,
        journal: &Journal,
        outcome: Outcome,
        cleanup: Cleanup,
    ) -> TransactionReport {
        let before_tree = match &journal.execution {
            PreparedMode::Export(plan) => Some(plan.source_tree.digest.clone()),
            PreparedMode::InPlace(plan) => Some(plan.before_tree.digest.clone()),
        };
        let after_tree = match (outcome, &journal.execution) {
            (Outcome::Refused, PreparedMode::Export(_)) => None,
            (Outcome::Refused, PreparedMode::InPlace(plan)) => {
                Some(plan.before_tree.digest.clone())
            }
            _ => journal.delivered_tree.clone(),
        };
        TransactionReport {
            project_key: journal.project_key.clone(),
            transaction_id: journal.transaction_id.clone(),
            plan_id: journal.plan_id.clone(),
            mode: journal.mode,
            outcome,
            assurance: if cfg!(windows) && outcome == Outcome::Verified
                || journal
                    .verification
                    .iter()
                    .any(|record| record.evidence.assurance == Assurance::Reduced)
            {
                Assurance::Reduced
            } else {
                Assurance::Full
            },
            cleanup,
            before_tree,
            after_tree,
            snapshots: journal.snapshots.clone(),
            verification: journal.verification.clone(),
            planned_mutations: journal
                .mutation_progress
                .iter()
                .map(|progress| PlannedMutationEvidence {
                    id: progress.id.clone(),
                    kind: progress.kind,
                })
                .collect(),
            actual_mutations: journal.actual_mutations.clone(),
            events: journal.events.clone(),
        }
    }
}
