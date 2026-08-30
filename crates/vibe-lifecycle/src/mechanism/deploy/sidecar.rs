//! The engine-owned durable lock sidecar — §6.3.1.2's
//! `lock-resources.json`, and the whole algebra of its two generations.
//!
//! §6.3.0.9 admitted a provider that owns a LOGICAL member of a shared
//! physical destination while locking the document itself, and §7.2's record
//! list is the OWNED set — so the physical lock lived only inside a plan,
//! and a plan does not survive to recovery or undeploy time. §6.3.1.2 closes
//! exactly that gap:
//!
//! > "The strict-serde epoch-1 `lock-resources.json` is engine-owned and
//! > outside the JTD intent and receipt wires. Each binding carries
//! > generation, plan hash and exact physical lock resources. A pending
//! > binding is durable before its matching intent and therefore before the
//! > first external write; finalisation promotes it to committed only after
//! > the receipt is durable. The old committed binding is retained
//! > throughout an update, so no crash window loses the inverse lock."
//!
//! Three decisions shape this cell:
//!
//! 1. **it is an ENGINE sidecar, never a wire record.** The intent and the
//!    receipt are frozen A2 shapes with `deny_unknown_fields`; a lock
//!    resource added to either would change a published contract to record
//!    something no reader of it may act on. So this record has its own file,
//!    its own epoch and its own strict shape, exactly as the checkpoint
//!    ledger next door does and for the same reason;
//! 2. **the stored spelling is the provider's own.** Locks are already
//!    sorted and deduplicated by the shared physical identity where they are
//!    TAKEN ([`DeployState::lock_destinations`]), so normalising them here
//!    would be a second identity law over the same values — and would lose
//!    the exact spelling a refusal has to quote;
//! 3. **every transition below runs under the deployment-state lock.** They
//!    are read-modify-write pairs over one file, and the caller holds
//!    [`DeployState::lock_deployment`] before it reaches any of them.
//!
//! [`DeployState::lock_destinations`]: super::state::DeployState::lock_destinations
//! [`DeployState::lock_deployment`]: super::state::DeployState::lock_deployment

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::error::DeployError;
use super::protocol::DeployPlan;
use super::state::{DeployState, DeploymentHome};

/// The sidecar's file name inside a deployment's own directory.
pub(crate) const LOCK_RESOURCES_FILE: &str = "lock-resources.json";

/// The sidecar's schema epoch.
pub(crate) const LOCK_RESOURCES_EPOCH: u32 = 1;

/// One deployment's durable lock record: what it currently holds, and what
/// the run in flight will hold if it finishes.
///
/// Both members are optional and neither is derivable from the other: a
/// first deployment has only a pending binding, a settled one has only a
/// committed binding, and an UPDATE has both at once — which is the whole
/// point, because the crash window between them is where the inverse lock
/// would otherwise be lost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LockResources {
    /// Record schema epoch; 1 in epoch 1.
    pub(crate) schema: u32,
    /// The binding the last finalised receipt owns — what an inverse
    /// operation must lock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) committed: Option<LockBinding>,
    /// The binding the deployment in flight will own — durable before its
    /// intent, and therefore before any external write could have begun.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pending: Option<LockBinding>,
}

/// One generation's physical lock set, joined to the plan that produced it.
///
/// `plan_hash` is what makes the binding attributable: a pending binding
/// left by an earlier plan cannot be read as this plan's progress, exactly
/// as the checkpoint ledger cannot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LockBinding {
    /// The deployment generation this binding belongs to.
    pub(crate) generation: u32,
    /// The 64-hex hash of the plan that declared these locks.
    pub(crate) plan_hash: String,
    /// The exact physical lock resource spellings, in the provider's own
    /// declared order.
    pub(crate) resources: Vec<String>,
}

