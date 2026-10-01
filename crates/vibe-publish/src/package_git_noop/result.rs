//! Historical no-effect result after two exact, source-owned local-bare reads.
//! No Mode A redemption, commit/tag staging, repository creation or Git push.

use super::*;
use crate::package_git_parent_source::{self, ParentSourceObservation, SourceLimits};
use crate::package_git_plan::RefSnapshot;

const RESULT: &str = "noeffect-result";
const RESULT_FORMAT: &str = "vibevm.package-git-noeffect-result/e1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NoEffectResultInspection {
    ReadyToObserve {
        run_id: String,
    },
    NoEffectCompleted {
        run_id: String,
        result_sha256: String,
    },
    Unavailable {
        run_id: Option<String>,
        reason: String,
    },
    RecoveryRequired {
        run_id: Option<String>,
        reason: String,
    },
}
fn result_recovery(run: Option<&str>, reason: impl Into<String>) -> NoEffectResultInspection {
    NoEffectResultInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn result_unavailable(run: Option<&str>, reason: impl Into<String>) -> NoEffectResultInspection {
    NoEffectResultInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoEffectResult {
    format: String,
    run_id: String,
    namespace_id: String,
    scope_context: String,
    scope_target: String,
    scope_principal: String,
    key_sha256: String,
    request_sha256: String,
    c1_plan_sha256: String,
    semantic_plan_sha256: String,
    lease_root_token: String,
    no_effect_seal_sha256: String,
    parent_landed_sha256: String,
    local_bare_root_token: String,
    local_bare_identity_sha256: String,
    observed_refs: RefSnapshot,
    observation_passes: u8,
    selected_main_tree_oid: String,
    selected_main_oid: String,
    selected_raw_tag_oid: String,
    selected_peeled_commit_oid: String,
}

#[allow(clippy::too_many_arguments)]
fn retained(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ReadyLimits,
) -> Result<(NoEffectSeal, PackageGitPlan, String, ParentReceipt), NoEffectResultInspection> {
    let (run, seal_sha256) = match inspect_ready_cold(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
    ) {
        NoEffectInspection::ReadyForObservation {
            run_id,
            seal_sha256,
        } => (run_id, seal_sha256),
        NoEffectInspection::Unavailable { run_id, reason } => {
            return Err(result_unavailable(run_id.as_deref(), reason));
        }
        other => {
            return Err(result_recovery(
                None,
                format!("no-effect Ready unavailable: {other:?}"),
            ));
        }
    };
    for forbidden in [
        "ready",
        "objects",
        "approval-link-pending",
        "approval-link",
        "repo-create-pending",
        "repo-create",
        "atomic-push-pending",
        "atomic-push",
        "result",
    ] {
        if !matches!(journal::read_bytes(lease, &run, forbidden), Ok(None)) {
            return Err(result_recovery(
                Some(&run),
                format!("effect record `{forbidden}` occupies no-effect run"),
            ));
        }
    }
    let bytes = exact_bytes(lease, &run, SEAL)
        .map_err(|s| result_recovery(Some(&run), format!("seal: {s:?}")))?;
    let seal: NoEffectSeal =
        serde_json::from_slice(&bytes).map_err(|e| result_recovery(Some(&run), e.to_string()))?;
    if sha(&bytes) != seal_sha256
        || serde_json::to_vec(&seal).ok().as_deref() != Some(bytes.as_slice())
    {
        return Err(result_recovery(Some(&run), "no-effect seal changed"));
    }
    let (_, proof) =
        package_git_operation::ready_proof_cold(lease, namespace_token, scope, key, package_source)
            .map_err(|e| result_recovery(Some(&run), e.to_string()))?;
    let proof = proof.ok_or_else(|| result_recovery(Some(&run), "Ready plan proof absent"))?;
    if proof.run_id != run
        || proof.plan.digest().ok().as_deref() != Some(seal.semantic_plan_sha256.as_str())
    {
        return Err(result_recovery(
            Some(&run),
            "Ready plan differs from no-effect seal",
        ));
    }
    let parent_bytes = exact_bytes(lease, &run, "parent")
        .map_err(|s| result_recovery(Some(&run), format!("parent: {s:?}")))?;
    let parent: ParentReceipt = serde_json::from_slice(&parent_bytes)
        .map_err(|e| result_recovery(Some(&run), e.to_string()))?;
    if sha(&parent_bytes) != seal.parent_landed_sha256
        || parent.local_bare_identity_sha256.as_str()
            != match &proof.plan.target {
                RepositoryTarget::DirectGitExisting {
                    remote_identity_sha256,
                } => remote_identity_sha256,
                _ => {
                    return Err(result_unavailable(
                        Some(&run),
                        "hosted no-effect profile unsupported",
                    ));
                }
            }
        || parent.observed_refs != proof.plan.before_refs
        || serde_json::to_vec(&parent).ok().as_deref() != Some(parent_bytes.as_slice())
    {
        return Err(result_recovery(Some(&run), "parent source witness differs"));
    }
    Ok((seal, proof.plan, seal_sha256, parent))
}

fn expected_result(
    seal: &NoEffectSeal,
    plan: &PackageGitPlan,
    seal_sha256: &str,
    parent: &ParentReceipt,
) -> NoEffectResult {
    NoEffectResult {
        format: RESULT_FORMAT.into(),
        run_id: seal.run_id.clone(),
        namespace_id: seal.namespace_id.clone(),
        scope_context: seal.scope_context.clone(),
        scope_target: seal.scope_target.clone(),
        scope_principal: seal.scope_principal.clone(),
        key_sha256: seal.key_sha256.clone(),
        request_sha256: seal.request_sha256.clone(),
        c1_plan_sha256: seal.c1_plan_sha256.clone(),
        semantic_plan_sha256: seal.semantic_plan_sha256.clone(),
        lease_root_token: seal.lease_root_token.clone(),
        no_effect_seal_sha256: seal_sha256.into(),
        parent_landed_sha256: seal.parent_landed_sha256.clone(),
        local_bare_root_token: parent.local_bare_root_token.clone(),
        local_bare_identity_sha256: parent.local_bare_identity_sha256.clone(),
        observed_refs: plan.before_refs.clone(),
        observation_passes: 2,
        selected_main_tree_oid: seal.selected_main_tree_oid.clone(),
        selected_main_oid: seal.selected_main_oid.clone(),
        selected_raw_tag_oid: seal.selected_raw_tag_oid.clone(),
        selected_peeled_commit_oid: seal.selected_peeled_commit_oid.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inspect_result_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    binary: &Path,
    limits: ReadyLimits,
) -> NoEffectResultInspection {
    let (seal, plan, digest, parent) = match retained(
        lease,
        namespace_token,
        scope,
        key,
        None,
        None,
        binary,
        limits,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let expected = expected_result(&seal, &plan, &digest, &parent);
    let run = seal.run_id.as_str();
    let bytes = match journal::read_bytes(lease, run, RESULT) {
        Ok(Some(value)) => value,
        Ok(None) => return NoEffectResultInspection::ReadyToObserve { run_id: run.into() },
        Err(e) => return result_recovery(Some(run), e.to_string()),
    };
    let row: NoEffectResult = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(e) => return result_recovery(Some(run), e.to_string()),
    };
    if row != expected || serde_json::to_vec(&row).ok().as_deref() != Some(bytes.as_slice()) {
        return result_recovery(
            Some(run),
            "no-effect result differs from retained two-epoch identity",
        );
    }
    if !matches!(journal::read_bytes(lease, run, RESULT), Ok(Some(ref now)) if now == &bytes) {
        return result_recovery(Some(run), "no-effect result changed during cold inspection");
    }
    NoEffectResultInspection::NoEffectCompleted {
        run_id: run.into(),
        result_sha256: sha(&bytes),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn observe_and_seal_result(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: &Project,
    binary: &Path,
    limits: ReadyLimits,
) -> NoEffectResultInspection {
    observe_inner(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
        || {},
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn observe_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: &Project,
    binary: &Path,
    limits: ReadyLimits,
    between_epochs: impl FnOnce(),
    stop_after_publish: bool,
) -> NoEffectResultInspection {
    let status = inspect_result_cold(lease, namespace_token, scope, key, binary, limits);
    let NoEffectResultInspection::ReadyToObserve { run_id } = status else {
        return status;
    };
    let Some(source_limits): Option<SourceLimits> = limits.parent.map(|parent| parent.source)
    else {
        return result_unavailable(Some(&run_id), "explicit direct source limits absent");
    };
    let (seal, plan, digest, parent) = match retained(
        lease,
        namespace_token,
        scope,
        key,
        Some(package_source),
        Some(local_bare_source),
        binary,
        limits,
    ) {
        Ok(value) if value.0.run_id == run_id => value,
        Ok(_) => return result_recovery(Some(&run_id), "run changed before no-effect observation"),
        Err(status) => return status,
    };
    let first = package_git_parent_source::observe_local_bare(
        local_bare_source,
        binary,
        &plan,
        source_limits,
    );
    between_epochs();
    let second = package_git_parent_source::observe_local_bare(
        local_bare_source,
        binary,
        &plan,
        source_limits,
    );
    let exact = ParentSourceObservation::Exact {
        refs: plan.before_refs.clone(),
        repository_root_token: parent.local_bare_root_token.clone(),
        local_bare_identity_sha256: parent.local_bare_identity_sha256.clone(),
    };
    if first != exact || second != exact {
        return result_recovery(
            Some(&run_id),
            format!("source-owned no-effect observations changed: {first:?} / {second:?}"),
        );
    }
    if !matches!(inspect_ready_cold(lease, namespace_token, scope, key,
        Some(package_source), Some(local_bare_source), binary, limits),
        NoEffectInspection::ReadyForObservation { run_id: ref same, seal_sha256: ref hash }
            if same == &run_id && hash == &digest)
    {
        return result_recovery(
            Some(&run_id),
            "no-effect Ready changed after remote observations",
        );
    }
    if !matches!(journal::read_bytes(lease, &run_id, RESULT), Ok(None)) {
        return result_recovery(Some(&run_id), "no-effect result appeared before create-new");
    }
    let result = expected_result(&seal, &plan, &digest, &parent);
    if let Err(e) = journal::publish(lease, &run_id, RESULT, &result) {
        return result_recovery(
            Some(&run_id),
            format!("no-effect result publication uncertain: {e:#}"),
        );
    }
    if stop_after_publish {
        return result_recovery(Some(&run_id), "injected lost no-effect result reply");
    }
    inspect_result_cold(lease, namespace_token, scope, key, binary, limits)
}

#[cfg(test)]
#[path = "result_tests.rs"]
mod tests;
