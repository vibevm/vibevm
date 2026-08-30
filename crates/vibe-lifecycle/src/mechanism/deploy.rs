//! The deploy phase's mechanism executor — §7.0's engine, and the third
//! sibling of the build and package executors.
//!
//! Built from the same parts as its two siblings on purpose: one resolver
//! ([`resolve_mechanism`]), one dependency walk, one containment cell, one
//! record reader. There is no `if builtin` shortcut here either — the
//! executor asks who services the target's logical key and only then looks
//! at the selected row's handler, so a host that routes `deploy:vibe-bin`
//! to a plugin gets the plugin's refusal and demonstrably NOT a builtin.
//!
//! Four things are this phase's own:
//!
//! 1. **the profile selection is DATA.** §7.0.5 resolves it once, in the
//!    command layer that owns flags, and this executor consumes it. There
//!    is no environment read, no `default_profile` walk and no
//!    exactly-one rule anywhere below this line — the engine cannot
//!    re-derive what it was told;
//! 2. **every selected plan is made before the first apply.** §6.3.0.10's
//!    pre-apply epoch is a transaction PREREQUISITE, not a preview: it
//!    resolves every artifact, calls every provider's `plan`, judges the
//!    whole owned/lock resource set through the shared physical identity,
//!    and only then may target 0 touch anything. Apply reuses exactly what
//!    it produced ([`preplan`]);
//! 3. **the destination is transacted, not written.** Every applied target
//!    goes through [`transaction`], whose order is §7.2's;
//! 4. **a failed multi-target run is a saga.** Already-applied REVERSIBLE
//!    targets are rolled back in reverse order; an irreversible one stays
//!    visible as partial, and the run reports both lists rather than a
//!    success.
//!
//! [`resolve_mechanism`]: vibe_extension_registry::resolve_mechanism

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{DeployTarget, ExtensionHandler};
use vibe_extension_registry::{MechanismSelection, SelectionStep, resolve_mechanism};
use vibe_wire::generated::deploy_receipt::{DeployIdentity, DeployReceipt};

pub(crate) mod artifact;
pub(crate) mod error;
pub(crate) mod inverse;
pub(crate) mod ledger;
pub(crate) mod model;
pub(crate) mod observation;
pub(crate) mod ownership;
pub(crate) mod plan;
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

use super::order::{GraphNode, OrderFault, Unresolved, dag_order};
use super::vibebin::VibeBinProvider;
use super::{BUILTIN_VIBE_BIN_NAME, DeployProvider, DeployTargetRequest};
use skill::SkillDeployProvider;
// The inverse path lives in its own cell and is re-exported here, so
// `undeploy_targets` and every test still spell it one way.
pub(crate) use inverse::undeploy_resolved;
use model::row;
use ownership::{ownership_of, refuse_changed_ownership, refuse_foreign_ownership};
use preplan::{Preplanned, preplan};
use saga::unwind;
use state::{DeployState, DeploymentHome};
use transaction::Transaction;

/// Deploy every selected target, in dependency order.
///
/// The canonical use is on [`DeployExecution`]. A selection with no
/// targets deploys nothing and says so, which is what makes an ordinary
/// `vibe deploy` on a project that declares no deploy section
/// byte-identical to the historical run.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn execute_deploy_targets(
    execution: &DeployExecution<'_>,
) -> Result<Vec<DeployOutcome>, DeployError> {
    let resolved = resolve_selection(execution)?;
    apply_selection(execution, &resolved)
}

/// Reverse every selected target, in reverse dependency order.
///
/// §7.2: "`undeploy` removes only receipt-owned state and refuses to erase
/// a path changed after deployment without an explicit force/recovery
/// decision." The drift refusal is the ENGINE's and fires before the
/// provider is asked to remove anything.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn undeploy_targets(
    execution: &DeployExecution<'_>,
) -> Result<Vec<RemovalOutcome>, DeployError> {
    let resolved = resolve_selection(execution)?;
    undeploy_resolved(execution, &resolved)
}

/// Every deployment this machine's state home records, in deployment-id
/// order.
///
/// Receipt facts only — §7.2's record list contains no secret-bearing
/// member, and this projection adds none.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn list_deployments(state_home: &Path) -> Result<Vec<DeploymentRow>, DeployError> {
    let state = DeployState::open(state_home)?;
    Ok(state
        .receipts()?
        .into_iter()
        .map(|(deployment, receipt)| row(deployment, &receipt))
        .collect())
}

/// One resolved target: the row, the provider, and the routing decision.
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

