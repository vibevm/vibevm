//! Deploy provider selection, transaction, inverse, and restart admission.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{DeployTarget, ExtensionHandler};
use vibe_extension_registry::{MechanismSelection, SelectionStep, resolve_mechanism};
use vibe_wire::generated::deploy_receipt::{DeployIdentity, DeployReceipt};

pub(crate) mod artifact;
pub(crate) mod error;
pub(crate) mod inverse;
mod inverse_resume;
pub(crate) mod ledger;
pub(crate) mod model;
mod native;
pub(crate) mod observation;
pub(crate) mod opt_launcher;
pub(crate) mod ownership;
pub(crate) mod plan;
pub(crate) mod plugin;
pub(crate) mod preplan;
pub(crate) mod protocol;
pub(crate) mod saga;
pub(crate) mod sidecar;
pub(crate) mod skill;
pub(crate) mod state;
pub(crate) mod transaction;
pub(crate) mod view;

pub use error::DeployError;
pub use model::{
    ClientExecutable, ClientExecutables, DEPLOY_STATE_DIR, DeployExecution, DeployOutcome,
    DeployPlanReport, DeployResourcePlan, DeploySelection, DeployStatus, DeployedResource,
    DeploymentRow, RemovalOutcome, deploy_state_home,
};
pub use plan::plan_deploy_targets;

use super::error::deploy::native_transport;
use super::order::{GraphNode, OrderFault, Unresolved, dag_order};
use super::vibebin::VibeBinProvider;
use super::{
    BUILTIN_VIBE_BIN_NAME, BUILTIN_VIBE_OPT_LAUNCHER_NAME, DeployProvider, DeployTargetRequest,
};
use opt_launcher::VibeOptLauncherProvider;
use plugin::{ClientPluginProvider, PluginClient};
use skill::SkillDeployProvider;
// The inverse path lives in its own cell and is re-exported here.
pub(crate) use inverse::undeploy_resolved;
use inverse_resume::resume_inverses;
use model::row;
use ownership::{ownership_of, refuse_changed_ownership, refuse_foreign_ownership};
use preplan::{Preplanned, preplan};
use saga::unwind;
use state::{DeployState, DeploymentHome};
use transaction::Transaction;

/// Deploy every selected target in dependency order.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn execute_deploy_targets(
    execution: &DeployExecution<'_>,
) -> Result<Vec<DeployOutcome>, DeployError> {
    let resolved = resolve_selection(execution)?;
    apply_selection(execution, &resolved)
}

pub(crate) fn execute_prepared_deploy_targets(
    execution: &DeployExecution<'_>,
    prepared: &crate::native::PreparedNativeMechanisms,
) -> Result<Vec<DeployOutcome>, DeployError> {
    let resolved = resolve_selection_with(execution, Some(prepared))?;
    apply_selection(execution, &resolved)
}

pub(crate) fn plan_prepared_deploy_targets(
    execution: &DeployExecution<'_>,
    prepared: &crate::native::PreparedNativeMechanisms,
) -> Result<Vec<DeployPlanReport>, DeployError> {
    let resolved = resolve_selection_with(execution, Some(prepared))?;
    plan::plan_resolved(execution, &resolved)
}

/// Reverse every selected target in reverse dependency order.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn undeploy_targets(
    execution: &DeployExecution<'_>,
) -> Result<Vec<RemovalOutcome>, DeployError> {
    let resolved = resolve_selection(execution)?;
    undeploy_resolved(execution, &resolved)
}

