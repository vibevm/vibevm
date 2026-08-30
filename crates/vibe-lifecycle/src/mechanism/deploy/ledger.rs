//! The engine's checkpoint sink, as a PROVIDER sees it — §7.2's "Apply
//! checkpoints completed operations without storing secrets".
//!
//! Its own cell rather than a member of the state home next door because it
//! is the state home's one provider-facing surface, and the narrowness is
//! the point: a provider can say "this operation completed" and can say
//! nothing else. It cannot read the ledger back, cannot rewrite it, and
//! cannot decide where it lives. Every call publishes the ledger atomically
//! before it returns, because a checkpoint that is only in memory is not a
//! checkpoint.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use super::error::DeployError;
use super::state::{CHECKPOINT_EPOCH, CheckpointRecord, DeployState, DeploymentHome};
use crate::mechanism::MechanismError;

/// One plan's checkpoint ledger, held open across that plan's apply.
#[derive(Debug)]
pub(crate) struct CheckpointLedger<'a> {
    state: &'a DeployState,
    home: &'a DeploymentHome,
    record: CheckpointRecord,
}

impl<'a> CheckpointLedger<'a> {
    /// Open a ledger for one plan, adopting whatever an interrupted apply
    /// of the SAME plan already completed.
    pub(crate) fn open(
        state: &'a DeployState,
        home: &'a DeploymentHome,
        plan_hash: &str,
    ) -> Result<Self, DeployError> {
        let record = state
            .read_checkpoints(home, plan_hash)?
            .unwrap_or_else(|| CheckpointRecord {
                schema: CHECKPOINT_EPOCH,
                plan_hash: plan_hash.to_owned(),
                completed: Vec::new(),
            });
        Ok(Self {
            state,
            home,
            record,
        })
    }

    /// Record that one completed operation completed.
    ///
    /// The provider-facing half of the ledger. Its callers name the
    /// operation, not necessarily a receipted resource: the vibe-bin
    /// provider checkpoints its content-addressed payload write under the
    /// payload's own store identity even though §7.1.0 ruling 4 keeps the
    /// payload out of the receipt's OWNED set. §7.2 asks apply to
    /// "checkpoint completed operations", and the payload write is one.
    pub(crate) fn completed(&mut self, resource: &str) -> Result<(), MechanismError> {
        if self.record.completed.iter().any(|done| done == resource) {
            return Ok(());
        }
        self.record.completed.push(resource.to_owned());
        self.state
            .write_checkpoints(self.home, &self.record)
            .map_err(|error| MechanismError::DeployCheckpoint {
                resource: resource.to_owned(),
                reason: error.to_string(),
            })
    }
}
