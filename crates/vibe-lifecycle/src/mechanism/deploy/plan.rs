//! §7.0.6's read-only planner.
//!
//! > "`--plan` is a read-only planner, not a chain run: resolve the
//! > profile, read records and receipts, compute staleness, call provider
//! > `plan` verbs only, report — no token read, no network, no build, no
//! > destination mutation."
//!
//! Its own cell because "read-only" is a property of a whole code path,
//! and a path that is separate is a path a reader can check. Nothing here
//! calls `apply`, `verify`, `remove` or `recover`; nothing here opens the
//! destination at all. Staleness is computed from the ENGINE's own
//! receipt and the provider's plan — never by observing the destination —
//! so a plan cannot even accidentally become a probe.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use specmark::spec;

use super::artifact::resolve_artifact;
use super::error::DeployError;
use super::model::{DeployExecution, DeployPlanReport, DeployResourcePlan};
use super::protocol::{DeployPlan, ResolvedDeployArtifact};
use super::state::DeployState;
use super::{Selected, home_of, resolve_selection};
use crate::mechanism::DeployTargetRequest;
use vibe_wire::generated::deploy_receipt::{DeployReceipt, ReceiptStatus};

/// Report what a deploy would do, without touching a destination.
///
/// §7.0.6's read-only planner: it resolves providers, reads the engine's
/// own records and receipts, calls each provider's `plan` verb — and NO
/// other verb — and reports. It reads no token, opens no socket, runs no
/// build and mutates nothing, including its own state home.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn plan_deploy_targets(
    execution: &DeployExecution<'_>,
) -> Result<Vec<DeployPlanReport>, DeployError> {
    let resolved = resolve_selection(execution)?;
    plan_resolved(execution, &resolved)
}

/// Report what one ALREADY-resolved selection would do.
///
/// The half below selection, separated for the reason [`apply_selection`]
/// is: the planner's own laws are provable with hermetic providers, while
/// selection still happens in exactly one place.
pub(crate) fn plan_resolved(
    execution: &DeployExecution<'_>,
    resolved: &[Selected<'_>],
) -> Result<Vec<DeployPlanReport>, DeployError> {
    let state = DeployState::open(execution.state_home)?;
    let mut reports: Vec<DeployPlanReport> = Vec::with_capacity(resolved.len());
    // Which target ids are already known to be planned work, so a target
    // that depends on one is reported as planned too.
    let mut planned_ids: Vec<&str> = Vec::new();
    for selected in resolved {
        let home = home_of(execution, &selected.target.id);
        let receipt = state.read_receipt(&home)?;
        let artifact = match resolve_artifact(execution.project_root, selected.target) {
            Ok(artifact) => Some(artifact),
            // A read-only planner never builds, so an artifact that has
            // not been produced yet is not a refusal: it is the honest
            // report that the producing work is planned too.
            Err(DeployError::ArtifactNotRecorded { .. } | DeployError::ArtifactMissing { .. }) => {
                None
            }
            Err(error) => return Err(error),
        };
        let upstream_planned = selected
            .target
            .depends_on
            .iter()
            .flatten()
            .any(|dependency| planned_ids.contains(&dependency.as_str()));
        let Some(artifact) = artifact else {
            planned_ids.push(&selected.target.id);
            reports.push(DeployPlanReport {
                target: selected.target.id.clone(),
                mechanism: selected.target.mechanism.to_string(),
                provider: selected.pin.clone(),
                via: selected.via.to_string(),
                displaced_default: selected.displaced.clone(),
                planned: true,
                reason: format!(
                    "artifact `{}` has no record yet, so producing it is planned work",
                    selected.target.artifact
                ),
                resources: Vec::new(),
                summary: "not planned in detail: the artifact is not produced yet".to_owned(),
            });
            continue;
        };
        let request = DeployTargetRequest {
            target: selected.target,
            profile: &execution.selection.profile,
            project_root: execution.project_root,
            settings_root: execution.settings_root,
            artifact: Some(&artifact),
            // A plan never stages: staging is an apply-time scratch, and
            // offering one here would be a directory a pure operation
            // could write into.
            staging: None,
        };
        let plan = selected.provider.plan(&request)?;
        let resources = planned_resources(&plan, receipt.as_ref());
        let stale = is_stale(&plan, &artifact, receipt.as_ref());
        let planned = stale || upstream_planned;
        if planned {
            planned_ids.push(&selected.target.id);
        }
        reports.push(DeployPlanReport {
            target: selected.target.id.clone(),
            mechanism: selected.target.mechanism.to_string(),
            provider: selected.pin.clone(),
            via: selected.via.to_string(),
            displaced_default: selected.displaced.clone(),
            planned,
            reason: reason(stale, upstream_planned),
            resources,
            summary: plan.summary,
        });
    }
    Ok(reports)
}

/// Whether one target's plan differs from what its receipt records.
fn is_stale(
    plan: &DeployPlan,
    artifact: &ResolvedDeployArtifact,
    receipt: Option<&DeployReceipt>,
) -> bool {
    let Some(receipt) = receipt else {
        return true;
    };
    if receipt.status != ReceiptStatus::Verified {
        return true;
    }
    if receipt.desired_config_digest != plan.config_digest
        || receipt.artifact_digest != artifact.digest
    {
        return true;
    }
    plan.resources.iter().any(|planned| {
        receipt
            .resources
            .iter()
            .find(|owned| owned.resource == planned.resource)
            .is_none_or(|owned| owned.post_digest != planned.desired_digest)
    })
}

/// The per-resource plan rows, joined against the last receipt.
fn planned_resources(
    plan: &DeployPlan,
    receipt: Option<&DeployReceipt>,
) -> Vec<DeployResourcePlan> {
    plan.resources
        .iter()
        .map(|planned| {
            let recorded = receipt.and_then(|receipt| {
                receipt
                    .resources
                    .iter()
                    .find(|owned| owned.resource == planned.resource)
                    .map(|owned| owned.post_digest.clone())
            });
            let change = match recorded.as_deref() {
                None => "create",
                Some(digest) if digest == planned.desired_digest => "unchanged",
                Some(_) => "update",
            };
            DeployResourcePlan {
                resource: planned.resource.clone(),
                desired_digest: planned.desired_digest.clone(),
                recorded_digest: recorded,
                change: change.to_owned(),
            }
        })
        .collect()
}

/// Why a target is (or is not) planned work.
fn reason(stale: bool, upstream: bool) -> String {
    match (stale, upstream) {
        (true, _) => "the recorded deployment does not match this plan".to_owned(),
        (false, true) => {
            "a preceding target in this profile is stale, so this one is planned work too"
                .to_owned()
        }
        (false, false) => "the recorded deployment already matches this plan".to_owned(),
    }
}
