//! Identity-only duplicate check; full Git proof stays in the cold inspector.
use super::*;

/// Identity-only duplicate check: `prepare_pending` cannot read Git objects.
/// The caller must still use `inspect_ready_cold` before any no-effect claim.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ready_duplicate_matches(
    lease: &LifecycleLease,
    run: &str,
    namespace_id: &str,
    scope: &SubmissionScope,
    key: &str,
    request_bytes: &[u8],
    plan: &PackageGitPlan,
    plan_bytes: &[u8],
    lease_root_token: &str,
    source_root_token: &str,
) -> bool {
    if !matches!(plan.staged.commit, StagedCommit::ReusedMain { .. }) {
        return false;
    }
    let Ok(Some(bytes)) = journal::read_bytes(lease, run, SEAL) else {
        return false;
    };
    let Ok(seal) = serde_json::from_slice::<NoEffectSeal>(&bytes) else {
        return false;
    };
    seal.format == FORMAT
        && seal.run_id == run
        && seal.namespace_id == namespace_id
        && seal.scope_context == scope.context
        && seal.scope_target == scope.target
        && seal.scope_principal == scope.principal
        && seal.key_sha256 == sha(key.as_bytes())
        && seal.request_sha256 == sha(request_bytes)
        && seal.c1_plan_sha256 == sha(plan_bytes)
        && plan.digest().ok().as_deref() == Some(seal.semantic_plan_sha256.as_str())
        && seal.lease_root_token == lease_root_token
        && seal.source_root_token == source_root_token
        && matches!(&plan.before_refs.main, RawRef::Present { oid } if oid == &seal.selected_main_oid)
        && matches!(&plan.before_refs.version_tag, TagRef::Present { raw_oid, peeled_commit_oid }
            if raw_oid == &seal.selected_raw_tag_oid && peeled_commit_oid == &seal.selected_peeled_commit_oid)
        && seal.selected_main_tree_oid == plan.staged.git_tree_oid
        && serde_json::to_vec(&seal).ok().as_deref() == Some(bytes.as_slice())
}
