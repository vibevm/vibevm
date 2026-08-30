//! §6.3.0.10's pre-apply epoch — every selected plan is made, and every
//! resource identity is judged, BEFORE the first destination is touched.
//!
//! > "Every selected plan is prepared before the first apply. The engine
//! > compares all owned and lock resources through the shared Unicode-9
//! > physical path identity. Duplicate owned identity always refuses.
//! > Duplicate physical lock identity refuses unless every participant
//! > explicitly uses reference ownership and owns a distinct logical member
//! > of that shared document/state. Thus a Codex/OpenCode combination
//! > cannot reach apply while competing for one skill or config member; the
//! > per-destination locks also serialize separate deployments of one shared
//! > document."
//!
//! Its own cell because it is its own responsibility and its own PROPERTY:
//! a refusal raised here is a refusal raised while every destination is
//! still byte-absent, and a path that is separate is a path a reader can
//! check. Nothing here opens a destination, writes a state file or takes a
//! lock — it resolves artifacts from the engine's own records, calls each
//! provider's `plan` verb, and compares strings.
//!
//! **Why the comparison is not `==`.** Two spellings that differ only by
//! case or by Unicode composition are ONE file on an APFS or NTFS volume.
//! `vibe_safefs::path_identity_key` is the audited answer this workspace
//! already uses for exactly that question (the package-skill receipt and
//! the artifact census read it too), so this cell reuses it rather than
//! carrying a second Unicode table. The exact declared spelling is what
//! travels into receipts and refusals; the key is a comparison value and
//! never becomes a path that is opened.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::collections::BTreeMap;

use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::artifact::resolve_artifact;
use super::error::DeployError;
use super::model::DeployExecution;
use super::protocol::{DeployPlan, ResolvedDeployArtifact};
use super::view::DeployStateView;
use super::{Selected, home_of};
use crate::mechanism::DeployTargetRequest;

/// One selected target, planned — the value apply reuses instead of
/// resolving and planning a second time.
///
/// §6.3.0.10 makes preplanning a transaction PREREQUISITE, not a preview:
/// "Reuse the resulting values during apply; do not resolve or plan a
/// second time." A second plan could differ from the one this cell judged,
/// and then the judgement would have been about a plan nobody applied.
pub(crate) struct Preplanned {
    /// The artifact this target reconciles, already proven from its record.
    pub(crate) artifact: ResolvedDeployArtifact,
    /// The plan its provider produced, already judged.
    pub(crate) plan: DeployPlan,
    /// The prior receipt this plan was made against — §6.3.1.1's injected
    /// ownership, OWNED here rather than borrowed because it has to outlive
    /// the read view that produced it and be compared, byte for byte,
    /// against whatever apply finds under the deployment-state lock.
    pub(crate) prior_receipt: Option<DeployReceipt>,
}

/// Prepare every selected target, then judge the whole resource set.
///
/// The order is the point: ALL plans are made first, so a refusal from the
/// last target leaves the first target's destination and state untouched.
pub(crate) fn preplan(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
) -> Result<Vec<Preplanned>, DeployError> {
    // §6.3.1.5: the prior receipts are read through the NO-CREATE view, so
    // a pre-apply epoch that refuses — or a `--plan` that never applies —
    // leaves an absent state home absent.
    let state = DeployStateView::open(execution.state_home)?;
    let mut prepared = Vec::with_capacity(resolved.len());
    for selected in resolved {
        let artifact = resolve_artifact(execution.project_root, selected.target)?;
        let prior_receipt = state.read_receipt(&home_of(execution, &selected.target.id))?;
        // The planning request carries no staging directory: staging is an
        // apply-time scratch, and a pre-apply epoch that offered one would
        // be handing a pure operation somewhere to write.
        let request = DeployTargetRequest {
            target: selected.target,
            profile: &execution.selection.profile,
            project_root: execution.project_root,
            settings_root: execution.settings_root,
            user_home: execution.user_home,
            clients: execution.clients,
            prior_receipt: prior_receipt.as_ref(),
            artifact: Some(&artifact),
            staging: None,
        };
        let plan = selected.provider.plan(&request)?;
        validate_lock_set(selected, &plan)?;
        prepared.push(Preplanned {
            artifact,
            plan,
            prior_receipt,
        });
    }
    let participants: Vec<Participant<'_, '_>> = resolved
        .iter()
        .zip(&prepared)
        .map(|(selected, planned)| Participant {
            selected,
            plan: &planned.plan,
        })
        .collect();
    judge_resources(&participants)?;
    Ok(prepared)
}

/// One target that produced a plan, as the judgement sees it.
///
/// A borrowed pair rather than an owned value because the two callers own
/// their plans differently — the pre-apply epoch keeps them for apply, the
/// read-only planner drops them after reporting — and the LAW is the same
/// either way. §6.3.0.10's rules are about the set, not about who stored
/// it.
pub(crate) struct Participant<'a, 'target> {
    pub(crate) selected: &'a Selected<'target>,
    pub(crate) plan: &'a DeployPlan,
}