pub(crate) fn undeploy_prepared_targets(
    execution: &DeployExecution<'_>,
    prepared: &crate::native::PreparedNativeMechanisms,
) -> Result<Vec<RemovalOutcome>, DeployError> {
    let resolved = resolve_selection_with(execution, Some(prepared))?;
    undeploy_resolved(execution, &resolved)
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn list_deployments(state_home: &Path) -> Result<Vec<DeploymentRow>, DeployError> {
    let state = DeployState::open(state_home)?;
    Ok(state
        .receipts()?
        .into_iter()
        .map(|(deployment, receipt)| row(deployment, &receipt))
        .collect())
}

pub(crate) struct Selected<'a> {
    pub(crate) target: &'a DeployTarget,
    pub(crate) provider: Box<dyn DeployProvider>,
    pub(crate) pin: String,
    pub(crate) via: SelectionStep,
    pub(crate) displaced: Option<String>,
}

impl GraphNode for Selected<'_> {
    fn id(&self) -> &str {
        &self.target.id
    }

    fn produces(&self) -> Vec<&str> {
        vec![self.target.id.as_str()]
    }

    fn consumes(&self) -> Vec<&str> {
        self.target
            .depends_on
            .iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }
}

fn resolve_selection<'a>(
    execution: &DeployExecution<'a>,
) -> Result<Vec<Selected<'a>>, DeployError> {
    resolve_selection_with(execution, None)
}

fn resolve_selection_with<'a>(
    execution: &DeployExecution<'a>,
    prepared: Option<&crate::native::PreparedNativeMechanisms>,
) -> Result<Vec<Selected<'a>>, DeployError> {
    let mut selected = Vec::with_capacity(execution.selection.targets.len());
    for id in &execution.selection.targets {
        let target = execution
            .targets
            .iter()
            .find(|candidate| candidate.id == *id)
            .ok_or_else(|| DeployError::UnknownTarget {
                profile: execution.selection.profile.clone(),
                target: id.clone(),
                declared: declared(execution.targets),
            })?;
        if let Some((entry, binding)) = prepared_binding(prepared, target)? {
            selected.push(Selected {
                target,
                provider: Box::new(native::NativeDeployProvider::new(
                    entry,
                    binding,
                    target,
                    execution.project_root,
                )?),
                pin: binding.pin.clone(),
                via: binding.via,
                displaced: binding.displaced_default.clone(),
            });
            continue;
        }
        let selection = resolve_mechanism(
            execution.registry,
            &target.mechanism,
            target.provider.as_ref(),
            execution.routes,
        )?;
        let row = selection.row();
        let pin = row.pin().to_string();
        if matches!(row.handler(), ExtensionHandler::Native { .. }) {
            return Err(native_transport(
                &target.id,
                &pin,
                "admit",
                "selected native provider has no prepared build-fence binding",
            )
            .into());
        }
        let provider = builtin_provider(row.handler(), &target.mechanism.to_string(), &pin)?;
        if !provider
            .descriptor()
            .implements(super::ProviderOperation::Plan)
        {
            return Err(DeployError::PlanNotSupported {
                target: target.id.clone(),
                pin,
            });
        }
        selected.push(Selected {
            target,
            provider,
            pin,
            via: selection.via(),
            displaced: displaced(&selection),
        });
    }
    order(selected)
}

fn prepared_binding<'a>(
    prepared: Option<&'a crate::native::PreparedNativeMechanisms>,
    target: &DeployTarget,
) -> Result<
    Option<(
        &'a crate::native::PreparedNativeMechanism,
        &'a crate::native::NativeMechanismBinding,
    )>,
    DeployError,
> {
    let mut found = None;
    for entry in prepared.into_iter().flat_map(|value| &value.entries) {
        for binding in &entry.bindings {
            if binding.target != target.id {
                continue;
            }
            if found.is_some() || binding.key != target.mechanism {
                return Err(native_transport(
                    &target.id,
                    &binding.pin,
                    "admit",
                    "prepared carriage has duplicate or mismatched target bindings",
                )
                .into());
            }
            found = Some((entry, binding));
        }
    }
    Ok(found)
}

