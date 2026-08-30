//! The two ownership refusals apply raises before it writes anything — the
//! ACROSS-RUNS half of §6.3.0.10's judgement, and §6.3.1.1's recheck.
//!
//! Its own cell because both answer one question the pre-apply epoch cannot:
//! *what does the state home say right now?* §6.3.0.10's judgement is about
//! one selection and is pure — it compares plans to each other. These two
//! read receipts, and they read them under the deployment-state lock,
//! because a fact about durable state is only true while it is held.
//!
//! They refuse for opposite reasons, and that is why both exist. One says
//! another DEPLOYMENT owns this resource; the other says this deployment's
//! own prior ownership changed since the plan in hand was made. A run that
//! checked only the first would apply a plan against a receipt that no
//! longer exists, and one that checked only the second would let two
//! deployments claim one file.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_wire::generated::deploy_receipt::{DeployReceipt, ReceiptStatus};

use super::Selected;
use super::error::DeployError;
use super::model::DeployStatus;
use super::preplan::Preplanned;
use super::sidecar::Ownership;
use super::state::{DeployState, DeploymentHome};

/// §6.3.1.1's recheck: "Apply rechecks the same receipt under the
/// deployment-state lock before writing."
///
/// The prior receipt is ENGINE evidence a provider plans against — "a
/// provider may update a present destination only when that receipt owns the
/// exact physical/logical resource and the observed digest still matches
/// it". Between the pre-apply epoch that read it and the moment this
/// deployment may write, another run of the same deployment can have
/// finished, been reversed, or first appeared. Any of those makes the plan
/// in hand a plan against ownership that no longer exists, so it refuses
/// rather than applying it — and it refuses HERE, under the lock, because
/// only under the lock is the answer still true a line later.
///
/// The whole receipt is compared, not its generation: a reversal keeps the
/// generation and empties the owned set, which is exactly the change a
/// provider must not plan through.
pub(crate) fn refuse_changed_ownership(
    state: &DeployState,
    home: &DeploymentHome,
    selected: &Selected<'_>,
    planned: &Preplanned,
) -> Result<(), DeployError> {
    let found = state.read_receipt(home)?;
    if found == planned.prior_receipt {
        return Ok(());
    }
    Err(DeployError::PriorReceiptChanged {
        target: selected.target.id.clone(),
        pin: selected.pin.clone(),
        planned: ownership_word(planned.prior_receipt.as_ref()),
        found: ownership_word(found.as_ref()),
    })
}

/// One prior-ownership value, in the words a refusal quotes.
fn ownership_word(receipt: Option<&DeployReceipt>) -> String {
    receipt.map_or_else(
        || "no prior deployment".to_owned(),
        |receipt| {
            format!(
                "generation {} ({}, {} owned resource(s))",
                receipt.generation,
                DeployStatus::of(&receipt.status).as_str(),
                receipt.resources.len(),
            )
        },
    )
}

/// The two facts every durable-lock law reads about one selected target.
pub(crate) fn ownership_of<'a>(selected: &'a Selected<'_>) -> Ownership<'a> {
    Ownership {
        target: &selected.target.id,
        pin: &selected.pin,
        reference: selected.provider.descriptor().reference_ownership,
    }
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
pub(crate) fn refuse_foreign_ownership(
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
