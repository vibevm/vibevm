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
use vibe_wire::generated::deploy_receipt::{DeployIdentity, DeployReceipt, ReceiptStatus};

pub(crate) mod artifact;
pub(crate) mod error;
pub(crate) mod model;
pub(crate) mod plan;
pub(crate) mod preplan;
pub(crate) mod protocol;
pub(crate) mod saga;
pub(crate) mod state;
pub(crate) mod transaction;

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
use model::row;
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

/// Reverse one ALREADY-resolved selection.
fn undeploy_resolved(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
) -> Result<Vec<RemovalOutcome>, DeployError> {
    // Reverse dependency order: a target is removed before the target it
    // depends on, exactly as the saga's rollback runs.
    let resolved: Vec<&Selected<'_>> = resolved.iter().rev().collect();
    let state = DeployState::open(execution.state_home)?;
    let identity = identity_of(execution);
    let mut outcomes = Vec::with_capacity(resolved.len());
    for selected in resolved {
        // §6.3.0.9's capability has no inverse yet, and this is where the
        // gap would BITE rather than where it is convenient to mention.
        //
        // A reference owner's receipt records its logical owned member
        // (`…/opencode.json#mcp/foo`); the PHYSICAL destination it locked
        // while editing (`…/opencode.json`) lives only in the plan, which
        // no longer exists at undeploy time. Locking the receipt's logical
        // member would therefore take a lock nobody else takes, and a
        // sibling entry's deployment could edit the same document at the
        // same moment. Re-deriving the physical destination by parsing the
        // resource string would be a second grammar for an identity the
        // engine never wrote down.
        //
        // So this refuses, by name, until the engine has a DURABLE record
        // from a receipt to its lock resources. An ordinary provider —
        // every provider that exists today — locks exactly what it owns and
        // is untouched by this arm.
        if selected.provider.descriptor().reference_ownership {
            return Err(DeployError::ReferenceOwnedRemovalNotLandable {
                target: selected.target.id.clone(),
                pin: selected.pin.clone(),
            });
        }
        let home = home_of(execution, &selected.target.id);
        let receipt = state
            .read_receipt(&home)?
            .ok_or_else(|| DeployError::NoReceipt {
                target: selected.target.id.clone(),
            })?;
        let resources: Vec<String> = receipt
            .resources
            .iter()
            .map(|owned| owned.resource.clone())
            .collect();
        let _guards = state.lock_destinations(&resources)?;
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
/// transport's name, and the one deploy builtin row constructs the §7.1
/// provider. The refusal arm is not a stub — nothing is deployed by it,
/// which is the whole point of a typed refusal.
fn builtin_provider(
    handler: &ExtensionHandler,
    key: &str,
    pin: &str,
) -> Result<Box<dyn DeployProvider>, DeployError> {
    match handler {
        ExtensionHandler::Builtin { name } if name == BUILTIN_VIBE_BIN_NAME => {
            Ok(Box::new(VibeBinProvider))
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
    // Every artifact resolved, every provider planned and the whole
    // owned/lock resource set judged — before a single destination byte.
    let prepared = preplan(execution, resolved)?;
    let state = DeployState::open(execution.state_home)?;
    let identity = identity_of(execution);
    let mut outcomes: Vec<DeployOutcome> = Vec::with_capacity(resolved.len());
    // What has been applied so far, newest last — the saga's stack.
    let mut applied: Vec<(usize, DeployReceipt)> = Vec::new();
    for ((index, selected), planned) in resolved.iter().enumerate().zip(&prepared) {
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

/// One target's whole apply: lock the PREPLANNED destinations, transact.
///
/// The artifact and the plan arrive from the pre-apply epoch and are not
/// recomputed (§6.3.0.10: "Reuse the resulting values during apply; do not
/// resolve or plan a second time"). What is still this function's is the
/// lock — taken over exactly the identities the judged plan named — and the
/// staging directory, which is apply-time scratch by definition.
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
    // §6.3.0.9: the locks are the plan's PHYSICAL set, which for every
    // ordinary provider is its owned set and for a reference owner is the
    // shared document it holds while it edits its own member.
    let _guards = state.lock_destinations(&plan.lock_resources)?;
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

/// §7.2's ownership law: "A collision with state owned by another
/// deployment is an error."
///
/// The exception §7.2 grants — two deployments sharing an identical
/// content-addressed payload under a provider that supports reference
/// ownership — needs a descriptor member no provider declares at this
/// atom, so the refusal here is unconditional and the exception arrives
/// with the first provider that can honestly claim it.
///
/// The comparison goes through the SAME
/// [`path_identity_key`](vibe_safefs::path_identity_key) §6.3.0.10's
/// pre-apply judgement uses, and for the same reason: `bin/Helper` and
/// `bin/helper` are one file on the hosts this project supports, so a
/// byte-equality test would let a second deployment claim a path a
/// recorded one already owns. This is the ACROSS-RUNS half of that law —
/// the pre-apply judgement covers one selection, this covers everything the
/// state home already remembers.
///
/// Both exact spellings survive into the evidence, because the two are what
/// an operator has to reconcile: the one this target planned, and the one a
/// prior receipt recorded.
fn refuse_foreign_ownership(
    state: &DeployState,
    home: &DeploymentHome,
    target: &str,
    resources: &[String],
) -> Result<(), DeployError> {
    let planned: std::collections::BTreeMap<String, &String> = resources
        .iter()
        .map(|resource| (vibe_safefs::path_identity_key(resource), resource))
        .collect();
    for (deployment, receipt) in state.receipts()? {
        if deployment == home.id() || receipt.status == ReceiptStatus::RolledBack {
            continue;
        }
        let clashing: Vec<String> = receipt
            .resources
            .iter()
            .filter_map(|owned| {
                let identity = vibe_safefs::path_identity_key(&owned.resource);
                planned.get(&identity).map(|mine| {
                    if **mine == owned.resource {
                        (*mine).clone()
                    } else {
                        format!("`{mine}` (recorded as `{}`)", owned.resource)
                    }
                })
            })
            .collect();
        if !clashing.is_empty() {
            return Err(DeployError::OwnershipCollision {
                target: target.to_owned(),
                owner: format!("{} ({})", receipt.target, deployment),
                resources: clashing.join(", "),
            });
        }
    }
    Ok(())
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
