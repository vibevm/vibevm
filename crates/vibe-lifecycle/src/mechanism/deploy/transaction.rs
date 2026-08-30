//! The §7.2 transaction — the atom's core, and the one cell that owns the
//! order of durable events.
//!
//! §7.2 verbatim, each sentence a step below:
//!
//! > Before apply, VibeVM atomically writes a durable intent journal
//! > containing the plan hash, prior receipt generation, every planned
//! > resource and its desired digest. Apply checkpoints completed
//! > operations without storing secrets. After independent verify, the
//! > finalized receipt is written and the intent is retired. Apply uses a
//! > per-destination lock and staging where the destination supports atomic
//! > replacement.
//!
//! > On restart, an intent without a matching final receipt enters
//! > `recover`: if all observed resources match either the prior or desired
//! > digest, the idempotent provider rolls forward and finalizes; a third
//! > digest means concurrent/user mutation, so recovery refuses and names
//! > the exact resources. A receipt plus its still-present matching intent
//! > is a benign crash after finalization: retire the intent.
//!
//! The ORDER is the whole point, so it is stated once, here, and no caller
//! may reorder it: a provider is handed the destination only after the
//! intent is durable, and the receipt is written only after an INDEPENDENT
//! observation of the destination — never from what apply claimed.
//!
//! One case §7.2 does not spell, decided here and named: an unretired
//! intent whose plan hash is NOT the plan now being applied. Its
//! three-digest law still runs in full — a foreign mutation refuses exactly
//! as it would otherwise — but the roll-forward does not, because rolling a
//! destination forward to a desired state nobody wants any more would be
//! this engine inventing an intent. The stale journal is retired, the fact
//! is reported, and the new plan applies over a destination the digest law
//! just proved is at either its prior or its old desired state.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use sha2::{Digest, Sha256};
use vibe_wire::behaviour::deploy_records::{INTENT_EPOCH, RECEIPT_EPOCH};
use vibe_wire::generated::deploy_intent::{
    DeployIntent, DeployTargetIdentity, PlannedResource as IntentResource,
};
use vibe_wire::generated::deploy_receipt::{
    DeployIdentity, DeployReceipt, OwnedResource, ProviderIdentity, ReceiptStatus, Rfc3339Timestamp,
};

use super::error::DeployError;
use super::protocol::{DeployPlan, ObservedResource, destination_scope};
use super::state::{CheckpointLedger, DeployState, DeploymentHome};
use crate::mechanism::record::sanitize;
use crate::mechanism::{DeployProvider, DeployTargetRequest, EffectClass};

/// Everything one target's transaction needs, and nothing it could use to
/// mint an identity of its own.
pub(crate) struct Transaction<'a> {
    /// The pinned state home.
    pub(crate) state: &'a DeployState,
    /// This deployment's own directory inside it.
    pub(crate) home: &'a DeploymentHome,
    /// The project/package identity the command layer resolved.
    pub(crate) identity: &'a DeployIdentity,
    /// The exact provider pin selection landed on.
    pub(crate) provider_pin: &'a str,
    /// The destination scope the selected provider's descriptor declared —
    /// carried so one receipt cannot disagree with the descriptor its plan
    /// was made under.
    pub(crate) scope: EffectClass,
    /// The run's injected RFC 3339 clock value.
    pub(crate) created_at: &'a str,
}

/// What one completed transaction produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppliedDeployment {
    pub(crate) generation: u32,
    pub(crate) reversible: bool,
    pub(crate) resources: Vec<OwnedResource>,
    pub(crate) prior_state_handle: Option<String>,
    /// What settling the previous run's journal did, if anything.
    pub(crate) settlement: Settlement,
}

/// What an unretired intent journal turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Settlement {
    /// There was no journal — the ordinary case.
    Nothing,
    /// A receipt plus its still-present matching intent: a benign crash
    /// after finalisation, so the intent retired and nothing else changed.
    BenignRetire,
    /// An interrupted deployment of THIS plan was rolled forward and
    /// finalised — §7.2's `recover` proper.
    RolledForward,
    /// An interrupted deployment of a DIFFERENT plan: the digest law held,
    /// the journal retired, and this run's own plan applied over it.
    StaleRetired,
}

impl Settlement {
    /// The word a narration prints.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Nothing => "none",
            Self::BenignRetire => "benign-intent-retired",
            Self::RolledForward => "recovered",
            Self::StaleRetired => "stale-intent-retired",
        }
    }
}

