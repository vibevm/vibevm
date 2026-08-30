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
//!
//! Every rollback below is a full inverse deployment and takes the same
//! locks one does (§6.3.1.3: "Apply, recovery, saga rollback and undeploy
//! take the deployment-id lock, then the union of current, committed and
//! pending destination locks"). The failing run holds no lock by the time
//! this cell runs — `apply_one`'s guards drop when it returns — so a
//! rollback that reversed a destination unlocked would race a concurrent
//! deployment of the same one, which is the exact race the locks exist for.
//! What is different here is only the failure handling: a lock this cell
//! cannot take, or a sidecar binding it cannot read, lands as RETAINED and
//! never as a second error replacing the one that started the unwind.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_wire::generated::deploy_receipt::{DeployIdentity, DeployReceipt, ReceiptStatus};

use super::error::DeployError;
use super::sidecar;
use super::state::DeployState;
use super::transaction::Transaction;
use super::{DeployExecution, Selected, home_of, ownership_of};
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
        let home = home_of(execution, &selected.target.id);
        // The deployment-state lock, then the committed physical
        // destinations this receipt's generation recorded. A reference
        // owner whose sidecar binding is missing or belongs to another
        // generation is RETAINED here — the saga's own word for a target it
        // could not reverse — because refusing would replace the failure
        // that started the unwind with one about bookkeeping.
        let Ok(guards) = lock_inverse(state, &home, selected, receipt) else {
            retained.push(selected.target.id.clone());
            continue;
        };
        let request = DeployTargetRequest {
            target: selected.target,
            profile: &execution.selection.profile,
            project_root: execution.project_root,
            settings_root: execution.settings_root,
            user_home: execution.user_home,
            clients: execution.clients,
            prior_receipt: Some(receipt),
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
        drop(guards);
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

/// Take one rollback's whole lock set, or say it could not be taken.
///
/// The guards are returned rather than taken inside the reversal because
/// their LIFETIME is the law: they must outlive `verify`, `remove` and the
/// rolled-back receipt, and a helper that dropped them on return would leave
/// exactly the unlocked inverse this fixes.
fn lock_inverse(
    state: &DeployState,
    home: &super::DeploymentHome,
    selected: &Selected<'_>,
    receipt: &DeployReceipt,
) -> Result<Vec<vibe_safefs::LockGuard>, DeployError> {
    let deployment = state.lock_deployment(home)?;
    let locks = sidecar::inverse_locks(
        state.read_lock_resources(home)?.as_ref(),
        receipt,
        ownership_of(selected),
    )?;
    let mut guards = state.lock_destinations(&locks)?;
    guards.push(deployment);
    Ok(guards)
}

/// One list, or the word that says it is empty.
pub(crate) fn list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}
