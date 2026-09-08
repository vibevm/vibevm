use super::*;

impl<'a, S, F, V, I> Engine<'a, S, F, V, I>
where
    S: TransactionStore,
    F: TransactionFilesystem,
    V: TransactionVerifier,
    I: FaultInjector,
{
    pub(super) fn complete_rollback(
        &mut self,
        journal: &mut Journal,
        restored: Digest,
    ) -> Result<TransactionReport, TransactionError> {
        let observed = match self.verifier.reprove_real_tree(
            journal,
            VerificationRootKind::Source,
            &journal.project_display_root,
        ) {
            Ok(observed) if observed.digest == restored => observed,
            Ok(_) => {
                return self.rollback_failed(
                    journal,
                    "restored real tree differs from sealed before tree".to_owned(),
                );
            }
            Err(error) => return self.rollback_failed(journal, error.to_string()),
        };
        journal.delivered_tree = Some(observed.digest);
        self.transition(journal, TransactionState::RolledBack)?;
        let report = self.rolled_back_report(journal, restored);
        self.finish_terminal_without_product(journal, report)
    }

    pub(super) fn rollback_failed<T>(
        &mut self,
        journal: &mut Journal,
        detail: String,
    ) -> Result<T, TransactionError> {
        if !journal.events.contains(&detail) {
            journal.events.push(detail.clone());
        }
        if journal.state.is_pre_verified() && journal.state != TransactionState::RollingBack {
            self.transition(journal, TransactionState::RollingBack)?;
        }
        let (root_kind, root_display) = match &journal.execution {
            PreparedMode::Export(plan) => (
                VerificationRootKind::ExportFinal,
                plan.output_display_path.clone(),
            ),
            PreparedMode::InPlace(_) => (
                VerificationRootKind::Source,
                journal.project_display_root.clone(),
            ),
        };
        journal.delivered_tree = self
            .verifier
            .reprove_real_tree(journal, root_kind, &root_display)
            .ok()
            .map(|tree| tree.digest);
        journal.state = TransactionState::RollbackFailed;
        let mut report = self.base_report(journal, Outcome::RollbackFailed, Cleanup::Pending);
        if !report.events.contains(&detail) {
            report.events.push(detail.clone());
        }
        journal.report = Some(report.clone());
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::JournalPersisted(
            TransactionState::RollbackFailed,
        ))?;
        self.persist_report(journal, &report)?;
        self.hit(DurableBoundary::ReportPersisted(report.cleanup))?;
        Err(TransactionError::ThirdState(detail))
    }

    pub(super) fn rollback_error<T>(
        &mut self,
        journal: &mut Journal,
        error: TransactionError,
    ) -> Result<T, TransactionError> {
        if is_fault(&error) {
            Err(error)
        } else {
            self.rollback_failed(journal, error.to_string())
        }
    }

    pub(super) fn rolled_back_report(
        &self,
        journal: &Journal,
        restored: Digest,
    ) -> TransactionReport {
        let mut report = self.base_report(journal, Outcome::RolledBack, Cleanup::Complete);
        report.after_tree = Some(restored);
        report
            .events
            .push("rollback restored every sealed before state".to_owned());
        report
    }
}
