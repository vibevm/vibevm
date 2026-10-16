//! Deploy provider selection, transaction, inverse, and restart admission.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{DeployTarget, ExtensionHandler, MechanismRole};
use vibe_extension_registry::{MechanismSelection, SelectionStep, resolve_mechanism};
use vibe_wire::behaviour::deploy_records::validate_operation_binding;
use vibe_wire::generated::deploy_receipt::{DeployIdentity, DeployReceipt};
use vibe_wire::generated::shared::EnvironmentOperationBinding;

pub(crate) mod artifact;
pub(crate) mod binding;
pub(crate) mod bridge;
pub mod client_commands;
pub(crate) mod derivative;
pub(crate) mod error;
pub(crate) mod executable_observation;
pub mod generic;
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
pub use plugin::client_state::{ClientStateBinding, ClientStateOwner, ClientStateWitness};

use super::error::deploy::native_transport;
use super::order::{GraphNode, OrderFault, Unresolved, dag_order};
use super::vibebin::VibeBinProvider;
use super::vibebin::bound::BoundVibeBinStore;
use super::vibebin::bound_provider::BoundVibeBinProvider;
use super::{
    BUILTIN_VIBE_BIN_NAME, BUILTIN_VIBE_OPT_LAUNCHER_NAME, DeployProvider, DeployTargetRequest,
};
use opt_launcher::VibeOptLauncherProvider;
use plugin::{ClientPluginProvider, PluginClient};
use skill::SkillDeployProvider;
// The inverse path lives in its own cell and is re-exported here.
use binding::AdmittedLocalDeployBinding;
pub(crate) use inverse::undeploy_resolved;
use inverse_resume::resume_inverses;
use model::row;
use ownership::{
    ownership_of, refuse_changed_ownership, refuse_foreign_ownership,
    refuse_foreign_ownership_bound,
};
use preplan::{Preplanned, preplan};
use saga::unwind;
use state::{DeployState, DeploymentHome};
use transaction::Transaction;
use view::DeployStateView;

/// Deploy every selected target in dependency order.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn execute_deploy_targets(
    execution: &DeployExecution<'_>,
) -> Result<Vec<DeployOutcome>, DeployError> {
    let resolved = resolve_selection(execution)?;
    apply_selection(execution, &resolved)
}

/// Explicit opt-in reach for an environment-bound deploy. A shaped binding
/// does not carry a pinned storage root or host effect grant. Refuse BEFORE
/// resolution, inverse settlement, provider planning or state creation until
/// the storage owner admits those capabilities. The legacy entry point above
/// keeps its original key and behavior.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING",
    r = 3
)]
pub fn execute_bound_deploy_targets(
    _execution: &DeployExecution<'_>,
    binding: &EnvironmentOperationBinding,
) -> Result<Vec<DeployOutcome>, DeployError> {
    validate_operation_binding(binding)
        .map_err(|_| DeployError::BindingMismatch { record: "request" })?;
    Err(DeployError::BoundRootAdmissionUnavailable {
        binding: binding.binding_id.clone(),
    })
}

/// First effectful environment route. Only a trusted host can supply the
/// non-cloneable admission and a fresh generation observation. The public
/// wire-only entry above deliberately cannot reach this function.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-BOUND-BUILTIN",
    r = 1
)]
pub(crate) fn execute_host_admitted_bound_vibe_bin(
    execution: &DeployExecution<'_>,
    request_binding: &EnvironmentOperationBinding,
    admitted: AdmittedLocalDeployBinding,
    principal: &str,
    current_generation: &mut dyn FnMut() -> Result<Option<String>, DeployError>,
) -> Result<Vec<DeployOutcome>, DeployError> {
    execute_host_admitted_bound_vibe_bin_with_hook(
        execution,
        request_binding,
        admitted,
        principal,
        current_generation,
        None,
    )
}