pub(crate) fn validate_restart(
    execution: &DeployExecution<'_>,
    prepared: &crate::native::PreparedNativeMechanisms,
) -> Result<(), DeployError> {
    let state = view::DeployStateView::open(execution.state_home)?;
    for id in &execution.selection.targets {
        let target = execution
            .targets
            .iter()
            .find(|target| target.id == *id)
            .ok_or_else(|| DeployError::UnknownTarget {
                profile: execution.selection.profile.clone(),
                target: id.clone(),
                declared: declared(execution.targets),
            })?;
        let selected = resolve_mechanism(
            execution.registry,
            &target.mechanism,
            target.provider.as_ref(),
            execution.routes,
        )?;
        let pin = selected.row().pin().to_string();
        let current = prepared_binding(Some(prepared), target)?
            .map(|(entry, binding)| {
                sidecar::restart_binding(entry, binding, execution.project_root)
            })
            .transpose()
            .map_err(|error| restart_error(target, &pin, &error.to_string()))?;
        if matches!(selected.row().handler(), ExtensionHandler::Native { .. }) != current.is_some()
        {
            return Err(restart_error(
                target,
                &pin,
                "current selected transport differs from the rehydrated binding",
            ));
        }
        let home = home_of(execution, &target.id);
        let receipt = state.read_receipt(&home)?;
        let intent = state.read_intent(&home)?;
        let sidecar = state.read_lock_resources(&home)?;
        if let Some(receipt) = receipt.as_ref() {
            if receipt.provider.key != pin {
                return Err(restart_error(target, &pin, "receipt provider pin changed"));
            }
            let committed = sidecar
                .as_ref()
                .and_then(|record| record.committed.as_ref())
                .filter(|binding| binding.generation == receipt.generation);
            sidecar::compare_restart(
                target,
                &pin,
                current.as_ref(),
                committed,
                Some(receipt),
                "receipt",
            )?;
        }
        if let Some(intent) = intent.as_ref() {
            let pending = sidecar
                .as_ref()
                .and_then(|record| record.pending.as_ref())
                .filter(|binding| binding.matches(intent.target.generation, &intent.plan_hash));
            sidecar::compare_restart(target, &pin, current.as_ref(), pending, None, "intent")?;
        }
    }
    Ok(())
}

fn restart_error(target: &DeployTarget, pin: &str, reason: &str) -> DeployError {
    native_transport(&target.id, pin, "rehydrate", reason).into()
}

pub(super) fn builtin_provider(
    handler: &ExtensionHandler,
    key: &str,
    pin: &str,
) -> Result<Box<dyn DeployProvider>, DeployError> {
    use skill::SkillClient;
    match handler {
        ExtensionHandler::Builtin { name } if name == BUILTIN_VIBE_BIN_NAME => {
            Ok(Box::new(VibeBinProvider))
        }
        ExtensionHandler::Builtin { name } if name == BUILTIN_VIBE_OPT_LAUNCHER_NAME => {
            Ok(Box::new(VibeOptLauncherProvider))
        }
        ExtensionHandler::Builtin { name } if name == super::BUILTIN_CLAUDE_SKILL_NAME => {
            Ok(Box::new(SkillDeployProvider::new(SkillClient::Claude)))
        }
        ExtensionHandler::Builtin { name } if name == super::BUILTIN_CODEX_SKILL_NAME => {
            Ok(Box::new(SkillDeployProvider::new(SkillClient::Codex)))
        }
        ExtensionHandler::Builtin { name } if name == super::BUILTIN_OPENCODE_SKILL_NAME => {
            Ok(Box::new(SkillDeployProvider::new(SkillClient::OpenCode)))
        }
        ExtensionHandler::Builtin { name } if name == super::BUILTIN_CLAUDE_PLUGIN_NAME => {
            Ok(Box::new(ClientPluginProvider::new(PluginClient::Claude)))
        }
        ExtensionHandler::Builtin { name } if name == super::BUILTIN_CODEX_PLUGIN_NAME => {
            Ok(Box::new(ClientPluginProvider::new(PluginClient::Codex)))
        }
        ExtensionHandler::Builtin { name } if name == super::BUILTIN_OPENCODE_PLUGIN_NAME => {
            Ok(Box::new(ClientPluginProvider::new(PluginClient::OpenCode)))
        }
        ExtensionHandler::Builtin { name } => Err(DeployError::UnknownBuiltinProvider {
            key: key.to_owned(),
            pin: pin.to_owned(),
            name: name.clone(),
        }),
        handler => Err(DeployError::TransportNotLanded {
            key: key.to_owned(),
            pin: pin.to_owned(),
            kind: handler.kind().to_string(),
        }),
    }
}