impl Transaction<'_> {
    /// Apply one target through the whole §7.2 sequence.
    ///
    /// The plan arrives already made and its destinations already locked:
    /// locking inside would make the plan an unlocked read of a
    /// destination another process may be mid-apply on.
    pub(crate) fn apply(
        &self,
        provider: &dyn DeployProvider,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<AppliedDeployment, DeployError> {
        let plan_hash = plan_hash(request, self.provider_pin, plan);
        // Step 0 — an unretired journal from an earlier run is settled
        // BEFORE this one writes anything durable.
        let settlement = match self.settle(provider, request, plan, &plan_hash)? {
            Settled::Completed(mut applied) => {
                applied.settlement = Settlement::RolledForward;
                return Ok(applied);
            }
            Settled::Continue(state) => state,
        };
        let prior = self.state.read_receipt(self.home)?;
        let generation = prior
            .as_ref()
            .map_or(0, |receipt| receipt.generation.saturating_add(1));
        // 1 — the durable intent, ATOMICALLY, before the first external
        // write. Every planned resource carries the digest the plan wants
        // and the digest the prior receipt recorded, because recovery
        // compares against exactly those two and nothing else.
        let intent = DeployIntent {
            plan_hash: plan_hash.clone(),
            resources: plan
                .resources
                .iter()
                .map(|planned| IntentResource {
                    resource: planned.resource.clone(),
                    desired_digest: planned.desired_digest.clone(),
                    prior_digest: prior_digest(prior.as_ref(), &planned.resource),
                })
                .collect(),
            schema: INTENT_EPOCH,
            started_at: self.timestamp(request)?,
            target: DeployTargetIdentity {
                generation,
                profile: request.profile.to_owned(),
                project: self.identity.project.clone(),
                target: request.target.id.clone(),
                package: self.identity.package.clone(),
            },
            prior_generation: prior.as_ref().map(|receipt| receipt.generation),
        };
        self.state.write_intent(self.home, &intent)?;
        // 2 — apply, checkpointing each completed operation. The
        // provider's own freshness fingerprint is taken first and rides
        // into the receipt's evidence: §3.2 lists `fingerprint` as one of
        // the six operations, and an operation nothing ever calls is a
        // declaration, not a protocol.
        let fingerprint = provider.fingerprint(request, plan)?;
        let mut ledger = CheckpointLedger::open(self.state, self.home, &plan_hash)?;
        let report = provider.apply(request, plan, &mut ledger)?;
        // 3 — INDEPENDENT verify, then the finalized receipt, then the
        // intent retires.
        let mut applied = self.finalize(
            provider,
            request,
            plan,
            generation,
            report.prior_state_handle,
            &format!(
                "{}; provider fingerprint {} over {}",
                report.evidence, fingerprint.digest, fingerprint.summary
            ),
            false,
        )?;
        applied.settlement = settlement;
        Ok(applied)
    }

    /// Settle an unretired intent journal, whatever it turns out to be.
    fn settle(
        &self,
        provider: &dyn DeployProvider,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        plan_hash: &str,
    ) -> Result<Settled, DeployError> {
        let Some(intent) = self.state.read_intent(self.home)? else {
            return Ok(Settled::Continue(Settlement::Nothing));
        };
        // The benign case first: a receipt whose generation is the one the
        // intent was opened for is a crash AFTER finalisation. Nothing is
        // interrupted; the journal simply outlived its purpose.
        if let Some(receipt) = self.state.read_receipt(self.home)?
            && receipt.generation == intent.target.generation
        {
            self.state.retire_intent(self.home)?;
            return Ok(Settled::Continue(Settlement::BenignRetire));
        }
        // The three-digest law runs against the INTERRUPTED journal's own
        // planned set — never against the new plan's, which would compare a
        // destination to digests nobody ever intended it to hold.
        let resources: Vec<String> = intent
            .resources
            .iter()
            .map(|planned| planned.resource.clone())
            .collect();
        let observed = provider.verify(request, &resources)?;
        let diverged = divergent(&intent, &observed);
        if !diverged.is_empty() {
            return Err(DeployError::RecoverDivergence {
                target: request.target.id.clone(),
                resources: diverged.join(", "),
            });
        }
        if intent.plan_hash != plan_hash {
            self.state.retire_intent(self.home)?;
            return Ok(Settled::Continue(Settlement::StaleRetired));
        }
        let mut ledger = CheckpointLedger::open(self.state, self.home, &intent.plan_hash)?;
        let report = provider.recover(request, plan, &observed, &mut ledger)?;
        // The recovered deployment finalises under the journal's OWN
        // generation: it is the completion of that deployment, not a new
        // one, and a receipt claiming a later generation would leave a gap
        // no journal explains.
        let applied = self.finalize(
            provider,
            request,
            plan,
            intent.target.generation,
            report.prior_state_handle,
            &report.evidence,
            true,
        )?;
        Ok(Settled::Completed(applied))
    }

    /// Verify independently, finalise the receipt, retire the intent.
    #[allow(clippy::too_many_arguments, reason = "one §7.2 step's named inputs")]
    fn finalize(
        &self,
        provider: &dyn DeployProvider,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        generation: u32,
        prior_state_handle: Option<String>,
        evidence: &str,
        recovered: bool,
    ) -> Result<AppliedDeployment, DeployError> {
        let resources: Vec<String> = plan
            .resources
            .iter()
            .map(|planned| planned.resource.clone())
            .collect();
        let observed = provider.verify(request, &resources)?;
        let mismatched = mismatched(plan, &observed);
        let owned: Vec<OwnedResource> = observed
            .iter()
            .filter_map(|resource| {
                resource.digest.as_ref().map(|digest| OwnedResource {
                    resource: resource.resource.clone(),
                    post_digest: digest.clone(),
                })
            })
            .collect();
        let status = if mismatched.is_empty() {
            ReceiptStatus::Verified
        } else {
            ReceiptStatus::Failed
        };
        let receipt = self.receipt(
            request,
            plan,
            generation,
            &owned,
            prior_state_handle.clone(),
            evidence,
            status,
            recovered,
        )?;
        // The receipt is written for BOTH verdicts. §7.2 puts finalisation
        // after verify so a failed verification cannot be reported as
        // success — but a deployment that applied and then failed to verify
        // really did touch the destination, and a state home that stayed
        // silent about it would leave that mutation with no owner.
        self.state.write_receipt(self.home, &receipt)?;
        self.state.retire_intent(self.home)?;
        if !mismatched.is_empty() {
            return Err(DeployError::VerifyMismatch {
                target: request.target.id.clone(),
                resources: mismatched.join(", "),
            });
        }
        Ok(AppliedDeployment {
            generation,
            reversible: plan.reversible,
            resources: owned,
            prior_state_handle,
            settlement: if recovered {
                Settlement::RolledForward
            } else {
                Settlement::Nothing
            },
        })
    }

    /// Reverse one applied deployment — the saga's rollback step and the
    /// body of `undeploy`.
    ///
    /// The drift law is the ENGINE's and is proven here, before the
    /// provider is asked to remove anything: a resource that is absent is
    /// already gone (benign), a resource at its recorded post-digest is
    /// this deployment's to remove, and anything else is §7.2's refusal.
    pub(crate) fn remove(
        &self,
        provider: &dyn DeployProvider,
        request: &DeployTargetRequest<'_>,
        receipt: &DeployReceipt,
        status: ReceiptStatus,
    ) -> Result<Vec<String>, DeployError> {
        let resources: Vec<String> = receipt
            .resources
            .iter()
            .map(|owned| owned.resource.clone())
            .collect();
        let observed = provider.verify(request, &resources)?;
        let drifted = drifted(receipt, &observed);
        if !drifted.is_empty() {
            return Err(DeployError::UndeployDrift {
                target: request.target.id.clone(),
                resources: drifted.join(", "),
            });
        }
        let present: Vec<String> = observed
            .iter()
            .filter(|resource| resource.digest.is_some())
            .map(|resource| resource.resource.clone())
            .collect();
        let report = provider.remove(request, &present, receipt.prior_state_handle.as_deref())?;
        let mut reversed = receipt.clone();
        reversed.status = status;
        reversed.finalized_at = Some(self.timestamp(request)?);
        // The receipt keeps existing and stops owning anything: §7.2 lets
        // `undeploy` remove only receipt-owned state, so a receipt that
        // still listed removed resources would authorise removing them
        // twice.
        reversed.resources = Vec::new();
        reversed.evidence = Some(sanitize(&format!(
            "{}; removed {} resource(s): {}",
            receipt.evidence.as_deref().unwrap_or("no prior evidence"),
            report.removed.len(),
            report.evidence,
        )));
        self.state.write_receipt(self.home, &reversed)?;
        Ok(report.removed)
    }

    /// Build one epoch-1 receipt.
    #[allow(clippy::too_many_arguments, reason = "§7.2's own record list")]
    fn receipt(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        generation: u32,
        owned: &[OwnedResource],
        prior_state_handle: Option<String>,
        evidence: &str,
        status: ReceiptStatus,
        recovered: bool,
    ) -> Result<DeployReceipt, DeployError> {
        let stamped = self.timestamp(request)?;
        // A target that deploys an artifact carries its digest; the zero
        // digest is the honest "no artifact took part" value for the
        // `remove` path, whose receipt is rewritten rather than minted.
        let artifact_digest = request
            .artifact
            .map_or_else(|| "0".repeat(64), |artifact| artifact.digest.clone());
        Ok(DeployReceipt {
            applied_at: stamped,
            artifact_digest,
            desired_config_digest: plan.config_digest.clone(),
            generation,
            identity: self.identity.clone(),
            profile: request.profile.to_owned(),
            provider: ProviderIdentity {
                key: self.provider_pin.to_owned(),
                version: None,
                content_hash: None,
            },
            resources: owned.to_vec(),
            reversible: plan.reversible,
            schema: RECEIPT_EPOCH,
            scope: destination_scope(self.scope),
            status,
            target: request.target.id.clone(),
            evidence: Some(sanitize(&format!(
                "{}{}; {}; {} owned resource(s)",
                if recovered { "recovered: " } else { "" },
                plan.summary,
                evidence,
                owned.len(),
            ))),
            finalized_at: Some(stamped),
            prior_state_handle,
        })
    }

    /// The injected clock, parsed once per record it stamps.
    fn timestamp(
        &self,
        request: &DeployTargetRequest<'_>,
    ) -> Result<Rfc3339Timestamp, DeployError> {
        self.created_at
            .parse::<Rfc3339Timestamp>()
            .map_err(|error| DeployError::Clock {
                target: request.target.id.clone(),
                value: self.created_at.to_owned(),
                reason: error.to_string(),
            })
    }
}

