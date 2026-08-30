//! §7.2's saga — what a failed multi-target deploy does with the targets
//! that already applied.
//!
//! > "A failed multi-target deploy is a recorded saga: already-applied
//! > reversible targets are rolled back in reverse order; irreversible
//! > results remain visible as partial, never reported as success."
//!
//! Its own cell because it is the one path that runs while the run is
//! ALREADY failing, and that changes what every decision in it is allowed
//! to do: it may not replace the original failure, it may not refuse, and
//! it may not leave a target it could not reverse unreported. Keeping it
//! beside the happy path made all three easy to read past.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_wire::generated::deploy_receipt::{DeployIdentity, DeployReceipt, ReceiptStatus};

use super::error::DeployError;
use super::state::DeployState;
use super::transaction::Transaction;
use super::{DeployExecution, Selected, home_of};
use crate::mechanism::DeployTargetRequest;

/// §7.2's saga: roll the reversible prefix back in REVERSE order and
/// report what survives as partial.
///
/// A rollback that itself fails does not replace the original failure —
/// the run is already failing, and the reason it started failing is the
/// one an operator needs. The target is simply not counted as reversed.
pub(crate) fn unwind(
    execution: &DeployExecution<'_>,
    state: &DeployState,
    identity: &DeployIdentity,
    resolved: &[Selected<'_>],
    applied: &[(usize, DeployReceipt)],
    failure: DeployError,
) -> DeployError {
    if applied.is_empty() {
        return failure;
    }
    let mut rolled_back: Vec<String> = Vec::new();
    let mut retained: Vec<String> = Vec::new();
    for (index, receipt) in applied.iter().rev() {
        let Some(selected) = resolved.get(*index) else {
            continue;
        };
        if !receipt.reversible {
            retained.push(selected.target.id.clone());
            continue;
        }
        // The same gap `undeploy` refuses on, at the same law: a reference
        // owner's receipt records its logical member, never the physical
        // destination it locked, so this engine cannot take the lock a
        // reversal would need. Here it lands as RETAINED — the saga's own
        // word for a target it could not reverse — rather than as a second
        // failure replacing the one that started the unwind.
        if selected.provider.descriptor().reference_ownership {
            retained.push(selected.target.id.clone());
            continue;
        }
        let home = home_of(execution, &selected.target.id);
        let request = DeployTargetRequest {
            target: selected.target,
            profile: &execution.selection.profile,
            project_root: execution.project_root,
            settings_root: execution.settings_root,
            user_home: execution.user_home,
            clients: execution.clients,
            artifact: None,
            staging: None,
        };
        let transaction = Transaction {
            state,
            home: &home,
            identity,
            provider_pin: &selected.pin,
            scope: selected.provider.descriptor().scope(),
            created_at: execution.created_at,
        };
        match transaction.remove(
            selected.provider.as_ref(),
            &request,
            receipt,
            // The saga RESTORES: the failed generation's handle is what
            // the destination held before it, and rolling back means
            // putting exactly that back.
            receipt.prior_state_handle.as_deref(),
            ReceiptStatus::RolledBack,
        ) {
            Ok(_) => rolled_back.push(selected.target.id.clone()),
            Err(_) => retained.push(selected.target.id.clone()),
        }
    }
    let failed = resolved
        .get(applied.len())
        .map_or_else(|| "<unknown>".to_owned(), |next| next.target.id.clone());
    DeployError::Saga {
        target: failed,
        reason: failure.to_string(),
        rolled_back: list(&rolled_back),
        retained: list(&retained),
    }
}

/// One list, or the word that says it is empty.
pub(crate) fn list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}