/// Who is asking, in the two facts every sidecar law reads.
///
/// A borrowed triple rather than the executor's `Selected`, because the laws
/// below are about a DEPLOYMENT's durable record and are called from apply,
/// from the inverse path and from the saga — three call sites that hold
/// three different things.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Ownership<'a> {
    /// The `[[deploy.target]]` id, for the refusal.
    pub(crate) target: &'a str,
    /// The exact provider pin, for the refusal.
    pub(crate) pin: &'a str,
    /// Whether the provider declared §6.3.0.9's reference ownership. `true`
    /// has NO missing-sidecar fallback anywhere below.
    pub(crate) reference: bool,
}

impl LockResources {
    /// A record that binds nothing yet.
    pub(crate) const fn empty() -> Self {
        Self {
            schema: LOCK_RESOURCES_EPOCH,
            committed: None,
            pending: None,
        }
    }

    /// Every law this record is held to, before it is written and again
    /// after it is read back.
    pub(crate) fn validate(&self) -> Result<(), DeployError> {
        if self.schema != LOCK_RESOURCES_EPOCH {
            return Err(invalid(format!(
                "schema epoch {} is not the {LOCK_RESOURCES_EPOCH} this engine writes",
                self.schema,
            )));
        }
        for (slot, binding) in [
            ("committed", self.committed.as_ref()),
            ("pending", self.pending.as_ref()),
        ] {
            if let Some(binding) = binding {
                binding.validate(slot)?;
            }
        }
        Ok(())
    }
}

impl LockBinding {
    /// Whether this binding is the one a given generation and plan produced.
    pub(crate) fn matches(&self, generation: u32, plan_hash: &str) -> bool {
        self.generation == generation && self.plan_hash == plan_hash
    }

    /// One binding's own laws: a real plan hash, and a resource list whose
    /// every spelling is usable and names a distinct physical destination.
    fn validate(&self, slot: &str) -> Result<(), DeployError> {
        if self.plan_hash.len() != 64
            || !self
                .plan_hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(invalid(format!(
                "the {slot} binding's plan hash is not 64 lowercase hex",
            )));
        }
        let mut seen: BTreeMap<String, &String> = BTreeMap::new();
        for resource in &self.resources {
            if resource.trim().is_empty() || resource.chars().any(char::is_control) {
                return Err(invalid(format!(
                    "the {slot} binding names a blank or control-bearing lock resource",
                )));
            }
            // The same Unicode-9 physical identity §6.3.0.10's pre-apply
            // judgement uses: two spellings of one destination in one
            // binding would take one lock and claim two.
            if let Some(first) = seen.insert(vibe_safefs::path_identity_key(resource), resource) {
                return Err(invalid(format!(
                    "the {slot} binding names one physical destination twice: `{first}` and \
                     `{resource}`",
                )));
            }
        }
        Ok(())
    }
}

/// Publish `pending = current` while RETAINING `committed` — §6.3.1.2's
/// second step, and the one write that has to precede the intent.
///
/// Retention is the law's whole content: an update that replaced the
/// committed binding here would leave a crash window in which the previous
/// generation's destinations are still deployed and no durable record says
/// which they were.
pub(crate) fn stage_pending(
    state: &DeployState,
    home: &DeploymentHome,
    generation: u32,
    plan_hash: &str,
    resources: &[String],
) -> Result<(), DeployError> {
    let mut record = state
        .read_lock_resources(home)?
        .unwrap_or(LockResources::empty());
    record.pending = Some(LockBinding {
        generation,
        plan_hash: plan_hash.to_owned(),
        resources: resources.to_vec(),
    });
    state.write_lock_resources(home, &record)
}