/// One provider's own answer, judged against what it declared.
///
/// A provider that did not declare reference ownership may lock exactly
/// what it owns — the identities, in any order. Anything else is a defect
/// in that provider and refuses by name rather than being quietly widened
/// or narrowed by the engine.
pub(crate) fn validate_lock_set(
    selected: &Selected<'_>,
    plan: &DeployPlan,
) -> Result<(), DeployError> {
    if selected.provider.descriptor().reference_ownership {
        return Ok(());
    }
    let owned = identities(plan.resources.iter().map(|planned| &planned.resource));
    let locked = identities(plan.lock_resources.iter());
    if owned == locked {
        return Ok(());
    }
    Err(DeployError::LockSetNotDeclared {
        target: selected.target.id.clone(),
        pin: selected.pin.clone(),
        owned: list(plan.resources.iter().map(|planned| &planned.resource)),
        locked: list(plan.lock_resources.iter()),
    })
}

/// The two set laws, over a WHOLE participant set at once.
///
/// The ONE implementation of §6.3.0.10's rules. Both the pre-apply epoch
/// and the read-only planner call it, over the participants each of them
/// has: a second copy of the Unicode and reference-ownership rules is how
/// `--plan` would come to promise a deployment apply then refuses.
pub(crate) fn judge_resources(participants: &[Participant<'_, '_>]) -> Result<(), DeployError> {
    // 1 — owned identity is exclusive, unconditionally. This is the law a
    //     reference-owning provider does NOT get an exception from: sharing
    //     a document is admitted, sharing a member of it never is.
    let mut owners: BTreeMap<String, Claim> = BTreeMap::new();
    for participant in participants {
        for resource in &participant.plan.resources {
            let claim = Claim {
                target: participant.selected.target.id.clone(),
                spelling: resource.resource.clone(),
            };
            if let Some(first) = owners.insert(key(&resource.resource), claim.clone()) {
                return Err(DeployError::DuplicateOwnedResource {
                    first: first.target,
                    second: claim.target,
                    resource: first.spelling,
                    alias: claim.spelling,
                });
            }
        }
    }
    // 2 — physical lock identity may be shared, but only by a group that
    //     ALL declared reference ownership. Law 1 above already proved the
    //     owned members distinct, so the two conditions §6.3.0.10 names are
    //     both discharged by the time this returns.
    let mut lockers: BTreeMap<String, Claim> = BTreeMap::new();
    for participant in participants {
        let references = participant
            .selected
            .provider
            .descriptor()
            .reference_ownership;
        for resource in &participant.plan.lock_resources {
            let claim = Claim {
                target: participant.selected.target.id.clone(),
                spelling: resource.clone(),
            };
            let Some(first) = lockers.insert(key(resource), claim.clone()) else {
                continue;
            };
            let first_references = reference_owner(participants, &first.target);
            if references && first_references {
                // A shared document under two reference owners: admitted,
                // and re-inserted so a THIRD participant is judged against
                // the same identity rather than against nothing.
                continue;
            }
            let unreferenced = unreferenced(&first, first_references, &claim, references);
            return Err(DeployError::SharedLockNotReferenced {
                first: first.target,
                second: claim.target,
                resource: first.spelling,
                alias: claim.spelling,
                unreferenced,
            });
        }
    }
    Ok(())
}

/// Who claimed one identity, and in exactly which spelling.
#[derive(Clone)]
struct Claim {
    target: String,
    spelling: String,
}

/// Whether the named target's provider declared reference ownership.
fn reference_owner(participants: &[Participant<'_, '_>], target: &str) -> bool {
    participants
        .iter()
        .find(|participant| participant.selected.target.id == target)
        .is_some_and(|participant| {
            participant
                .selected
                .provider
                .descriptor()
                .reference_ownership
        })
}

/// Which participants of one refused share did not declare the capability.
fn unreferenced(first: &Claim, first_references: bool, second: &Claim, references: bool) -> String {
    let mut named = Vec::new();
    if !first_references {
        named.push(first.target.clone());
    }
    if !references {
        named.push(second.target.clone());
    }
    named.join(", ")
}

/// The shared physical identity of one resource spelling.
fn key(resource: &str) -> String {
    vibe_safefs::path_identity_key(resource)
}

/// The identity SET of a resource list, for the equal-sets comparison.
fn identities<'a>(
    resources: impl Iterator<Item = &'a String>,
) -> std::collections::BTreeSet<String> {
    resources.map(|resource| key(resource)).collect()
}

/// One resource list, in the exact spellings a refusal quotes.
fn list<'a>(resources: impl Iterator<Item = &'a String>) -> String {
    let joined: Vec<&str> = resources.map(String::as_str).collect();
    if joined.is_empty() {
        return "none".to_owned();
    }
    joined.join(", ")
}