/// Resolve the selection's targets and their providers, in dependency
/// order.
///
/// This is the executor's ONE selection path (§7.0.2). Every caller —
/// apply, plan and undeploy — goes through it, so the three surfaces
/// cannot disagree about who services a key.
fn resolve_selection<'a>(
    execution: &DeployExecution<'a>,
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
        let selection = resolve_mechanism(
            execution.registry,
            &target.mechanism,
            target.provider.as_ref(),
            execution.routes,
        )?;
        let row = selection.row();
        let pin = row.pin().to_string();
        let key = target.mechanism.to_string();
        let provider = builtin_provider(row.handler(), &key, &pin)?;
        let descriptor = provider.descriptor();
        // §3.2: "`plan` is mandatory for deploy providers."
        if !descriptor.implements(super::ProviderOperation::Plan) {
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

/// The closed builtin dispatch of the deploy role.
///
/// §7.0.2 in one function: a non-builtin handler refuses by the unlanded
/// transport's name, the `#vibe-bin` row constructs the §7.1 provider, and
/// the three §6.3.0.5 skill rows construct ONE closed provider
/// parameterised by its client — the same lesson the three projection
/// rows landed: what differs between the three is DATA, not behaviour, so
/// it lives in [`SkillClient`] and the adapter is written once. The
/// refusal arms are not stubs — nothing is deployed by them, which is the
/// whole point of a typed refusal.
fn builtin_provider(
    handler: &ExtensionHandler,
    key: &str,
    pin: &str,
) -> Result<Box<dyn DeployProvider>, DeployError> {
    use skill::SkillClient;
    match handler {
        ExtensionHandler::Builtin { name } if name == BUILTIN_VIBE_BIN_NAME => {
            Ok(Box::new(VibeBinProvider))
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

/// Apply one already-resolved selection: §6.3.0.10's pre-apply epoch, then
/// the §7.2 saga.
///
/// Separated from [`resolve_selection`] so the saga's own laws — reverse
/// rollback, the irreversible partial — are provable with hermetic
/// providers. Selection still happens in exactly one place; this half
/// receives its result and never re-derives it.
///
/// The [`preplan`] call is INSIDE this function rather than beside it
/// because that is what makes its promise checkable: there is no order of
/// calls a caller could choose in which target 0 applies before target 1
/// has been planned and judged.
pub(crate) fn apply_selection(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
) -> Result<Vec<DeployOutcome>, DeployError> {
    if resolved.is_empty() {
        return Ok(Vec::new());
    }
    // Every artifact resolved, every provider planned, every prior receipt
    // read and the whole owned/lock resource set judged — before a single
    // destination byte, and before the state home is even created.
    let prepared = preplan(execution, resolved)?;
    apply_prepared(execution, resolved, &prepared)
}

/// Apply what the pre-apply epoch already prepared.
///
/// Separated from [`apply_selection`] so §6.3.1.1's recheck is provable:
/// the window it closes is *between* preplanning and applying, and a
/// function that owns both ends has no such window a test can open. Every
/// shipped caller still goes through [`apply_selection`], which composes the
/// two in the only order the law admits.
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
    // What has been applied so far, newest last — the saga's stack.
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
    Ok(outcomes)
}

/// One target's whole apply: lock the deployment, lock the PREPLANNED and
/// recorded destinations, re-read prior ownership, transact.
///
/// The artifact and the plan arrive from the pre-apply epoch and are not
/// recomputed (§6.3.0.10: "Reuse the resulting values during apply; do not
/// resolve or plan a second time"). What is still this function's is the
/// lock ORDER — §6.3.1.3's deployment-state lock, then the canonical union
/// of the current plan's, the committed and the pending destinations — the
/// §6.3.1.1 recheck those locks make meaningful, and the staging directory,
/// which is apply-time scratch by definition.
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
    // §6.3.1.3, in the order it states: the stable deployment-state lock
    // FIRST, so every sidecar, intent and receipt transition below is this
    // run's alone.
    let _deployment = state.lock_deployment(&home)?;
    // Prior ownership is the first state judgement under that lock. In
    // particular it precedes the legacy pending-binding repair below: a
    // plan made against a receipt that changed may not write even an
    // engine-owned sidecar before it refuses.
    refuse_changed_ownership(state, &home, selected, planned)?;
    // Then the durable bindings — including the typed fallback an
    // interrupted ORDINARY deployment needs before its recovery may take
    // the physical locks its journal implies.
    let bindings = sidecar::settle_bindings(state, &home, ownership_of(selected))?;
    // §6.3.0.9: the locks are the plan's PHYSICAL set, which for every
    // ordinary provider is its owned set and for a reference owner is the
    // shared document it holds while it edits its own member — UNION the
    // committed and pending bindings, because an update reconciles a new
    // destination set while the previous one is still deployed.
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
        // Deliberately `None`: the recovery intent is settlement-
        // reachability EVIDENCE for a plan, and this is the apply-time
        // request. The locked occupant recheck inside `apply` stays
        // receipt-only; the transaction settles whatever journal is
        // unretired under its own plan-hash law before this request is
        // ever built into a write.
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

/// This deployment's own directory inside the state home.
fn home_of(execution: &DeployExecution<'_>, target: &str) -> DeploymentHome {
    DeploymentHome::new(
        execution.state_home,
        execution.project,
        execution.package,
        target,
    )
}

/// The project/package identity every record of this run is keyed under.
fn identity_of(execution: &DeployExecution<'_>) -> DeployIdentity {
    DeployIdentity {
        project: execution.project.to_owned(),
        package: execution.package.map(str::to_owned),
    }
}

/// The selection, in dependency order.
fn order(selected: Vec<Selected<'_>>) -> Result<Vec<Selected<'_>>, DeployError> {
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

/// The displaced builtin default, when a replacement really replaced one.
fn displaced(selection: &MechanismSelection<'_>) -> Option<String> {
    match selection.via() {
        SelectionStep::BuiltinDefault => None,
        SelectionStep::TargetPin | SelectionStep::HostRoute => selection
            .displaced_default()
            .map(|row| row.pin().to_string()),
    }
}

/// The declared target ids, for a refusal that names what IS available.
fn declared(targets: &[DeployTarget]) -> String {
    if targets.is_empty() {
        return "none declared".to_owned();
    }
    targets
        .iter()
        .map(|target| target.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// The hermetic provider every §7.2 law is proven against, and the world it
// is proven inside — two cells, because they answer two questions.
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

// §6.3.1's own three laws, one cell each: the injected prior ownership, the
// durable lock sidecar's crash windows, and the inverse that reads it.
#[cfg(test)]
#[path = "deploy/prior_receipt_tests.rs"]
mod prior_receipt_tests;

#[cfg(test)]
#[path = "deploy/sidecar_tests.rs"]
mod sidecar_tests;

#[cfg(test)]
#[path = "deploy/inverse_tests.rs"]
mod inverse_tests;
