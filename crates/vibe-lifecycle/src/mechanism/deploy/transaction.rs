//! The §7.2 transaction — the cell that owns the durable event order.
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
//! Providers see destinations only after durable intent; receipts follow
//! independent observation, never an apply claim.
//!
//! A stale unretired plan still runs the three-digest law, then retires
//! without roll-forward; the new plan applies only over a proven old state.
//!
//! §6.3.1.2 adds ONE record to that order and does not otherwise disturb it:
//! the durable lock sidecar's PENDING binding, published and read back
//! before the intent — and therefore before any external write could have
//! begun — and promoted to COMMITTED only after the receipt is durable. Each
//! of the four settlements below moves exactly one slot of it, and every
//! transition runs under the deployment-state lock this cell's caller holds.

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
use super::observation::{divergent, drifted, mismatched, prior_digest};
use super::protocol::{DeployPlan, destination_scope};
use super::sidecar;
use super::state::{CheckpointLedger, DeployState, DeploymentHome, InverseRecord};
use crate::mechanism::record::sanitize;
use crate::mechanism::{DeployProvider, DeployTargetRequest, EffectClass};

#[path = "transaction/inverse.rs"]
mod inverse;

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
        // 1 — the PENDING lock binding, published and read back BEFORE the
        // intent. §6.3.1.2's ordering is what makes a crash between the two
        // safe in one direction only: a pending binding with no intent
        // proves no external write can have begun, so the next run may
        // replace it — while an intent with no binding would leave a
        // reference owner's physical destinations unrecorded. The COMMITTED
        // binding is retained throughout, so the inverse lock of the
        // generation still deployed survives the whole update.
        sidecar::stage_pending(
            self.state,
            self.home,
            generation,
            &plan_hash,
            &plan.lock_resources,
        )?;
        // 2 — the durable intent, ATOMICALLY, before the first external
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
        // 3 — apply, checkpointing each completed operation. The
        // provider's own freshness fingerprint is taken first and rides
        // into the receipt's evidence: §3.2 lists `fingerprint` as one of
        // the six operations, and an operation nothing ever calls is a
        // declaration, not a protocol.
        let fingerprint = provider.fingerprint(request, plan)?;
        let mut ledger = CheckpointLedger::open(self.state, self.home, &plan_hash)?;
        let report = provider.apply(request, plan, &mut ledger)?;
        // 4 — INDEPENDENT verify, the finalized receipt, the promotion of
        // this generation's pending binding, then the intent retires.
        let mut applied = self.finalize(
            provider,
            request,
            plan,
            Generation {
                number: generation,
                plan_hash: &plan_hash,
            },
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
    ///
    /// Each of the three outcomes moves exactly one sidecar slot, and which
    /// one it moves is §6.3.1.3's own sentence: a benign window PROMOTES the
    /// matching pending binding (the receipt is already durable, so the
    /// crash was after finalisation), a stale journal CLEARS only its own
    /// pending generation, and a real recovery finalises through
    /// [`Self::finalize`], which promotes.
    ///
    /// The matching pending binding a recovery runs under is guaranteed by
    /// the caller: [`settle_bindings`](super::sidecar::settle_bindings)
    /// materialises the ordinary provider's typed fallback and refuses a
    /// reference owner outright, BEFORE the physical locks this settlement
    /// runs inside were taken.
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
            sidecar::promote(
                self.state,
                self.home,
                intent.target.generation,
                &intent.plan_hash,
            )?;
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
            // Only THIS journal's pending generation goes; the committed
            // binding describes a deployment that is still deployed.
            sidecar::clear_pending(
                self.state,
                self.home,
                intent.target.generation,
                &intent.plan_hash,
            )?;
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
            Generation {
                number: intent.target.generation,
                plan_hash: &intent.plan_hash,
            },
            report.prior_state_handle,
            &report.evidence,
            true,
        )?;
        Ok(Settled::Completed(applied))
    }

    /// Verify independently, finalise the receipt, promote this
    /// generation's pending lock binding, retire the intent.
    #[allow(clippy::too_many_arguments, reason = "one §7.2 step's named inputs")]
    fn finalize(
        &self,
        provider: &dyn DeployProvider,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        generation: Generation<'_>,
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
            generation.number,
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
        // And for the same reason the promotion is unconditional: a failed
        // receipt owns whatever independent verify observed, so the inverse
        // that removes it must lock the physical destinations this
        // generation held. Promotion follows the receipt and never precedes
        // it — a committed binding without a receipt would claim a
        // deployment nobody recorded.
        sidecar::promote(
            self.state,
            self.home,
            generation.number,
            generation.plan_hash,
        )?;
        self.state.retire_intent(self.home)?;
        if !mismatched.is_empty() {
            return Err(DeployError::VerifyMismatch {
                target: request.target.id.clone(),
                resources: mismatched.join(", "),
            });
        }
        Ok(AppliedDeployment {
            generation: generation.number,
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

    pub(crate) fn remove(
        &self,
        provider: &dyn DeployProvider,
        request: &DeployTargetRequest<'_>,
        receipt: &DeployReceipt,
        prior_state_handle: Option<&str>,
        status: ReceiptStatus,
    ) -> Result<Vec<String>, DeployError> {
        let inverse = match (prior_state_handle, receipt.resources.as_slice()) {
            (Some(handle), [owned])
                if self.provider_pin == crate::mechanism::BUILTIN_VIBE_OPT_LAUNCHER_PIN =>
            {
                Some(InverseRecord::new(
                    receipt.generation,
                    self.provider_pin,
                    &owned.resource,
                    &owned.post_digest,
                    handle,
                ))
            }
            _ => None,
        };
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
        if let Some(marker) = inverse.as_ref() {
            self.state.write_inverse(self.home, marker)?;
        }
        let present: Vec<String> = observed
            .iter()
            .filter(|resource| resource.digest.is_some())
            .map(|resource| resource.resource.clone())
            .collect();
        let report = provider.remove(request, &present, prior_state_handle)?;
        let restored = if inverse.is_some() {
            let independently_observed = provider.verify(request, &resources)?;
            if independently_observed != report.expected_remaining {
                return Err(DeployError::VerifyMismatch {
                    target: request.target.id.clone(),
                    resources: resources.join(", "),
                });
            }
            independently_observed
                .into_iter()
                .filter_map(|resource| {
                    resource.digest.map(|digest| OwnedResource {
                        resource: resource.resource,
                        post_digest: digest,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut reversed = receipt.clone();
        reversed.status = status;
        reversed.finalized_at = Some(self.timestamp(request)?);
        reversed.resources = restored;
        if inverse.is_some() {
            reversed.prior_state_handle = None;
        }
        reversed.evidence = Some(sanitize(&format!(
            "{}; removed {} resource(s): {}",
            receipt.evidence.as_deref().unwrap_or("no prior evidence"),
            report.removed.len(),
            report.evidence,
        )));
        self.state.write_receipt(self.home, &reversed)?;
        if inverse.is_none() {
            sidecar::clear_committed(self.state, self.home, receipt.generation)?;
        }
        if inverse.is_some() {
            self.state.retire_inverse(self.home)?;
        }
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

/// The generation one finalisation belongs to, and the plan that opened it.
///
/// The two travel together because every finalisation does two things with
/// them at once — stamps a receipt and promotes a lock binding — and a
/// promotion keyed on a generation without its plan hash would adopt a
/// pending binding some other plan left behind.
#[derive(Debug, Clone, Copy)]
struct Generation<'a> {
    number: u32,
    plan_hash: &'a str,
}

/// One plan's hash — the identity the intent journal and its checkpoint
/// ledger join on.
///
/// It folds the exact provider identity in for §4.1's reason ("Provider
/// changes invalidate the target even when its logical mechanism name did
/// not change"), and the resources in their planned order, so a plan that
/// touches the same set in a different order is a different plan.
///
/// §6.3.1.3: "The deploy plan hash binds `lock_resources` as well as owned
/// resources." A LOCK-ONLY change is a different plan, and it has to be: the
/// pending binding and the checkpoint ledger both join on this hash, so two
/// plans that own the same resources while locking different physical
/// destinations would otherwise share one binding — and a recovery would run
/// under the wrong document's lock. The two lists get DISTINCT frames, so a
/// resource moving between them changes the hash rather than cancelling out.
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
    for locked in &plan.lock_resources {
        hash.update(b"\x00lock\x00");
        hash.update(locked.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