fn execute_host_admitted_bound_vibe_bin_with_hook(
    execution: &DeployExecution<'_>,
    request_binding: &EnvironmentOperationBinding,
    admitted: AdmittedLocalDeployBinding,
    principal: &str,
    current_generation: &mut dyn FnMut() -> Result<Option<String>, DeployError>,
    mut before_acquire: Option<
        &mut dyn FnMut(
            &DeploymentHome,
            &DeployTargetRequest<'_>,
            &crate::mechanism::deploy::protocol::DeployPlan,
            &str,
        ) -> Result<(), DeployError>,
    >,
) -> Result<Vec<DeployOutcome>, DeployError> {
    let [target_id] = execution.selection.targets.as_slice() else {
        return Err(DeployError::BoundScopeUnsupported {
            reason: "the first bound route requires exactly one selected target".into(),
        });
    };
    let current = current_generation()?;
    admitted
        .preflight(execution, request_binding, principal, current.as_deref())
        .map_err(|error| DeployError::BoundGrantChanged {
            target: target_id.clone(),
            reason: error.to_string(),
        })?;
    let target = execution
        .targets
        .iter()
        .find(|candidate| candidate.id == *target_id)
        .ok_or_else(|| DeployError::UnknownTarget {
            profile: execution.selection.profile.clone(),
            target: target_id.clone(),
            declared: declared(execution.targets),
        })?;
    if target.mechanism.to_string() != "deploy:vibe-bin"
        || target
            .depends_on
            .as_ref()
            .is_some_and(|deps| !deps.is_empty())
    {
        return Err(DeployError::BoundScopeUnsupported {
            reason: "the first bound route accepts only an independent deploy:vibe-bin target"
                .into(),
        });
    }
    let selection = resolve_mechanism(
        execution.registry,
        &target.mechanism,
        target.provider.as_ref(),
        execution.routes,
    )?;
    if !matches!(selection.row().handler(), ExtensionHandler::Builtin { name } if name == BUILTIN_VIBE_BIN_NAME)
    {
        return Err(DeployError::BoundScopeUnsupported {
            reason: "the selected provider is not the builtin vibe-bin handler".into(),
        });
    }
    let pin = selection.row().pin().to_string();
    let via = selection.via().to_string();
    let displaced_default = displaced(&selection);
    let binding = admitted.binding().clone();
    let home = DeploymentHome::new_bound(
        execution.state_home,
        execution.project,
        execution.package,
        target_id,
        &binding,
    )?;
    let state_view = DeployStateView::open_bound(admitted.settings())?;
    for (record, present) in [
        ("intent", state_view.read_intent(&home)?.is_some()),
        ("receipt", state_view.read_receipt(&home)?.is_some()),
        (
            "lock-resources",
            state_view.read_lock_resources(&home)?.is_some(),
        ),
    ] {
        if present {
            return Err(DeployError::BoundPriorGenerationUnavailable {
                target: target_id.clone(),
                record,
            });
        }
    }
    let (artifact, bound_artifact) =
        artifact::resolve_bound_project_artifact(admitted.project(), target)?;
    let preplan_provider = BoundVibeBinProvider::for_preplan(admitted.settings(), &bound_artifact);
    let request = DeployTargetRequest {
        target,
        profile: &execution.selection.profile,
        project_root: execution.project_root,
        settings_root: execution.settings_root,
        user_home: execution.user_home,
        clients: execution.clients,
        prior_receipt: None,
        recovery_intent: None,
        artifact: Some(&artifact),
        staging: None,
    };
    let plan = preplan_provider.plan(&request)?;
    // The descriptor belongs to the selected builtin; the plan was produced
    // through its pinned bound adapter. Reuse the incumbent resource laws.
    let selected = Selected {
        target,
        provider: Box::new(VibeBinProvider),
        pin: pin.clone(),
        via: selection.via(),
        displaced: displaced_default.clone(),
    };
    preplan::validate_lock_set(&selected, &plan)?;
    preplan::judge_resources(&[preplan::Participant {
        selected: &selected,
        plan: &plan,
    }])?;

    let current = current_generation()?;
    if let Some(callback) = before_acquire.as_deref_mut() {
        callback(&home, &request, &plan, &pin)?;
    }
    let locked = admitted
        .acquire_destination(current.as_deref())
        .map_err(|error| DeployError::BoundGrantChanged {
            target: target_id.clone(),
            reason: error.to_string(),
        })?;
    let state = DeployState::open_bound(locked.settings())?;
    state.verify_bound_home(locked.settings())?;
    let _deployment = state.lock_deployment(&home)?;
    for (record, present) in [
        ("intent", state.read_intent(&home)?.is_some()),
        ("receipt", state.read_receipt(&home)?.is_some()),
        (
            "lock-resources",
            state.read_lock_resources(&home)?.is_some(),
        ),
        ("inverse", state.read_inverse(&home)?.is_some()),
    ] {
        if present {
            return Err(DeployError::BoundPriorGenerationUnavailable {
                target: target_id.clone(),
                record,
            });
        }
    }
    let _resource_guards = state.lock_destinations(&plan.lock_resources)?;
    let resources: Vec<String> = plan
        .resources
        .iter()
        .map(|owned| owned.resource.clone())
        .collect();
    refuse_foreign_ownership_bound(&state, &home, target_id, &resources)?;
    let store = BoundVibeBinStore::from_locked_admission(&locked);
    let provider = BoundVibeBinProvider::for_locked(&store, &bound_artifact);
    let locked_plan = provider.plan(&request)?;
    if locked_plan != plan {
        return Err(DeployError::BoundGrantChanged {
            target: target_id.clone(),
            reason: "the bound provider plan changed after the physical lock was acquired".into(),
        });
    }
    let identity = identity_of(execution);
    let transaction = Transaction {
        state: &state,
        home: &home,
        identity: &identity,
        provider_pin: &pin,
        scope: provider.descriptor().scope(),
        created_at: execution.created_at,
    };
    let applied =
        transaction.apply_bound(&provider, &request, &plan, &locked, current_generation)?;
    Ok(vec![DeployOutcome {
        target: target_id.clone(),
        mechanism: target.mechanism.to_string(),
        provider: pin,
        via,
        displaced_default,
        generation: applied.generation,
        reversible: applied.reversible,
        resources: applied
            .resources
            .into_iter()
            .map(|owned| DeployedResource {
                resource: owned.resource,
                post_digest: owned.post_digest,
            })
            .collect(),
        settlement: applied.settlement.as_str().to_owned(),
    }])
}