/// Run the pre-apply epoch, then the deploy saga.
pub(crate) fn apply_selection(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
) -> Result<Vec<DeployOutcome>, DeployError> {
    if resolved.is_empty() {
        return Ok(Vec::new());
    }
    resume_inverses(execution, resolved)?;
    // Every artifact resolved, every provider planned, every prior receipt
    // read and the whole owned/lock resource set judged — before a single
    // destination byte, and before the state home is even created.
    let prepared = preplan(execution, resolved)?;
    apply_prepared(execution, resolved, &prepared)
}

pub(crate) fn apply_prepared(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
    prepared: &[Preplanned],
) -> Result<Vec<DeployOutcome>, DeployError> {
    if resolved.is_empty() {
        return Ok(Vec::new());
    }
    let state = DeployState::open(execution.state_home)?;
    let identity = identity_of(execution);
    let mut outcomes: Vec<DeployOutcome> = Vec::with_capacity(resolved.len());
    let mut applied: Vec<(usize, DeployReceipt)> = Vec::new();
    for ((index, selected), planned) in resolved.iter().enumerate().zip(prepared) {
        match apply_one(execution, &state, &identity, selected, planned) {
            Ok((outcome, receipt)) => {
                applied.push((index, receipt));
                outcomes.push(outcome);
            }
            Err(error) => {
                return Err(unwind(
                    execution, &state, &identity, resolved, &applied, error,
                ));
            }
        }
    }
    for (index, _) in &applied {
        if let Some(selected) = resolved.get(*index) {
            state.cleanup_staging(&home_of(execution, &selected.target.id))?;
        }
    }
    Ok(outcomes)
}

fn apply_one(
    execution: &DeployExecution<'_>,
    state: &DeployState,
    identity: &DeployIdentity,
    selected: &Selected<'_>,
    planned: &Preplanned,
) -> Result<(DeployOutcome, DeployReceipt), DeployError> {
    let home = home_of(execution, &selected.target.id);
    let artifact = &planned.artifact;
    let plan = &planned.plan;
    let descriptor = selected.provider.descriptor();
    let _deployment = state.lock_deployment(&home)?;
    refuse_changed_ownership(state, &home, selected, planned)?;
    let bindings = sidecar::settle_bindings(
        state,
        &home,
        ownership_of(selected),
        selected.provider.native_binding(),
    )?;
    let _guards = state.lock_destinations(&sidecar::union(plan, &bindings))?;
    let resources: Vec<String> = plan
        .resources
        .iter()
        .map(|owned| owned.resource.clone())
        .collect();
    refuse_foreign_ownership(state, &home, &selected.target.id, &resources)?;
    let staging = if descriptor.atomic_replacement {
        Some(state.prepare_staging(&home)?)
    } else {
        None
    };
    let request = DeployTargetRequest {
        target: selected.target,
        profile: &execution.selection.profile,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
        clients: execution.clients,
        prior_receipt: planned.prior_receipt.as_ref(),
        recovery_intent: None,
        artifact: Some(artifact),
        staging: staging.as_deref(),
    };
    let transaction = Transaction {
        state,
        home: &home,
        identity,
        provider_pin: &selected.pin,
        scope: descriptor.scope(),
        created_at: execution.created_at,
    };
    let applied = transaction.apply(selected.provider.as_ref(), &request, plan)?;
    let receipt = state
        .read_receipt(&home)?
        .ok_or_else(|| DeployError::NoReceipt {
            target: selected.target.id.clone(),
        })?;
    Ok((
        DeployOutcome {
            target: selected.target.id.clone(),
            mechanism: selected.target.mechanism.to_string(),
            provider: selected.pin.clone(),
            via: selected.via.to_string(),
            displaced_default: selected.displaced.clone(),
            generation: applied.generation,
            reversible: applied.reversible,
            resources: applied
                .resources
                .iter()
                .map(|owned| DeployedResource {
                    resource: owned.resource.clone(),
                    post_digest: owned.post_digest.clone(),
                })
                .collect(),
            settlement: applied.settlement.as_str().to_owned(),
        },
        receipt,
    ))
}