/// Promote the pending binding of one generation to committed, retaining no
/// pending — §6.3.1.2's "finalisation promotes it to committed only after
/// the receipt is durable".
///
/// A pending binding that is NOT this generation's is left exactly where it
/// is: it belongs to a deployment this call knows nothing about, and
/// promoting it would hand this receipt somebody else's destinations.
pub(crate) fn promote(
    state: &DeployState,
    home: &DeploymentHome,
    generation: u32,
    plan_hash: &str,
) -> Result<(), DeployError> {
    let mut record = state.read_lock_resources(home)?.ok_or_else(|| {
        invalid(format!(
            "the pending binding for generation {generation} and plan `{plan_hash}` is absent"
        ))
    })?;
    let pending = record.pending.take().ok_or_else(|| {
        invalid(format!(
            "the pending binding for generation {generation} and plan `{plan_hash}` is absent"
        ))
    })?;
    if !pending.matches(generation, plan_hash) {
        return Err(invalid(format!(
            "the pending binding names generation {} and plan `{}`, not generation {generation} \
             and plan `{plan_hash}`",
            pending.generation, pending.plan_hash,
        )));
    }
    record.committed = Some(pending);
    state.write_lock_resources(home, &record)
}

/// Clear ONLY the pending binding of one generation — the stale-intent
/// decision's own transition.
///
/// §6.3.1.3: "stale retirement clears only that pending generation." The
/// committed binding survives untouched, because the deployment it describes
/// is still deployed.
pub(crate) fn clear_pending(
    state: &DeployState,
    home: &DeploymentHome,
    generation: u32,
    plan_hash: &str,
) -> Result<(), DeployError> {
    let Some(mut record) = state.read_lock_resources(home)? else {
        return Ok(());
    };
    if !record
        .pending
        .as_ref()
        .is_some_and(|pending| pending.matches(generation, plan_hash))
    {
        return Ok(());
    }
    record.pending = None;
    state.write_lock_resources(home, &record)
}

/// Clear the committed binding of one generation — §6.3.1.3's "successful
/// inverse clears committed ownership after the rolled-back receipt is
/// durable".
///
/// Keyed on the generation rather than cleared outright: a binding recorded
/// for a generation this receipt is not is not this reversal's to drop.
pub(crate) fn clear_committed(
    state: &DeployState,
    home: &DeploymentHome,
    generation: u32,
) -> Result<(), DeployError> {
    let Some(mut record) = state.read_lock_resources(home)? else {
        return Ok(());
    };
    if record
        .committed
        .as_ref()
        .is_none_or(|committed| committed.generation != generation)
    {
        return Ok(());
    }
    record.committed = None;
    state.write_lock_resources(home, &record)
}

/// The bindings this apply must hold, with the legacy fallback materialised
/// when an interrupted ORDINARY deployment needs one.
///
/// Two §6.3.1.4 sentences, in one place because they decide one thing:
///
/// > "An ordinary non-reference receipt created before the sidecar may fall
/// > back to its owned resources because its descriptor proves lock set
/// > equals owned set. A reference owner never has that fallback and never
/// > reconstructs a physical lock by parsing a logical resource string."
///
/// The fallback is MATERIALISED rather than merely computed, because the
/// recovery that follows takes the physical locks from this record and
/// §6.3.1.3 requires the binding to exist before it runs. The derivation is
/// typed — the intent journal's own planned resource list — and never parses
/// a resource string.
pub(crate) fn settle_bindings(
    state: &DeployState,
    home: &DeploymentHome,
    owner: Ownership<'_>,
) -> Result<LockResources, DeployError> {
    let mut record = state
        .read_lock_resources(home)?
        .unwrap_or(LockResources::empty());
    let Some(intent) = state.read_intent(home)? else {
        return Ok(record);
    };
    if record
        .pending
        .as_ref()
        .is_some_and(|pending| pending.matches(intent.target.generation, &intent.plan_hash))
    {
        return Ok(record);
    }
    if owner.reference {
        return Err(DeployError::LockSidecarMissing {
            target: owner.target.to_owned(),
            pin: owner.pin.to_owned(),
            sidecar: LOCK_RESOURCES_FILE,
            operation: "settling an interrupted deployment",
        });
    }
    // This is both the interrupted-intent legacy repair and the benign
    // receipt+intent legacy repair. In the benign case finalisation already
    // happened, but materialising the typed ordinary fallback lets the
    // settlement perform the same pending-to-committed transition every new
    // deployment uses instead of retiring with an unrecorded inverse lock.
    record.pending = Some(LockBinding {
        generation: intent.target.generation,
        plan_hash: intent.plan_hash.clone(),
        resources: intent
            .resources
            .iter()
            .map(|planned| planned.resource.clone())
            .collect(),
    });
    state.write_lock_resources(home, &record)?;
    Ok(record)
}

