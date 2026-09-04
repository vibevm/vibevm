//! Honest default for the capability surface not yet exported by vibe-safefs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use super::model::*;
use super::traits::{ExportTreeSlot, TransactionFilesystem};

/// This backend deliberately never reports success. It makes the integration
/// blocker precise instead of falling back to ambient `std::fs` rename/delete.
#[derive(Debug, Default)]
pub struct SafefsCapabilityGap;

impl SafefsCapabilityGap {
    fn missing<T>(primitive: RequiredPrimitive) -> Result<T, TransactionError> {
        Err(TransactionError::MissingPrimitive(primitive))
    }
}

impl TransactionFilesystem for SafefsCapabilityGap {
    fn rebind_owned_tree(&mut self, _journal: &Journal) -> Result<(), TransactionError> {
        Self::missing(RequiredPrimitive::ExclusivePinnedDirectory)
    }

    fn owned_tree_seal(
        &mut self,
        _name: &str,
        _ownership_token: &str,
    ) -> Result<OwnedTreeSeal, TransactionError> {
        Self::missing(RequiredPrimitive::ExclusivePinnedDirectory)
    }
    fn create_export_candidate(
        &mut self,
        _plan: &ExportPlan,
        _candidate_name: &str,
        _ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        Self::missing(RequiredPrimitive::ExclusivePinnedDirectory)
    }
    fn apply_export_entry(
        &mut self,
        _plan: &ExportPlan,
        _candidate_name: &str,
        _ownership_token: &str,
        _entry: &ExportEntry,
        _prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        Self::missing(RequiredPrimitive::CapabilityRelativeRename)
    }
    fn observe_export_tree(
        &mut self,
        _plan: &ExportPlan,
        _slot: ExportTreeSlot,
        _candidate_name: &str,
        _ownership_token: &str,
    ) -> Result<OwnedTreeObservation, TransactionError> {
        Self::missing(RequiredPrimitive::ExactManifestTreeRemoval)
    }
    fn publish_export_noreplace(
        &mut self,
        _plan: &ExportPlan,
        _candidate_name: &str,
        _ownership_token: &str,
    ) -> Result<(), TransactionError> {
        Self::missing(RequiredPrimitive::AtomicNoReplaceDirectoryRename)
    }
    fn unpublish_export(
        &mut self,
        _plan: &ExportPlan,
        _candidate_name: &str,
        _ownership_token: &str,
    ) -> Result<(), TransactionError> {
        Self::missing(RequiredPrimitive::CapabilityRelativeRename)
    }
    fn prepare_owned_tree_cleanup(
        &mut self,
        _journal: &Journal,
        _name: &str,
        _ownership_token: &str,
        _seal: &OwnedTreeSeal,
        _completed: &[String],
    ) -> Result<OwnedTreeCleanupPreparation, TransactionError> {
        Self::missing(RequiredPrimitive::ExactManifestTreeRemoval)
    }
    fn execute_owned_tree_cleanup(
        &mut self,
        _journal: &Journal,
        _name: &str,
        _ownership_token: &str,
        _seal: &OwnedTreeSeal,
        _completed: &[String],
        _intent: &OwnedTreeCleanupIntent,
    ) -> Result<OwnedTreeCleanupCompletion, TransactionError> {
        Self::missing(RequiredPrimitive::ExactManifestTreeRemoval)
    }
    fn create_quarantine(
        &mut self,
        _plan: &InPlacePlan,
        _quarantine_name: &str,
        _ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        Self::missing(RequiredPrimitive::SameVolumeIdentityComparison)
    }
    fn observe_step(
        &mut self,
        _plan: &InPlacePlan,
        _quarantine_name: &str,
        _ownership_token: &str,
        _step: &MutationStep,
    ) -> Result<SealedObservation, TransactionError> {
        Self::missing(RequiredPrimitive::CapabilityRelativeRename)
    }
    fn observe_quarantine_root(
        &mut self,
        _plan: &InPlacePlan,
        _quarantine_name: &str,
        _ownership_token: &str,
    ) -> Result<OwnedRootObservation, TransactionError> {
        Self::missing(RequiredPrimitive::ExactManifestTreeRemoval)
    }
    fn apply_step(
        &mut self,
        _plan: &InPlacePlan,
        _quarantine_name: &str,
        _ownership_token: &str,
        _step: &MutationStep,
        _prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        Self::missing(RequiredPrimitive::CapabilityRelativeRename)
    }
    fn rollback_step(
        &mut self,
        _plan: &InPlacePlan,
        _quarantine_name: &str,
        _ownership_token: &str,
        _step: &MutationStep,
    ) -> Result<(), TransactionError> {
        Self::missing(RequiredPrimitive::CapabilityRelativeRename)
    }
    fn cleanup_unpublished_step_stage(
        &mut self,
        _plan: &InPlacePlan,
        _quarantine_name: &str,
        _ownership_token: &str,
        _step: &MutationStep,
    ) -> Result<(), TransactionError> {
        Self::missing(RequiredPrimitive::CapabilityRelativeRename)
    }
}
