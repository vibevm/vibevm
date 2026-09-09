//! Journal-first export and in-place state machines.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use super::model::*;
use super::report;
use super::sha256::{digest, project_key};
use super::traits::*;
use super::validate;

mod export;
mod helpers;
mod in_place;
mod in_place_rollback;
mod persistence;
mod rollback;
mod terminal;

use helpers::*;

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
            schema: JOURNAL_EPOCH,
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
}