fn home_of(execution: &DeployExecution<'_>, target: &str) -> DeploymentHome {
    DeploymentHome::new(
        execution.state_home,
        execution.project,
        execution.package,
        target,
    )
}

fn identity_of(execution: &DeployExecution<'_>) -> DeployIdentity {
    DeployIdentity {
        project: execution.project.to_owned(),
        package: execution.package.map(str::to_owned),
    }
}

pub(super) fn order(selected: Vec<Selected<'_>>) -> Result<Vec<Selected<'_>>, DeployError> {
    let indices = dag_order(&selected, Unresolved::Refuse).map_err(|fault| match fault {
        OrderFault::Cycle { cycle } => DeployError::Cycle { cycle },
        // A profile that selects a target without its dependency is
        // already a validate error; reaching it here means the selection
        // was built programmatically, and it refuses by name rather than
        // deploying half a graph.
        OrderFault::UnknownInput { target, input } => DeployError::UnknownTarget {
            profile: target,
            target: input,
            declared: "the selected targets".to_owned(),
        },
    })?;
    let mut ordered: Vec<Option<Selected<'_>>> = selected.into_iter().map(Some).collect();
    let mut result = Vec::with_capacity(ordered.len());
    for index in indices {
        if let Some(entry) = ordered.get_mut(index).and_then(Option::take) {
            result.push(entry);
        }
    }
    Ok(result)
}

pub(super) fn displaced(selection: &MechanismSelection<'_>) -> Option<String> {
    match selection.via() {
        SelectionStep::BuiltinDefault => None,
        SelectionStep::TargetPin | SelectionStep::HostRoute => selection
            .displaced_default()
            .map(|row| row.pin().to_string()),
    }
}

pub(super) fn declared(targets: &[DeployTarget]) -> String {
    if targets.is_empty() {
        return "none declared".to_owned();
    }
    targets
        .iter()
        .map(|target| target.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "deploy/fixture.rs"]
pub(crate) mod fixture;

#[cfg(test)]
#[path = "deploy/support.rs"]
pub(crate) mod support;

#[cfg(test)]
#[path = "deploy/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "deploy/authority_tests.rs"]
mod authority_tests;

#[cfg(test)]
#[path = "deploy/preplan_tests.rs"]
mod preplan_tests;

#[cfg(test)]
#[path = "deploy/transaction_tests.rs"]
mod transaction_tests;

#[cfg(test)]
#[path = "deploy/lock_tests.rs"]
mod lock_tests;

#[cfg(test)]
#[path = "deploy/saga_tests.rs"]
mod saga_tests;

#[cfg(test)]
#[path = "deploy/prior_receipt_tests.rs"]
mod prior_receipt_tests;

#[cfg(test)]
#[path = "deploy/sidecar_tests.rs"]
mod sidecar_tests;

#[cfg(test)]
#[path = "deploy/inverse_tests.rs"]
mod inverse_tests;

#[cfg(test)]
#[path = "deploy/client_gate_tests.rs"]
mod client_gate_tests;