/// What settling an unretired journal decided.
enum Settled {
    /// The interrupted deployment WAS this plan, and it is now finished.
    Completed(AppliedDeployment),
    /// Nothing is outstanding; apply normally, carrying what happened.
    Continue(Settlement),
}

/// One plan's hash — the identity the intent journal and its checkpoint
/// ledger join on.
///
/// It folds the exact provider identity in for §4.1's reason ("Provider
/// changes invalidate the target even when its logical mechanism name did
/// not change"), and the resources in their planned order, so a plan that
/// touches the same set in a different order is a different plan.
pub(crate) fn plan_hash(
    request: &DeployTargetRequest<'_>,
    provider_pin: &str,
    plan: &DeployPlan,
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"deploy-plan/1\x00");
    hash.update(request.target.id.as_bytes());
    hash.update(b"\x00mechanism\x00");
    hash.update(request.target.mechanism.to_string().as_bytes());
    hash.update(b"\x00provider\x00");
    hash.update(provider_pin.as_bytes());
    hash.update(b"\x00config\x00");
    hash.update(plan.config_digest.as_bytes());
    if let Some(artifact) = request.artifact {
        hash.update(b"\x00artifact\x00");
        hash.update(artifact.id.as_bytes());
        hash.update(b"\x00");
        hash.update(artifact.digest.as_bytes());
    }
    for planned in &plan.resources {
        hash.update(b"\x00resource\x00");
        hash.update(planned.resource.as_bytes());
        hash.update(b"\x00");
        hash.update(planned.desired_digest.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

/// The prior receipt's digest for one resource, when it owned it.
fn prior_digest(prior: Option<&DeployReceipt>, resource: &str) -> Option<String> {
    prior?
        .resources
        .iter()
        .find(|owned| owned.resource == resource)
        .map(|owned| owned.post_digest.clone())
}

/// §7.2's third-digest test, as a set.
///
/// A resource is acceptable when what is observed is the desired digest
/// (already rolled forward) or the prior one (never touched). Absence
/// counts as the prior state only when there WAS no prior digest — a
/// resource the deployment was going to create. Everything else is the
/// third digest, and every one of them is named.
fn divergent(intent: &DeployIntent, observed: &[ObservedResource]) -> Vec<String> {
    let mut diverged = Vec::new();
    for planned in &intent.resources {
        let found = observed
            .iter()
            .find(|resource| resource.resource == planned.resource)
            .and_then(|resource| resource.digest.as_deref());
        let acceptable = match found {
            Some(digest) => {
                digest == planned.desired_digest || planned.prior_digest.as_deref() == Some(digest)
            }
            None => planned.prior_digest.is_none(),
        };
        if !acceptable {
            diverged.push(planned.resource.clone());
        }
    }
    diverged
}

/// Which planned resources independent verify did NOT find at the digest
/// the plan wanted.
fn mismatched(plan: &DeployPlan, observed: &[ObservedResource]) -> Vec<String> {
    let mut wrong = Vec::new();
    for planned in &plan.resources {
        let found = observed
            .iter()
            .find(|resource| resource.resource == planned.resource)
            .and_then(|resource| resource.digest.as_deref());
        if found != Some(planned.desired_digest.as_str()) {
            wrong.push(planned.resource.clone());
        }
    }
    wrong
}

/// Which receipt-owned resources changed after the deployment recorded
/// them. Absence is not drift: the resource is already gone.
fn drifted(receipt: &DeployReceipt, observed: &[ObservedResource]) -> Vec<String> {
    let mut changed = Vec::new();
    for owned in &receipt.resources {
        let found = observed
            .iter()
            .find(|resource| resource.resource == owned.resource)
            .and_then(|resource| resource.digest.as_deref());
        if let Some(digest) = found
            && digest != owned.post_digest
        {
            changed.push(owned.resource.clone());
        }
    }
    changed
}
