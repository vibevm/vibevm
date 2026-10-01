//! Distinct local C.1 Ready seal for an exact ReusedMain no-effect candidate.
//! This is retained byte/parent proof, not a remote observation or result.

use std::path::Path;

use serde::{Deserialize, Serialize};
use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;

use crate::package_git_operation::journal::{self, ParentReceipt, sha};
use crate::package_git_operation::{self, PendingInspection, PendingProof};
use crate::package_git_parent::{self, ParentInspection};
use crate::package_git_plan::{
    PackageGitPlan, RawRef, RepositoryIdentity, RepositoryObservation, RepositoryTarget,
    StagedCommit, TagRef,
};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_ready::ReadyLimits;
use crate::package_git_tree::{self, TreeInspection};

const SEAL: &str = "noeffect-ready";
const FORMAT: &str = "vibevm.package-git-noeffect-ready/e1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NoEffectInspection {
    NotReady {
        run_id: Option<String>,
    },
    SealRetainedC1Pending {
        run_id: String,
        seal_sha256: String,
    },
    ReadyForObservation {
        run_id: String,
        seal_sha256: String,
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
fn recovery(run: Option<&str>, reason: impl Into<String>) -> NoEffectInspection {
    NoEffectInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn unavailable(run: Option<&str>, reason: impl Into<String>) -> NoEffectInspection {
    NoEffectInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoEffectSeal {
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
    source_root_token: String,
    vault_root_token: String,
    object_root_token: String,
    intent_sha256: String,
    payload_manifest_sha256: String,
    stage_receipt_sha256: String,
    tree_receipt_sha256: String,
    provenance_pending_sha256: String,
    provenance_landed_sha256: String,
    parent_pending_sha256: String,
    parent_landed_sha256: String,
    selected_main_oid: String,
    selected_main_tree_oid: String,
    selected_raw_tag_oid: String,
    selected_peeled_commit_oid: String,
}

fn exact_bytes(
    lease: &LifecycleLease,
    run: &str,
    suffix: &str,
) -> Result<Vec<u8>, NoEffectInspection> {
    journal::read_bytes(lease, run, suffix)
        .map_err(|e| recovery(Some(run), e.to_string()))?
        .ok_or_else(|| recovery(Some(run), format!("{suffix} receipt absent")))
}

fn no_effect_shape<'a>(
    plan: &'a PackageGitPlan,
    run: &str,
) -> Result<(&'a str, &'a str, &'a str), NoEffectInspection> {
    let (
        RepositoryTarget::DirectGitExisting {
            remote_identity_sha256: selected,
        },
        RepositoryObservation::Present(RepositoryIdentity::Direct {
            remote_identity_sha256: observed,
        }),
        RawRef::Present { oid: main },
        TagRef::Present {
            raw_oid,
            peeled_commit_oid,
        },
        StagedCommit::ReusedMain { oid: reused },
    ) = (
        &plan.target,
        &plan.before_repo,
        &plan.before_refs.main,
        &plan.before_refs.version_tag,
        &plan.staged.commit,
    )
    else {
        return Err(unavailable(
            Some(run),
            "ReusedMain no-effect supports selected DirectGitExisting only",
        ));
    };
    if selected != observed
        || main != reused
        || peeled_commit_oid != main
        || plan.staged.tag_target_commit_oid != *main
        || plan.staged.annotated_tag_oid.is_some()
    {
        return Err(unavailable(
            Some(run),
            "main/tag/peeled no-op identities differ",
        ));
    }
    Ok((main, raw_oid, peeled_commit_oid))
}

#[allow(clippy::too_many_arguments)]
fn local_proof(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ReadyLimits,
    admission: JournalAdmission,
) -> Result<PendingProof, NoEffectInspection> {
    let Some(parent_limits) = limits.parent else {
        return Err(unavailable(None, "explicit retained parent limits absent"));
    };
    let (status, proof) = if admission == JournalAdmission::Ready {
        package_git_operation::ready_proof_cold(lease, namespace_token, scope, key, package_source)
    } else {
        package_git_operation::pending_proof_cold(
            lease,
            namespace_token,
            scope,
            key,
            package_source,
        )
    }
    .map_err(|e| recovery(None, e.to_string()))?;
    let Some(proof) = proof else {
        return match status {
            PendingInspection::Absent => Err(NoEffectInspection::NotReady { run_id: None }),
            PendingInspection::PendingIncomplete { run_id } => Err(NoEffectInspection::NotReady {
                run_id: Some(run_id),
            }),
            other => Err(recovery(
                None,
                format!("C.1 local bytes unavailable: {other:?}"),
            )),
        };
    };
    let run = proof.run_id.as_str();
    no_effect_shape(&proof.plan, run)?;
    let tree = if admission == JournalAdmission::Ready {
        package_git_tree::inspect_tree_ready_cold(
            lease,
            namespace_token,
            scope,
            key,
            package_source,
            binary,
        )
    } else {
        package_git_tree::inspect_tree_cold(
            lease,
            namespace_token,
            scope,
            key,
            package_source,
            binary,
        )
    }
    .map_err(|e| recovery(Some(run), e.to_string()))?;
    if !matches!(&tree, TreeInspection::TreeCompleteButNotReady { run_id, .. } if run_id == run) {
        return Err(recovery(
            Some(run),
            format!("vaulted tree unavailable: {tree:?}"),
        ));
    }
    let provenance = if admission == JournalAdmission::Ready {
        package_git_provenance::inspect_no_gitlinks_ready_cold(
            lease,
            namespace_token,
            scope,
            key,
            package_source,
            binary,
            limits.census,
        )
    } else {
        package_git_provenance::inspect_no_gitlinks_cold(
            lease,
            namespace_token,
            scope,
            key,
            package_source,
            binary,
            limits.census,
        )
    };
    if !matches!(&provenance, ProvenanceInspection::NoGitlinksRetainedButNotReady {
        run_id, flattened_submodules_sha256, ..
    } if run_id == run && flattened_submodules_sha256 == &proof.plan.flattened_submodules_sha256)
    {
        return Err(recovery(
            Some(run),
            format!("no-Gitlinks provenance unavailable: {provenance:?}"),
        ));
    }
    let parent = if admission == JournalAdmission::Ready {
        package_git_parent::inspect_parent_ready_cold(
            lease,
            namespace_token,
            scope,
            key,
            package_source,
            local_bare_source,
            binary,
            parent_limits,
        )
    } else {
        package_git_parent::inspect_parent_cold(
            lease,
            namespace_token,
            scope,
            key,
            package_source,
            local_bare_source,
            binary,
            parent_limits,
        )
    };
    if !matches!(&parent, ParentInspection::ClosureCompleteButNotReady { run_id, .. } if run_id == run)
    {
        return Err(recovery(
            Some(run),
            format!("exact retained parent unavailable: {parent:?}"),
        ));
    }
    if !matches!(journal::read_bytes(lease, run, "objects"), Ok(None))
        || !matches!(journal::read_bytes(lease, run, "ready"), Ok(None))
    {
        return Err(recovery(
            Some(run),
            "no-effect run has new-object/effect Ready record",
        ));
    }
    Ok(proof)
}

fn expected_seal(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    proof: &PendingProof,
) -> Result<NoEffectSeal, NoEffectInspection> {
    let run = proof.run_id.as_str();
    let namespace = SubmissionNamespace::from_host_token(namespace_token.to_owned())
        .map_err(|e| recovery(Some(run), e.to_string()))?;
    let row = SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)
        .map_err(|e| recovery(Some(run), e.to_string()))?
        .ok_or_else(|| recovery(Some(run), "C.1 reservation disappeared"))?;
    let plan_bytes =
        serde_json::to_vec(&proof.plan).map_err(|e| recovery(Some(run), e.to_string()))?;
    if row.retired || row.domain_run_id != run || row.plan_sha256 != sha(&plan_bytes) {
        return Err(recovery(Some(run), "C.1 run/plan changed"));
    }
    let (main, raw_tag, peeled) = no_effect_shape(&proof.plan, run)?;
    let stage = exact_bytes(lease, run, "stage")?;
    let tree = exact_bytes(lease, run, "tree")?;
    let provenance_pending = exact_bytes(lease, run, "provenance-pending")?;
    let provenance_landed = exact_bytes(lease, run, "provenance")?;
    let parent_pending = exact_bytes(lease, run, "parent-pending")?;
    let parent_landed = exact_bytes(lease, run, "parent")?;
    let parent: ParentReceipt =
        serde_json::from_slice(&parent_landed).map_err(|e| recovery(Some(run), e.to_string()))?;
    if parent.format != journal::PARENT_FORMAT
        || parent.run_id != run
        || parent.pending_sha256 != sha(&parent_pending)
        || parent.parent_oid != main
        || parent.parent_tree_oid != proof.plan.staged.git_tree_oid
        || parent.observed_refs != proof.plan.before_refs
        || parent.semantic_plan_sha256 != proof.plan.digest().unwrap_or_default()
        || serde_json::to_vec(&parent).ok().as_deref() != Some(parent_landed.as_slice())
    {
        return Err(recovery(
            Some(run),
            "parent main tree/tag witness differs from no-effect plan",
        ));
    }
    Ok(NoEffectSeal {
        format: FORMAT.into(),
        run_id: run.into(),
        namespace_id: namespace.id().into(),
        scope_context: scope.context.clone(),
        scope_target: scope.target.clone(),
        scope_principal: scope.principal.clone(),
        key_sha256: sha(key.as_bytes()),
        request_sha256: row.request_sha256,
        c1_plan_sha256: row.plan_sha256,
        semantic_plan_sha256: proof
            .plan
            .digest()
            .map_err(|e| recovery(Some(run), e.to_string()))?,
        lease_root_token: lease
            .package_git_journal_project()
            .root_correlation_token()
            .map_err(|e| recovery(Some(run), e.to_string()))?,
        source_root_token: proof.manifest.source_root_token.clone(),
        vault_root_token: proof.manifest.destination_root_token.clone(),
        object_root_token: parent.object_root_token,
        intent_sha256: proof.intent_sha256.clone(),
        payload_manifest_sha256: proof.payload_manifest_sha256.clone(),
        stage_receipt_sha256: sha(&stage),
        tree_receipt_sha256: sha(&tree),
        provenance_pending_sha256: sha(&provenance_pending),
        provenance_landed_sha256: sha(&provenance_landed),
        parent_pending_sha256: sha(&parent_pending),
        parent_landed_sha256: sha(&parent_landed),
        selected_main_oid: main.into(),
        selected_main_tree_oid: proof.plan.staged.git_tree_oid.clone(),
        selected_raw_tag_oid: raw_tag.into(),
        selected_peeled_commit_oid: peeled.into(),
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inspect_ready_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ReadyLimits,
) -> NoEffectInspection {
    let namespace = match SubmissionNamespace::from_host_token(namespace_token.to_owned()) {
        Ok(value) => value,
        Err(e) => return recovery(None, e.to_string()),
    };
    let row = match SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key) {
        Ok(Some(value)) => value,
        Ok(None) => return NoEffectInspection::NotReady { run_id: None },
        Err(e) => return recovery(None, e.to_string()),
    };
    let run = row.domain_run_id.clone();
    if row.retired {
        return recovery(Some(&run), "C.1 reservation retired");
    }
    let bytes = match journal::read_bytes(lease, &run, SEAL) {
        Ok(Some(value)) => value,
        Ok(None) if row.journal_admission == JournalAdmission::Pending => {
            return NoEffectInspection::NotReady { run_id: Some(run) };
        }
        Ok(None) => return recovery(Some(&run), "C.1 Ready lacks no-effect seal"),
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    let seal: NoEffectSeal = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    let proof = match local_proof(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
        row.journal_admission,
    ) {
        Ok(value) => value,
        Err(status) => {
            return recovery(
                Some(&run),
                format!("no-effect local proof unavailable: {status:?}"),
            );
        }
    };
    let expected = match expected_seal(lease, namespace_token, scope, key, &proof) {
        Ok(value) => value,
        Err(status) => return status,
    };
    if seal != expected || serde_json::to_vec(&seal).ok().as_deref() != Some(bytes.as_slice()) {
        return recovery(
            Some(&run),
            "no-effect seal differs from exact retained evidence",
        );
    }
    if !matches!(journal::read_bytes(lease, &run, SEAL), Ok(Some(ref now)) if now == &bytes) {
        return recovery(Some(&run), "no-effect seal changed during cold inspection");
    }
    let seal_sha256 = sha(&bytes);
    match row.journal_admission {
        JournalAdmission::Pending => NoEffectInspection::SealRetainedC1Pending {
            run_id: run,
            seal_sha256,
        },
        JournalAdmission::Ready => NoEffectInspection::ReadyForObservation {
            run_id: run,
            seal_sha256,
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn seal_ready(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: &Project,
    binary: &Path,
    limits: ReadyLimits,
) -> NoEffectInspection {
    seal_inner(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn seal_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: &Project,
    binary: &Path,
    limits: ReadyLimits,
    stop_after_seal: bool,
) -> NoEffectInspection {
    let current = inspect_ready_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(package_source),
        Some(local_bare_source),
        binary,
        limits,
    );
    let run = match &current {
        NoEffectInspection::ReadyForObservation { .. } => return current,
        NoEffectInspection::SealRetainedC1Pending { run_id, .. } => run_id.clone(),
        NoEffectInspection::NotReady {
            run_id: Some(run_id),
        } => run_id.clone(),
        _ => return current,
    };
    if matches!(current, NoEffectInspection::NotReady { .. }) {
        let proof = match local_proof(
            lease,
            namespace_token,
            scope,
            key,
            Some(package_source),
            Some(local_bare_source),
            binary,
            limits,
            JournalAdmission::Pending,
        ) {
            Ok(value) if value.run_id == run => value,
            Ok(_) => return recovery(Some(&run), "C.1 run changed before no-effect seal"),
            Err(status) => return status,
        };
        let seal = match expected_seal(lease, namespace_token, scope, key, &proof) {
            Ok(value) => value,
            Err(status) => return status,
        };
        if let Err(e) = journal::publish(lease, &run, SEAL, &seal) {
            return recovery(
                Some(&run),
                format!("no-effect seal publication uncertain: {e:#}"),
            );
        }
    }
    let before_mark = inspect_ready_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(package_source),
        Some(local_bare_source),
        binary,
        limits,
    );
    if !matches!(before_mark, NoEffectInspection::SealRetainedC1Pending { run_id: ref same, .. } if same == &run)
    {
        return recovery(
            Some(&run),
            format!("no-effect seal not cold-proved: {before_mark:?}"),
        );
    }
    if stop_after_seal {
        return before_mark;
    }
    let namespace = match SubmissionNamespace::from_host_token(namespace_token.to_owned()) {
        Ok(value) => value,
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    if let Err(e) = SubmissionReservations::mark_journal_ready(lease, &namespace, scope, key, &run)
    {
        return recovery(Some(&run), format!("C.1 Ready publication uncertain: {e}"));
    }
    inspect_ready_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(package_source),
        Some(local_bare_source),
        binary,
        limits,
    )
}

mod duplicate;
pub(crate) use duplicate::ready_duplicate_matches;
mod result;

#[cfg(test)]
mod tests;
