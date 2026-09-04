//! Narrow integration boundaries for durable storage, capability mutation and
//! the already-prepared health/residual engine.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use super::model::*;

#[derive(Debug)]
pub struct ProjectLock {
    _private: (),
}

impl ProjectLock {
    /// Store implementations return this only after an exclusive, identity-
    /// rechecked OS lock has been acquired outside the project.
    pub fn acquired() -> Self {
        Self { _private: () }
    }
}

/// External, implementation-owned transaction home. Every successful write
/// method is a durability promise: file data and the containing directory have
/// been synced before return.
pub trait TransactionStore {
    fn prove_outside_project(&mut self, project_display_root: &str)
    -> Result<(), TransactionError>;
    fn lock_project(&mut self, project: &ProjectKey) -> Result<ProjectLock, TransactionError>;
    fn pending(&mut self, project: &ProjectKey) -> Result<Option<Journal>, TransactionError>;
    /// Re-read every snapshot in the journaled durable prefix, prove its exact
    /// bytes/digest/mode, and prove no unjournaled snapshot name is present.
    fn verify_snapshot_progress(
        &mut self,
        journal: &Journal,
    ) -> Result<SnapshotActiveObservation, TransactionError>;
    /// Read one exact journaled member of the durable snapshot prefix.
    fn read_snapshot(&mut self, journal: &Journal, name: &str)
    -> Result<Vec<u8>, TransactionError>;
    fn mint_transaction_id(
        &mut self,
        project: &ProjectKey,
    ) -> Result<TransactionId, TransactionError>;
    /// Exclusively creates the transaction home and durably publishes its
    /// discoverable preparation journal in the same operation. A crash after
    /// success is therefore recoverable even before snapshot zero.
    /// Compute the immutable verification-workspace name/path/ownership intent
    /// without creating any namespace entry.
    fn verification_workspace_intent(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspaceIntent, TransactionError>;
    /// Strictly serialize and size-check the exact revision-zero journal
    /// before creating any transaction namespace. Then create the transaction
    /// home/workspace and publish those already-checked exact bytes.
    fn create_transaction(
        &mut self,
        journal: &Journal,
    ) -> Result<VerificationWorkspace, TransactionError>;
    fn persist_snapshot(
        &mut self,
        transaction: &TransactionId,
        record: &SnapshotRecord,
        bytes: &[u8],
    ) -> Result<(), TransactionError>;
    fn persist_journal(&mut self, journal: &Journal) -> Result<(), TransactionError>;
    /// Persist the generated canonical `scrape_report/e1` bytes after proving
    /// they are the projection of the durable journal/domain evidence.
    fn persist_report(
        &mut self,
        report: &TransactionReport,
        canonical_wire: &[u8],
    ) -> Result<(), TransactionError>;
    /// Retires only the exact journal directory named by the transaction.
    fn retire_transaction(&mut self, journal: &Journal) -> Result<(), TransactionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTreeSlot {
    Candidate,
    Output,
}

/// Capability-only project and sibling-tree mutations. Implementations are
/// responsible for no-follow walks, single-link regular files, identity
/// rechecks, same-volume proof and the strongest truthful host namespace
/// primitive.  Windows namespace mutations are ordered by the engine's
/// durable WAL; an unavailable POSIX-style directory fsync must never be
/// reported as though it succeeded. No method may use an ambient recursive
/// deletion.
pub trait TransactionFilesystem {
    fn rebind_owned_tree(&mut self, journal: &Journal) -> Result<(), TransactionError>;

    fn owned_tree_seal(
        &mut self,
        name: &str,
        ownership_token: &str,
    ) -> Result<OwnedTreeSeal, TransactionError>;
    /// Create the exact sibling name exclusively and bind it to the ownership
    /// token that was durable before this call. Existing state refuses.
    fn create_export_candidate(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError>;
    fn apply_export_entry(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
        entry: &ExportEntry,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError>;
    fn observe_export_tree(
        &mut self,
        plan: &ExportPlan,
        slot: ExportTreeSlot,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<OwnedTreeObservation, TransactionError>;
    /// Atomic directory rename that must fail if the output name has any
    /// occupant. Replacing publication is forbidden.
    fn publish_export_noreplace(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<(), TransactionError>;
    /// Roll a still-exact output back to the transaction-owned sibling name.
    fn unpublish_export(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<(), TransactionError>;
    /// Prepare the next canonical, manifest-bound owned-tree removal without
    /// mutating. The engine durably journals an `Intent` before executing it.
    fn prepare_owned_tree_cleanup(
        &mut self,
        journal: &Journal,
        name: &str,
        ownership_token: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
    ) -> Result<OwnedTreeCleanupPreparation, TransactionError>;
    /// Execute exactly the journaled cleanup intent. A missing target is
    /// accepted only when it is the target of this exact in-flight intent.
    fn execute_owned_tree_cleanup(
        &mut self,
        journal: &Journal,
        name: &str,
        ownership_token: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
        intent: &OwnedTreeCleanupIntent,
    ) -> Result<OwnedTreeCleanupCompletion, TransactionError>;

    fn create_quarantine(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError>;
    fn observe_step(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<SealedObservation, TransactionError>;
    /// Observe only the identity-bound quarantine root. `ExactOwned` proves
    /// the journaled sibling name still denotes the exclusively created root;
    /// it says nothing about descendant state, which step observations prove.
    fn observe_quarantine_root(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
    ) -> Result<OwnedRootObservation, TransactionError>;
    fn apply_step(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError>;
    fn rollback_step(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError>;
    /// Remove only an exact deterministic unpublished stage explained by the
    /// durable step intent. Absence is idempotent; every other occupant is a
    /// third state.
    fn cleanup_unpublished_step_stage(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError>;
}

/// Adapter over the finalized health engine and residual/preservation proof.
/// It receives only journaled snapshots. Recovery has no contract-source path
/// and cannot reread or reparse the source contract.
pub trait TransactionVerifier {
    /// Release every live view/scratch capability before the transaction
    /// store begins exact workspace/home retirement.
    fn release_verification_workspace(&mut self) {}

    /// Observe the private phase view before any child is executed. The core
    /// compares the complete returned manifest to `context.expected_tree`.
    fn observe_phase_view(
        &mut self,
        journal: &Journal,
        context: &VerificationContext<'_>,
    ) -> Result<TreeManifest, TransactionError>;

    fn execute_verification(
        &mut self,
        journal: &Journal,
        context: VerificationContext<'_>,
    ) -> Result<VerificationEvidence, TransactionError>;

    /// Re-observe the protected real source/delivered tree. This is distinct
    /// from the disposable phase view and cannot be implemented by returning
    /// that view's manifest.
    fn reprove_real_tree(
        &mut self,
        journal: &Journal,
        root_kind: VerificationRootKind,
        root_display: &str,
    ) -> Result<TreeManifest, TransactionError>;
}

pub trait FaultInjector {
    fn boundary(&mut self, boundary: DurableBoundary) -> Result<(), TransactionError>;
}

#[derive(Debug, Default)]
pub struct NoFaults;

impl FaultInjector for NoFaults {
    fn boundary(&mut self, _boundary: DurableBoundary) -> Result<(), TransactionError> {
        Ok(())
    }
}

/// Temporary adapter seam while `crate::model::PreparedScrape` and generated
/// report types are still moving. It must not perform fresh observation.
pub trait PreparedScrapeSource {
    fn into_transaction(self) -> Result<PreparedTransaction, TransactionError>;
}

impl PreparedScrapeSource for PreparedTransaction {
    fn into_transaction(self) -> Result<PreparedTransaction, TransactionError> {
        Ok(self)
    }
}