/// The physical destinations an INVERSE operation must hold — §6.3.1.3's
/// "Apply, recovery, saga rollback and undeploy take the deployment-id lock,
/// then the union of current, committed and pending destination locks".
///
/// For a reference owner the committed binding is the only answer there is:
/// its receipt records `…/opencode.json#mcp/foo`, and turning that back into
/// `…/opencode.json` would be a second grammar for an identity the engine
/// never wrote down. Missing or belonging to another generation, it refuses.
///
/// For every ordinary provider the receipt's own owned set IS the physical
/// set, because [`validate_lock_set`] proved the two equal before the plan
/// was ever applied — so a pre-sidecar receipt stays removable.
///
/// [`validate_lock_set`]: super::preplan::validate_lock_set
pub(crate) fn inverse_locks(
    record: Option<&LockResources>,
    receipt: &DeployReceipt,
    owner: Ownership<'_>,
) -> Result<Vec<String>, DeployError> {
    let Some(record) = record else {
        if owner.reference {
            return Err(DeployError::LockSidecarMissing {
                target: owner.target.to_owned(),
                pin: owner.pin.to_owned(),
                sidecar: LOCK_RESOURCES_FILE,
                operation: "reversing a reference-owned deployment",
            });
        }
        // The only fallback: the sidecar is wholly absent, so this may be an
        // ordinary receipt from before the sidecar epoch. Its descriptor
        // proved lock set equals owned set before it was ever applied.
        return Ok(receipt
            .resources
            .iter()
            .map(|owned| owned.resource.clone())
            .collect());
    };

    let mut locks = match record.committed.as_ref() {
        Some(binding) if binding.generation == receipt.generation => binding.resources.clone(),
        Some(binding) => {
            return Err(DeployError::LockSidecarMismatch {
                target: owner.target.to_owned(),
                pin: owner.pin.to_owned(),
                sidecar: LOCK_RESOURCES_FILE,
                recorded: binding.generation,
                wanted: receipt.generation,
            });
        }
        None if receipt.resources.is_empty() => Vec::new(),
        None => {
            return Err(DeployError::RecordInvalid {
                record: LOCK_RESOURCES_FILE,
                reason: format!(
                    "the receipt for generation {} owns resources but the committed binding is \
                     absent",
                    receipt.generation,
                ),
            });
        }
    };
    // An interrupted update may have touched its pending destinations. An
    // inverse has no current plan, so §6.3.1.3's canonical union is exactly
    // committed plus pending. Holding only committed would let a sibling
    // deployment edit the in-flight destination while this one reverses.
    if let Some(pending) = record.pending.as_ref() {
        locks.extend(pending.resources.iter().cloned());
    }
    Ok(locks)
}

/// The canonical union of one plan's, the committed and the pending
/// destination locks.
///
/// Handed to [`DeployState::lock_destinations`], which owns the canonical
/// ORDER (the sorted identity-derived lock name) and the deduplication. This
/// function's only job is to leave nothing out.
///
/// [`DeployState::lock_destinations`]: super::state::DeployState::lock_destinations
pub(crate) fn union(plan: &DeployPlan, record: &LockResources) -> Vec<String> {
    let mut all = plan.lock_resources.clone();
    for binding in [record.committed.as_ref(), record.pending.as_ref()]
        .into_iter()
        .flatten()
    {
        all.extend(binding.resources.iter().cloned());
    }
    all
}

/// One broken sidecar law, in the engine's own record refusal.
fn invalid(reason: String) -> DeployError {
    DeployError::RecordInvalid {
        record: LOCK_RESOURCES_FILE,
        reason,
    }
}
