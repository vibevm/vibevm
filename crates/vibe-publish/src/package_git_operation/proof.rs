//! Two-pass held-vault reconstruction under exact C.1 Pending or Ready.
use super::journal::{self, Intent, sha};
use super::{OperationError, PendingInspection, PendingProof, inspect_local_cold};
use crate::package_git_payload::{LocalPayloadInspection, inspect_local_payload};
use crate::package_git_plan::PackageGitPlan;
use vibe_lifecycle::{JournalAdmission, LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

pub(crate) fn pending_proof_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
) -> Result<(PendingInspection, Option<PendingProof>), OperationError> {
    proof_cold_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        JournalAdmission::Pending,
        |_| {},
    )
}

pub(crate) fn ready_proof_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
) -> Result<(PendingInspection, Option<PendingProof>), OperationError> {
    proof_cold_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        JournalAdmission::Ready,
        |_| {},
    )
}

#[cfg(test)]
pub(super) fn pending_proof_cold_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    after_first_inspection: impl FnOnce(&str),
) -> Result<(PendingInspection, Option<PendingProof>), OperationError> {
    proof_cold_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        JournalAdmission::Pending,
        after_first_inspection,
    )
}

fn proof_cold_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    required: JournalAdmission,
    after_first_inspection: impl FnOnce(&str),
) -> Result<(PendingInspection, Option<PendingProof>), OperationError> {
    let status = inspect_local_cold(lease, namespace_token, scope, key, source, required)?;
    let run_id = match &status {
        PendingInspection::VaultCompleteButNotReady { run_id, .. }
            if required == JournalAdmission::Pending =>
        {
            run_id
        }
        PendingInspection::VaultCompleteReady { run_id, .. }
            if required == JournalAdmission::Ready =>
        {
            run_id
        }
        _ => return Ok((status, None)),
    };
    let uncertain = |reason: String| PendingInspection::Uncertain {
        run_id: run_id.clone(),
        reason,
    };
    after_first_inspection(run_id);
    // From this point onward the run has already been observed as complete.
    // Any read/decode failure is a changed or unreadable retained witness, not
    // an ordinary preflight refusal that a caller may retry by staging again.
    let rebuilt = (|| -> anyhow::Result<PendingProof> {
        let intent_bytes = journal::read_bytes(lease, run_id, "intent")?
            .ok_or_else(|| anyhow::anyhow!("owner intent vanished"))?;
        let intent: Intent = serde_json::from_slice(&intent_bytes)?;
        let plan: PackageGitPlan = serde_json::from_str(&intent.plan_json)?;
        let vault = journal::vault(lease, run_id, false)?
            .ok_or_else(|| anyhow::anyhow!("payload vault vanished"))?;
        let manifest = match inspect_local_payload(
            &vault,
            intent.limits,
            &intent.source_root_token,
            &intent.source_tree_sha256,
        ) {
            LocalPayloadInspection::Complete(value) => value,
            other => anyhow::bail!("payload changed during proof: {other:?}"),
        };
        let root = vault.root_dir()?;
        let manifest_bytes = vault
            .read_file_bounded_in(&root, "complete.json", intent.limits.max_manifest_bytes)?
            .ok_or_else(|| anyhow::anyhow!("payload completion vanished"))?;
        let again = inspect_local_cold(lease, namespace_token, scope, key, source, required)?;
        if again != status {
            anyhow::bail!("C.1 or payload changed while rebuilding proof");
        }
        Ok(PendingProof {
            run_id: run_id.clone(),
            plan,
            manifest,
            vault,
            intent_sha256: sha(&intent_bytes),
            payload_manifest_sha256: sha(&manifest_bytes),
        })
    })();
    match rebuilt {
        Ok(proof) => Ok((status, Some(proof))),
        Err(error) => Ok((
            uncertain(format!("retained Pending proof changed: {error:#}")),
            None,
        )),
    }
}
