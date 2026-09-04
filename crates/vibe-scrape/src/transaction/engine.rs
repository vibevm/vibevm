//! Journal-first export and in-place state machines.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use super::model::*;
use super::sha256::{digest, project_key};
use super::traits::*;
use super::validate;

pub struct Engine<'a, S, F, V, I> {
    store: &'a mut S,
    filesystem: &'a mut F,
    verifier: &'a mut V,
    faults: &'a mut I,
    origin: MutationOrigin,
    verification_workspace: Option<VerificationWorkspace>,
}

impl<'a, S, F, V, I> Engine<'a, S, F, V, I>
where
    S: TransactionStore,
    F: TransactionFilesystem,
    V: TransactionVerifier,
    I: FaultInjector,
{
    pub fn new(
        store: &'a mut S,
        filesystem: &'a mut F,
        verifier: &'a mut V,
        faults: &'a mut I,
    ) -> Self {
        Self {
            store,
            filesystem,
            verifier,
            faults,
            origin: MutationOrigin::Execution,
            verification_workspace: None,
        }
    }

    /// Acquire the external project lock and settle the pending-journal gate
    /// before invoking `prepare`. The closure is the only place allowed to
    /// load the source contract or build a fresh plan.
    pub fn execute_locked<P, Prepare>(
        &mut self,
        project_identity_token: &str,
        project_display_root: &str,
        prepare: Prepare,
    ) -> Result<TransactionReport, TransactionError>
    where
        P: PreparedScrapeSource,
        Prepare: FnOnce() -> Result<P, TransactionError>,
    {
        self.store.prove_outside_project(project_display_root)?;
        self.hit(DurableBoundary::StoreProvedExternal)?;
        let key = project_key(project_identity_token);
        let _lock = self.store.lock_project(&key)?;
        self.hit(DurableBoundary::ProjectLockAcquired)?;
        if self.store.pending(&key)?.is_some() {
            return Err(TransactionError::Store(
                "a pending scrape transaction must be recovered before loading a contract"
                    .to_owned(),
            ));
        }
        let prepared = prepare()?.into_transaction()?;
        validate::prepared(&prepared)?;
        if prepared.project_identity_token != project_identity_token
            || prepared.project_display_root != project_display_root
        {
            return Err(TransactionError::InvalidPrepared(
                "locked project identity/root differs from prepared scrape".to_owned(),
            ));
        }
        self.execute_prepared(key, prepared)
    }

    /// Execute after the caller acquired the store lock, proved `pending ==
    /// None`, and only then prepared the transaction. This enables adapters
    /// whose verifier/filesystem instances depend on the prepared value.
    pub fn execute_under_held_gate<P: PreparedScrapeSource>(
        &mut self,
        key: ProjectKey,
        project_identity_token: &str,
        project_display_root: &str,
        source: P,
    ) -> Result<TransactionReport, TransactionError> {
        let prepared = source.into_transaction()?;
        validate::prepared(&prepared)?;
        if prepared.project_identity_token != project_identity_token
            || prepared.project_display_root != project_display_root
        {
            return Err(TransactionError::InvalidPrepared(
                "held-gate identity/root differs from prepared scrape".to_owned(),
            ));
        }
        if key != project_key(project_identity_token) {
            return Err(TransactionError::InvalidPrepared(
                "held-gate project key differs from prepared identity".to_owned(),
            ));
        }
        self.execute_prepared(key, prepared)
    }

    fn execute_prepared(
        &mut self,
        key: ProjectKey,
        prepared: PreparedTransaction,
    ) -> Result<TransactionReport, TransactionError> {
        let transaction_id = self.store.mint_transaction_id(&key)?;
        validate_transaction_id(&transaction_id)?;
        let records = prepared
            .snapshots
            .iter()
            .map(|snapshot| SnapshotRecord {
                kind: snapshot.kind,
                name: snapshot.name.clone(),
                sha256: digest(&snapshot.bytes),
                bytes: snapshot.bytes.len() as u64,
                mode: snapshot.mode,
            })
            .collect::<Vec<_>>();
        let mode = prepared.mode();
        let execution = prepared.mode.clone();
        let mut journal = Journal {
            schema: 1,
            revision: 0,
            project_key: key,
            transaction_id,
            mode,
            plan_id: prepared.plan_id,
            canonical_plan: prepared.canonical_plan,
            verification_workspace: None,
            project_display_root: prepared.project_display_root,
            execution,
            state: TransactionState::Preparing,
            snapshots: records,
            snapshots_persisted: 0,
            snapshot_active: None,
            candidate_name: None,
            quarantine_name: None,
            owned_tree_token: None,
            owned_tree_seal: None,
            cleanup_wal: None,
            completed_steps: 0,
            active_step: None,
            mutation_progress: planned_mutations(&prepared.mode),
            actual_mutations: Vec::new(),
            settlement_intent: None,
            delivered_tree: None,
            verification: Vec::new(),
            events: Vec::new(),
            report: None,
        };
        // `create_transaction` publishes this preparation journal atomically;
        // the boundary after it is therefore discoverable on restart.
        journal.verification_workspace = Some(self.store.verification_workspace_intent(&journal)?);
        self.verification_workspace = Some(self.store.create_transaction(&journal)?);
        self.hit(DurableBoundary::TransactionCreated)?;

        for (index, snapshot) in prepared.snapshots.iter().enumerate() {
            journal.snapshot_active = Some(index);
            self.persist_journal(&mut journal)?;
            self.hit(DurableBoundary::SnapshotIntentPersisted { index })?;
            self.store.persist_snapshot(
                &journal.transaction_id,
                &journal.snapshots[index],
                &snapshot.bytes,
            )?;
            self.hit(DurableBoundary::SnapshotDataPersisted { index })?;
            journal.snapshots_persisted = index + 1;
            journal.snapshot_active = None;
            self.persist_journal(&mut journal)?;
            self.hit(DurableBoundary::SnapshotPersisted { index })?;
        }
        self.transition(&mut journal, TransactionState::Prepared)?;
        match journal.execution.clone() {
            PreparedMode::Export(plan) => {
                self.execute_export(&mut journal, &plan, &prepared.snapshots)
            }
            PreparedMode::InPlace(plan) => {
                self.execute_in_place(&mut journal, &plan, &prepared.snapshots)
            }
        }
    }

    /// Settle the one external journal for this pinned project identity. The
    /// caller supplies no contract path and no prepared source value.
    pub fn recover(
        &mut self,
        project_identity_token: &str,
        project_display_root: &str,
    ) -> Result<TransactionReport, TransactionError> {
        self.origin = MutationOrigin::Recovery;
        self.store.prove_outside_project(project_display_root)?;
        self.hit(DurableBoundary::StoreProvedExternal)?;
        let key = project_key(project_identity_token);
        let _lock = self.store.lock_project(&key)?;
        self.hit(DurableBoundary::ProjectLockAcquired)?;
        let mut journal = self
            .store
            .pending(&key)?
            .ok_or(TransactionError::NoPendingTransaction)?;
        validate::journal(&journal, &key, project_display_root)?;
        if journal.state == TransactionState::Complete {
            return self.finish_complete(&journal);
        }
        self.filesystem.rebind_owned_tree(&journal)?;
        let snapshot_observation = self.store.verify_snapshot_progress(&journal)?;
        match (journal.snapshot_active, snapshot_observation) {
            (None, SnapshotActiveObservation::None)
            | (
                Some(_),
                SnapshotActiveObservation::Absent | SnapshotActiveObservation::ExactPresent,
            ) => {}
            _ => {
                return Err(TransactionError::Store(
                    "snapshot intent/data observation is inconsistent".to_owned(),
                ));
            }
        }
        if journal.state == TransactionState::Preparing {
            return self.refuse_unmutated(
                &mut journal,
                "recovered incomplete preparation; no project mutation was possible".to_owned(),
            );
        }
        let execution = journal.execution.clone();
        match execution {
            PreparedMode::Export(plan) if journal.state.is_pre_verified() => {
                self.rollback_export(&mut journal, &plan)
            }
            PreparedMode::InPlace(plan) if journal.state.is_pre_verified() => {
                self.rollback_in_place(&mut journal, &plan)
            }
            PreparedMode::Export(plan) if journal.state.rolls_forward() => {
                self.finish_verified(&mut journal, CleanupPlan::Export(&plan))
            }
            PreparedMode::InPlace(plan) if journal.state.rolls_forward() => {
                self.finish_verified(&mut journal, CleanupPlan::InPlace(&plan))
            }
            _ if journal.state == TransactionState::RolledBack => {
                self.finish_rolled_back(&mut journal)
            }
            _ if journal.state == TransactionState::RollbackFailed => {
                self.finish_rollback_failed(&journal)
            }
            _ => Err(TransactionError::Store(format!(
                "journal state {:?} is invalid for {:?}",
                journal.state, journal.mode
            ))),
        }
    }

    /// Recovery after the caller acquired the external project lock and read
    /// the journal/verifier snapshots under that same lock.
    pub fn recover_under_held_gate(
        &mut self,
        key: ProjectKey,
        project_identity_token: &str,
        project_display_root: &str,
        mut journal: Journal,
    ) -> Result<TransactionReport, TransactionError> {
        self.origin = MutationOrigin::Recovery;
        if key != project_key(project_identity_token) || journal.project_key != key {
            return Err(TransactionError::Store(
                "held recovery gate does not match journal project identity".to_owned(),
            ));
        }
        validate::journal(&journal, &key, project_display_root)?;
        if journal.state == TransactionState::Complete {
            return self.finish_complete(&journal);
        }
        self.filesystem.rebind_owned_tree(&journal)?;
        let snapshot_observation = self.store.verify_snapshot_progress(&journal)?;
        match (journal.snapshot_active, snapshot_observation) {
            (None, SnapshotActiveObservation::None)
            | (
                Some(_),
                SnapshotActiveObservation::Absent | SnapshotActiveObservation::ExactPresent,
            ) => {}
            _ => {
                return Err(TransactionError::Store(
                    "snapshot intent/data observation is inconsistent".to_owned(),
                ));
            }
        }
        if journal.state == TransactionState::Preparing {
            return self.refuse_unmutated(
                &mut journal,
                "recovered incomplete preparation; no project mutation was possible".to_owned(),
            );
        }
        let execution = journal.execution.clone();
        match execution {
            PreparedMode::Export(plan) if journal.state.is_pre_verified() => {
                self.rollback_export(&mut journal, &plan)
            }
            PreparedMode::InPlace(plan) if journal.state.is_pre_verified() => {
                self.rollback_in_place(&mut journal, &plan)
            }
            PreparedMode::Export(plan) if journal.state.rolls_forward() => {
                self.finish_verified(&mut journal, CleanupPlan::Export(&plan))
            }
            PreparedMode::InPlace(plan) if journal.state.rolls_forward() => {
                self.finish_verified(&mut journal, CleanupPlan::InPlace(&plan))
            }
            _ if journal.state == TransactionState::RolledBack => {
                self.finish_rolled_back(&mut journal)
            }
            _ if journal.state == TransactionState::RollbackFailed => {
                self.finish_rollback_failed(&journal)
            }
            _ => Err(TransactionError::Store(format!(
                "journal state {:?} is invalid for {:?}",
                journal.state, journal.mode
            ))),
        }
    }

    fn execute_export(
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

    fn execute_in_place(
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
    fn apply_in_place_step(
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

    fn rollback_export_with_event(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
        event: String,
    ) -> Result<TransactionReport, TransactionError> {
        journal.events.push(event);
        self.rollback_export(journal, plan)
    }

    fn rollback_export(
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

    fn refuse_export_race(
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

    fn rollback_in_place_with_event(
        &mut self,
        journal: &mut Journal,
        plan: &InPlacePlan,
        event: String,
    ) -> Result<TransactionReport, TransactionError> {
        journal.events.push(event);
        self.rollback_in_place(journal, plan)
    }

    fn rollback_in_place(
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

    fn complete_rollback(
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

    fn rollback_failed<T>(
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

    fn rollback_error<T>(
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

    fn finish_verified(
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

    fn finish_rolled_back(
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

    fn finish_complete(
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

    fn finish_rollback_failed(
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

    fn refuse_unmutated(
        &mut self,
        journal: &mut Journal,
        detail: String,
    ) -> Result<TransactionReport, TransactionError> {
        let mut report = self.base_report(journal, Outcome::Refused, Cleanup::Complete);
        report.events.push(detail);
        self.finish_terminal_without_product(journal, report)
    }

    fn finish_terminal_without_product(
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

    fn rolled_back_report(&self, journal: &Journal, restored: Digest) -> TransactionReport {
        let mut report = self.base_report(journal, Outcome::RolledBack, Cleanup::Complete);
        report.after_tree = Some(restored);
        report
            .events
            .push("rollback restored every sealed before state".to_owned());
        report
    }

    #[allow(clippy::too_many_arguments)]
    fn verify(
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
    fn reprove_and_record(
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

    fn base_report(
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

    fn transition(
        &mut self,
        journal: &mut Journal,
        next: TransactionState,
    ) -> Result<(), TransactionError> {
        validate_transition(journal.mode, &journal.state, &next)?;
        journal.state = next.clone();
        self.persist_journal(journal)?;
        self.hit(DurableBoundary::JournalPersisted(next))
    }

    fn persist_same_state(
        &mut self,
        journal: &mut Journal,
        boundary: DurableBoundary,
    ) -> Result<(), TransactionError> {
        self.persist_journal(journal)?;
        self.hit(boundary)
    }

    fn hit(&mut self, boundary: DurableBoundary) -> Result<(), TransactionError> {
        self.faults.boundary(boundary)
    }

    fn persist_journal(&mut self, journal: &mut Journal) -> Result<(), TransactionError> {
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

    fn persist_report(
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

    fn retire_transaction(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.verifier.release_verification_workspace();
        self.store.retire_transaction(journal)
    }

    fn mark_apply_intent(
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

    fn refresh_owned_seal(
        &mut self,
        journal: &mut Journal,
        name: &str,
        owner: &str,
    ) -> Result<(), TransactionError> {
        journal.owned_tree_seal = Some(self.filesystem.owned_tree_seal(name, owner)?);
        self.persist_journal(journal)
    }

    fn cleanup_owned_tree(
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

    fn mark_applied(
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

    fn mark_rollback_intent(
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

    fn mark_rolled_back(
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

    fn mark_export_tree_rolled_back(
        &mut self,
        journal: &mut Journal,
        plan: &ExportPlan,
        actual: &TreeManifest,
    ) -> Result<(), TransactionError> {
        self.mark_export_count_rolled_back(journal, plan, actual.entries.len())
    }

    fn mark_export_count_rolled_back(
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

    fn mark_absent_export_rollbacks(
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

enum CleanupPlan<'a> {
    Export(&'a ExportPlan),
    InPlace(&'a InPlacePlan),
}

fn prepared_after_for_export<'a>(
    entry: &ExportEntry,
    snapshots: &'a [Snapshot],
) -> Result<Option<&'a [u8]>, TransactionError> {
    match &entry.payload {
        Some(ExportPayload::PreparedAfter { snapshot_name }) => snapshots
            .iter()
            .find(|snapshot| snapshot.name == *snapshot_name)
            .map(|snapshot| Some(snapshot.bytes.as_slice()))
            .ok_or_else(|| {
                TransactionError::InvalidPrepared(format!(
                    "missing prepared-after snapshot `{snapshot_name}`"
                ))
            }),
        _ => Ok(None),
    }
}

fn prepared_after_for_step<'a>(
    step: &MutationStep,
    snapshots: &'a [Snapshot],
) -> Result<Option<&'a [u8]>, TransactionError> {
    if step.kind != MutationKind::AtomicRewrite {
        return Ok(None);
    }
    let name = format!("after/{}", step.id);
    snapshots
        .iter()
        .find(|snapshot| snapshot.name == name)
        .map(|snapshot| Some(snapshot.bytes.as_slice()))
        .ok_or_else(|| TransactionError::InvalidPrepared(format!("missing `{name}`")))
}

fn expected_export_manifests(journal: &Journal, plan: &ExportPlan) -> Vec<TreeManifest> {
    let before = partial_export_manifest(plan, journal.completed_steps);
    let mut expected = vec![before];
    if let Some(active) = journal.active_step {
        let after = partial_export_manifest(plan, active.saturating_add(1));
        if !expected.contains(&after) {
            expected.push(after);
        }
    }
    expected
}

fn partial_export_manifest(plan: &ExportPlan, count: usize) -> TreeManifest {
    let count = count.min(plan.final_manifest.entries.len());
    if count == plan.final_manifest.entries.len() {
        return plan.final_manifest.clone();
    }
    logical_tree_manifest(plan.final_manifest.entries[..count].to_vec())
}

fn require_exact_tree(
    observed: OwnedTreeObservation,
    expected: &TreeManifest,
    label: &str,
) -> Result<(), TransactionError> {
    match observed {
        OwnedTreeObservation::Exact(actual) if actual == *expected => Ok(()),
        OwnedTreeObservation::Third { detail } => {
            Err(TransactionError::ThirdState(format!("{label}: {detail}")))
        }
        other => Err(TransactionError::ThirdState(format!(
            "{label}: expected exact manifest, observed {other:?}"
        ))),
    }
}

fn validate_transaction_id(value: &TransactionId) -> Result<(), TransactionError> {
    if !(6..=64).contains(&value.0.len())
        || !value.0.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(TransactionError::Store(
            "store minted a transaction id outside 6..64 ASCII alphanumerics".to_owned(),
        ));
    }
    Ok(())
}

fn ownership_token(journal: &Journal, role: &str) -> String {
    let material = format!(
        "vibe-scrape-owner-e1\0{}\0{}\0{role}",
        journal.project_key.0, journal.transaction_id.0
    );
    digest(material.as_bytes()).0
}

fn validate_transition(
    mode: TransactionMode,
    from: &TransactionState,
    to: &TransactionState,
) -> Result<(), TransactionError> {
    use TransactionState as S;
    let valid = match mode {
        TransactionMode::Export => matches!(
            (from, to),
            (S::Preparing, S::Prepared)
                | (S::Prepared, S::Candidate)
                | (S::Candidate, S::PublishedPendingVerify)
                | (S::PublishedPendingVerify, S::Verified)
                | (
                    S::Prepared | S::Candidate | S::PublishedPendingVerify,
                    S::RollingBack
                )
                | (S::RollingBack, S::RollingBack)
                | (S::RollingBack, S::RolledBack | S::RollbackFailed)
                | (S::Verified, S::CleanupPending)
                | (S::CleanupPending, S::CleanupPending | S::Complete)
        ),
        TransactionMode::InPlace => matches!(
            (from, to),
            (S::Preparing, S::Prepared)
                | (S::Prepared, S::BeforePassed)
                | (S::BeforePassed, S::Mutating)
                | (S::Mutating, S::ContractBoundary(_))
                | (S::ContractBoundary(_), S::Verified)
                | (
                    S::Prepared | S::BeforePassed | S::Mutating | S::ContractBoundary(_),
                    S::RollingBack
                )
                | (S::RollingBack, S::RollingBack)
                | (S::RollingBack, S::RolledBack | S::RollbackFailed)
                | (S::Verified, S::CleanupPending)
                | (S::CleanupPending, S::CleanupPending | S::Complete)
        ),
    };
    if valid {
        Ok(())
    } else {
        Err(TransactionError::Store(format!(
            "invalid {mode:?} journal transition {from:?} -> {to:?}"
        )))
    }
}

fn planned_mutations(mode: &PreparedMode) -> Vec<MutationProgress> {
    let planned = match mode {
        PreparedMode::Export(plan) => {
            let mut values = vec![PlannedMutationEvidence {
                id: "export/candidate".to_owned(),
                kind: PlannedMutationKind::ExportCandidateCreate,
            }];
            values.extend(plan.entries.iter().enumerate().map(|(index, entry)| {
                PlannedMutationEvidence {
                    id: export_entry_id(index, entry),
                    kind: PlannedMutationKind::ExportEntry,
                }
            }));
            values.push(PlannedMutationEvidence {
                id: "export/publish".to_owned(),
                kind: PlannedMutationKind::ExportPublish,
            });
            values
        }
        PreparedMode::InPlace(plan) => {
            let mut values = vec![PlannedMutationEvidence {
                id: "in-place/quarantine".to_owned(),
                kind: PlannedMutationKind::InPlaceQuarantineCreate,
            }];
            values.extend(
                plan.steps
                    .iter()
                    .chain(std::iter::once(&plan.contract_step))
                    .chain(plan.contract_cleanup_step.iter())
                    .map(|step| PlannedMutationEvidence {
                        id: step.id.clone(),
                        kind: PlannedMutationKind::InPlace(step.kind),
                    }),
            );
            values
        }
    };
    planned
        .into_iter()
        .map(|value| MutationProgress {
            id: value.id,
            kind: value.kind,
            status: MutationStatus::Planned,
        })
        .collect()
}

fn export_entry_id(index: usize, entry: &ExportEntry) -> String {
    format!("export/entry/{index}/{}", entry.target_path)
}

fn set_progress_status(
    journal: &mut Journal,
    id: &str,
    status: MutationStatus,
) -> Result<(), TransactionError> {
    let progress = journal
        .mutation_progress
        .iter_mut()
        .find(|progress| progress.id == id)
        .ok_or_else(|| TransactionError::Store(format!("journal has no mutation `{id}`")))?;
    progress.status = status;
    Ok(())
}

fn progress_status(journal: &Journal, id: &str) -> Option<MutationStatus> {
    journal
        .mutation_progress
        .iter()
        .find(|progress| progress.id == id)
        .map(|progress| progress.status)
}

fn both_absent_is_explained(journal: &Journal) -> bool {
    let candidate = progress_status(journal, "export/candidate");
    let publish = progress_status(journal, "export/publish");
    let publish_applied_without_rollback = journal.actual_mutations.iter().any(|actual| {
        actual.id == "export/publish" && actual.direction == MutationDirection::Apply
    }) && !journal.actual_mutations.iter().any(|actual| {
        actual.id == "export/publish" && actual.direction == MutationDirection::Rollback
    });
    if publish == Some(MutationStatus::Applied) || publish_applied_without_rollback {
        return false;
    }
    let pre_create = matches!(
        candidate,
        Some(MutationStatus::Planned | MutationStatus::ApplyIntent)
    ) && journal.completed_steps == 0
        && journal.active_step.is_none()
        && !journal.actual_mutations.iter().any(|actual| {
            actual.id == "export/candidate" && actual.direction == MutationDirection::Apply
        });
    let rolled_back = matches!(
        candidate,
        Some(MutationStatus::RollbackIntent | MutationStatus::RolledBack)
    );
    pre_create || rolled_back
}

fn record_actual(
    journal: &mut Journal,
    id: &str,
    direction: MutationDirection,
    status: MutationStatus,
    origin: MutationOrigin,
) -> Result<(), TransactionError> {
    let kind = journal
        .mutation_progress
        .iter()
        .find(|progress| progress.id == id)
        .map(|progress| progress.kind)
        .ok_or_else(|| TransactionError::Store(format!("journal has no mutation `{id}`")))?;
    let evidence = ActualMutationEvidence {
        id: id.to_owned(),
        kind,
        direction,
        origin,
        status,
    };
    if journal
        .actual_mutations
        .iter()
        .any(|prior| prior.id == evidence.id && prior.direction == evidence.direction)
    {
        return Ok(());
    }
    journal.actual_mutations.push(evidence);
    Ok(())
}

fn infer_actual(
    journal: &mut Journal,
    id: &str,
    direction: MutationDirection,
    origin: MutationOrigin,
) -> Result<(), TransactionError> {
    let status = match direction {
        MutationDirection::Apply => MutationStatus::Applied,
        MutationDirection::Rollback => MutationStatus::RolledBack,
    };
    set_progress_status(journal, id, status)?;
    record_actual(journal, id, direction, status, origin)
}

fn infer_export_apply_progress(
    journal: &mut Journal,
    plan: &ExportPlan,
    actual: &TreeManifest,
    origin: MutationOrigin,
) -> Result<(), TransactionError> {
    infer_actual(
        journal,
        "export/candidate",
        MutationDirection::Apply,
        origin,
    )?;
    for (index, entry) in plan.entries.iter().enumerate().take(actual.entries.len()) {
        infer_actual(
            journal,
            &export_entry_id(index, entry),
            MutationDirection::Apply,
            origin,
        )?;
    }
    Ok(())
}

fn is_fault(error: &TransactionError) -> bool {
    matches!(error, TransactionError::FaultInjected(_))
}