pub(crate) fn execute_prepared_deploy_targets(
    execution: &DeployExecution<'_>,
    prepared: &crate::native::PreparedNativeMechanisms,
) -> Result<Vec<DeployOutcome>, DeployError> {
    let resolved = resolve_selection_with(execution, Some(prepared), false)?;
    apply_selection(execution, &resolved)
}

pub(crate) fn plan_prepared_deploy_targets(
    execution: &DeployExecution<'_>,
    _prepared: &crate::native::PreparedNativeMechanisms,
) -> Result<Vec<DeployPlanReport>, DeployError> {
    // A prepared image is not a prepared Plan result. Never admit/load an
    // image just because a read-only caller supplied the build-fence sidecar.
    plan::plan_deploy_targets(execution)
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
    let resolved = resolve_selection_with(execution, Some(prepared), false)?;
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
    resolve_selection_with(execution, None, false)
}

fn resolve_query_selection<'a>(
    execution: &DeployExecution<'a>,
) -> Result<Vec<Selected<'a>>, DeployError> {
    resolve_selection_with(execution, None, true)
}

fn resolve_selection_with<'a>(
    execution: &DeployExecution<'a>,
    prepared: Option<&crate::native::PreparedNativeMechanisms>,
    query: bool,
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
            if query {
                return Err(super::MechanismError::from(super::error::DeployProviderError::NeedsPreparation {
                    target: target.id.clone(), reason: vibe_wire::generated::adapter_plan::e1::error::ErrorReason::NativePlanCaptureMissing,
                }).into());
            }
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
    prepared
        .map(|value| value.binding_for(MechanismRole::Deploy, &target.id, &target.mechanism))
        .transpose()
        .map_err(|(pin, reason)| native_transport(&target.id, pin, "admit", reason).into())
        .map(Option::flatten)
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
) -> Result<Box<dyn DeployProvider + Send + Sync>, DeployError> {
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

#[cfg(test)]
#[path = "deploy/bound_tests.rs"]
mod bound_tests;

#[cfg(test)]
#[path = "deploy/binding_tests.rs"]
mod binding_tests;

#[cfg(test)]
#[path = "deploy/bound_effect_tests.rs"]
mod bound_effect_tests;

#[cfg(test)]
#[path = "deploy/bridge_tests.rs"]
mod bridge_tests;

#[cfg(test)]
mod m_08_a_1_cases;

#[cfg(test)]
mod m_08_b_1_cases;

mod prepared_query;
pub use prepared_query::{
    PreparedDeployQuery, PreparedDeployRow, prepare_deploy_query,
    prepare_deploy_query_with_client_state_owner,
};

pub mod client_capabilities;

/// Host-supplied common exclusion capability for deployments participating in
/// generic handoff. Acquire arena before all incumbent deployment/destination
/// leases; the arena confers no provider or destination authority.
pub struct CoordinatedDeploy {
    arena: vibe_safefs::Project,
    identity: String,
}
impl CoordinatedDeploy {
    pub fn new(arena: vibe_safefs::Project) -> Result<Self, DeployError> {
        let identity = arena
            .root_correlation_token()
            .map_err(|e| DeployError::RecordInvalid {
                record: "coordination arena",
                reason: e.to_string(),
            })?;
        Ok(Self { arena, identity })
    }
    fn current_arena(&self) -> Result<(), DeployError> {
        if vibe_safefs::Project::open(self.arena.root_path())
            .and_then(|named| named.root_correlation_token())
            .map_err(|e| DeployError::RecordInvalid {
                record: "coordination arena",
                reason: e.to_string(),
            })?
            != self.identity
        {
            return Err(DeployError::BindingMismatch {
                record: "coordination arena",
            });
        }
        Ok(())
    }
    fn lock(&self) -> Result<vibe_safefs::LockGuard, DeployError> {
        self.current_arena()?;
        let guard = self
            .arena
            .lock("deploy-physical-coordination.lock")
            .map_err(|e| DeployError::RecordInvalid {
                record: "coordination arena",
                reason: e.to_string(),
            })?;
        self.current_arena()?;
        Ok(guard)
    }
    pub fn apply(
        &self,
        execution: &DeployExecution<'_>,
    ) -> Result<Vec<DeployOutcome>, DeployError> {
        let _arena = self.lock()?;
        execute_deploy_targets(execution)
    }
    pub fn undeploy(
        &self,
        execution: &DeployExecution<'_>,
    ) -> Result<Vec<RemovalOutcome>, DeployError> {
        let _arena = self.lock()?;
        undeploy_targets(execution)
    }
}

pub mod client_deploy;
