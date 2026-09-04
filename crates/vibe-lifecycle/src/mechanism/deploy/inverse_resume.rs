//! Settle a causally marked saga inverse before ordinary preplanning.

use vibe_wire::generated::deploy_receipt::DeployIdentity;

use super::error::DeployError;
use super::sidecar;
use super::state::DeployState;
use super::transaction::Transaction;
use super::{DeployExecution, Selected, home_of, ownership_of};
use crate::mechanism::DeployTargetRequest;

pub(super) fn resume_inverses(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
) -> Result<(), DeployError> {
    if !execution.state_home.exists() {
        return Ok(());
    }
    let state = DeployState::open(execution.state_home)?;
    let identity = DeployIdentity {
        project: execution.project.to_owned(),
        package: execution.package.map(str::to_owned),
    };
    for selected in resolved {
        let home = home_of(execution, &selected.target.id);
        let _deployment = state.lock_deployment(&home)?;
        let Some(marker) = state.read_inverse(&home)? else {
            continue;
        };
        let receipt = state
            .read_receipt(&home)?
            .ok_or_else(|| DeployError::NoReceipt {
                target: selected.target.id.clone(),
            })?;
        let locks = sidecar::inverse_locks(
            state.read_lock_resources(&home)?.as_ref(),
            &receipt,
            ownership_of(selected),
        )?;
        let _guards = state.lock_destinations(&locks)?;
        let staging = home.staging();
        let request = DeployTargetRequest {
            target: selected.target,
            profile: &execution.selection.profile,
            project_root: execution.project_root,
            settings_root: execution.settings_root,
            user_home: execution.user_home,
            clients: execution.clients,
            prior_receipt: Some(&receipt),
            recovery_intent: None,
            artifact: None,
            staging: Some(&staging),
        };
        Transaction {
            state: &state,
            home: &home,
            identity: &identity,
            provider_pin: &selected.pin,
            scope: selected.provider.descriptor().scope(),
            created_at: execution.created_at,
        }
        .resume_inverse(selected.provider.as_ref(), &request, &receipt, &marker)?;
        state.cleanup_staging(&home)?;
    }
    Ok(())
}
