//! `undeploy` — the inverse deployment, and the half of §6.3.1.3 that runs
//! long after the plan that produced the locks is gone.
//!
//! Its own cell because the inverse is the operation the durable sidecar
//! exists FOR. Apply always knows its own physical destinations: the plan is
//! in its hand. An inverse has only a receipt, and §7.2's record list is the
//! OWNED set — so for a reference owner the receipt says
//! `…/opencode.json#mcp/foo` and the document it must lock is
//! `…/opencode.json`, which the receipt never mentions. §6.3.1.3 is the
//! answer:
//!
//! > "Apply, recovery, saga rollback and undeploy take the deployment-id
//! > lock, then the union of current, committed and pending destination
//! > locks in canonical order. … Successful inverse clears committed
//! > ownership after the rolled-back receipt is durable."
//!
//! What this cell will NOT do is derive the document by parsing the logical
//! member. That would be a second grammar for an identity the engine never
//! wrote down, and §6.3.1.4 forbids it in as many words. An ordinary
//! provider needs no derivation at all — its lock set was proven equal to
//! its owned set before the plan ever applied — so a pre-sidecar receipt
//! stays removable while a reference owner without a binding refuses.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_wire::generated::deploy_receipt::ReceiptStatus;

use super::error::DeployError;
use super::model::{DeployExecution, RemovalOutcome};
use super::sidecar;
use super::state::DeployState;
use super::transaction::Transaction;
use super::{Selected, home_of, identity_of, ownership_of};
use crate::mechanism::DeployTargetRequest;

/// Reverse one ALREADY-resolved selection, in reverse dependency order.
///
/// A target is removed before the target it depends on, exactly as the
/// saga's rollback runs. Each target takes its OWN deployment-state lock and
/// its own committed destination locks, and drops both before the next one:
/// the inverse of a selection is a sequence of independent reversals, not
/// one transaction over all of them, and holding every lock to the end would
/// serialise deployments that never met.
pub(crate) fn undeploy_resolved(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
) -> Result<Vec<RemovalOutcome>, DeployError> {
    let resolved: Vec<&Selected<'_>> = resolved.iter().rev().collect();
    let state = DeployState::open(execution.state_home)?;
    let identity = identity_of(execution);
    let mut outcomes = Vec::with_capacity(resolved.len());
    for selected in resolved {
        let home = home_of(execution, &selected.target.id);
        // §6.3.1.3's order: the deployment-state lock first, so the receipt
        // and the sidecar this reversal reads cannot change under it.
        let _deployment = state.lock_deployment(&home)?;
        let receipt = state
            .read_receipt(&home)?
            .ok_or_else(|| DeployError::NoReceipt {
                target: selected.target.id.clone(),
            })?;
        // …then the PHYSICAL destinations that deployment recorded. For a
        // reference owner this is the only place they exist; a missing or
        // mismatched binding refuses here, before `verify` observes
        // anything and long before `remove` is asked for anything.
        let locks = sidecar::inverse_locks(
            state.read_lock_resources(&home)?.as_ref(),
            &receipt,
            ownership_of(selected),
        )?;
        let _guards = state.lock_destinations(&locks)?;
        let request = DeployTargetRequest {
            target: selected.target,
            profile: &execution.selection.profile,
            project_root: execution.project_root,
            settings_root: execution.settings_root,
            user_home: execution.user_home,
            clients: execution.clients,
            // The engine's read of prior ownership is the receipt being
            // reversed: a provider that has to recognise its own occupant
            // before removing it is looking at exactly this value. No
            // recovery intent here — an inverse settles nothing.
            prior_receipt: Some(&receipt),
            recovery_intent: None,
            artifact: None,
            staging: None,
        };
        let transaction = Transaction {
            state: &state,
            home: &home,
            identity: &identity,
            provider_pin: &selected.pin,
            scope: selected.provider.descriptor().scope(),
            created_at: execution.created_at,
        };
        // `None`: this is UNDEPLOY, not the saga — the receipt-owned files
        // are removed, never "restored" to a generation nobody asked for.
        let removed = transaction.remove(
            selected.provider.as_ref(),
            &request,
            &receipt,
            None,
            ReceiptStatus::RolledBack,
        )?;
        outcomes.push(RemovalOutcome {
            target: selected.target.id.clone(),
            provider: selected.pin.clone(),
            removed,
        });
    }
    Ok(outcomes)
}
