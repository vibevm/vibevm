//! What INDEPENDENT observation means — the three §7.2 comparisons, as pure
//! set functions.
//!
//! Its own cell because they are the one part of the transaction that is not
//! about ordering at all: given a record and what `verify` really saw, which
//! resources break the law. Each returns the EXACT resource identities rather
//! than a boolean, because every one of them ends in a refusal an operator
//! has to act on, and "something diverged" is not an instruction.
//!
//! The three are deliberately different laws over the same shape, and the
//! difference is worth reading in one place:
//!
//! - [`divergent`] is recovery's three-digest test — prior OR desired is
//!   acceptable, anything else is a concurrent mutation;
//! - [`mismatched`] is post-apply verification — only the desired digest is
//!   acceptable, because the plan said so;
//! - [`drifted`] is the inverse's refusal — only the recorded post-digest is
//!   acceptable, and ABSENCE is fine, because a resource already gone is a
//!   removal that already happened.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use vibe_wire::generated::deploy_intent::DeployIntent;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::protocol::{DeployPlan, ObservedResource};

/// The prior receipt's digest for one resource, when it owned it.
pub(super) fn prior_digest(prior: Option<&DeployReceipt>, resource: &str) -> Option<String> {
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
pub(super) fn divergent(intent: &DeployIntent, observed: &[ObservedResource]) -> Vec<String> {
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
pub(super) fn mismatched(plan: &DeployPlan, observed: &[ObservedResource]) -> Vec<String> {
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
pub(super) fn drifted(receipt: &DeployReceipt, observed: &[ObservedResource]) -> Vec<String> {
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
