//! Parentless populated-Gitlink Ready proof. This returns retained local
//! evidence only; Mode A, push and terminal owners have no route to it yet.

use super::*;
use crate::package_git_provenance_populated::ApprovedGitdir;
use crate::package_git_submodule::SubmoduleLimits;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PopulatedInspection {
    NotReady {
        run_id: Option<String>,
    },
    SealRetainedC1Pending {
        run_id: String,
        seal_sha256: String,
    },
    ReadyForLocalProof {
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
fn recovery(run: Option<&str>, reason: impl Into<String>) -> PopulatedInspection {
    PopulatedInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn unavailable(run: Option<&str>, reason: impl Into<String>) -> PopulatedInspection {
    PopulatedInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn exact(lease: &LifecycleLease, run: &str, suffix: &str) -> Result<Vec<u8>, PopulatedInspection> {
    journal::read_bytes(lease, run, suffix)
        .map_err(|e| recovery(Some(run), e.to_string()))?
        .ok_or_else(|| recovery(Some(run), format!("{suffix} receipt absent")))
}

#[allow(clippy::too_many_arguments)]
fn local_proof(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    approved: Option<&[ApprovedGitdir<'_>]>,
    binary: &Path,
    limits: SubmoduleLimits,
    admission: JournalAdmission,
) -> Result<PendingProof, PopulatedInspection> {
    if source.is_some() != approved.is_some() {
        return Err(unavailable(
            None,
            "source-present populated proof needs every approved Gitdir",
        ));
    }
    let (status, proof) = if admission == JournalAdmission::Ready {
        package_git_operation::ready_proof_cold(lease, namespace, scope, key, source)
    } else {
        package_git_operation::pending_proof_cold(lease, namespace, scope, key, source)
    }
    .map_err(|e| recovery(None, e.to_string()))?;
    let Some(proof) = proof else {
        return match status {
            PendingInspection::Absent => Err(PopulatedInspection::NotReady { run_id: None }),
            PendingInspection::PendingIncomplete { run_id } => Err(PopulatedInspection::NotReady {
                run_id: Some(run_id),
            }),
            other => Err(recovery(
                None,
                format!("C.1 local bytes unavailable: {other:?}"),
            )),
        };
    };
    let run = proof.run_id.as_str();
    if !matches!(
        (
            &proof.plan.target,
            &proof.plan.before_repo,
            &proof.plan.before_refs.main,
            &proof.plan.before_refs.version_tag,
            &proof.plan.staged.commit
        ),
        (
            RepositoryTarget::HostedCreate { .. },
            RepositoryObservation::Absent,
            RawRef::Absent,
            TagRef::Absent,
            StagedCommit::New {
                parent_oid: None,
                ..
            }
        )
    ) || proof.plan.staged.annotated_tag_oid.is_none()
    {
        return Err(unavailable(
            Some(run),
            "only parentless New hosted populated profile is Ready-capable",
        ));
    }
    let tree = if admission == JournalAdmission::Ready {
        package_git_tree::inspect_tree_ready_cold(lease, namespace, scope, key, source, binary)
    } else {
        package_git_tree::inspect_tree_cold(lease, namespace, scope, key, source, binary)
    }
    .map_err(|e| recovery(Some(run), e.to_string()))?;
    if !matches!(&tree, TreeInspection::TreeCompleteButNotReady { run_id, root_tree_oid }
        if run_id == run && root_tree_oid == &proof.plan.staged.git_tree_oid)
    {
        return Err(recovery(
            Some(run),
            format!("flattened vault tree unproved: {tree:?}"),
        ));
    }
    // The object owner's cold proof rehashes the complete aggregate and every
    // retained link store; the typed receipt is compared below against both
    // aggregate record bytes and the selected semantic digest.
    let objects = if admission == JournalAdmission::Ready {
        package_git_objects::inspect_parentless_populated_objects_ready_cold(
            lease, namespace, scope, key, source, approved, binary, limits,
        )
    } else {
        package_git_objects::inspect_parentless_populated_objects_cold(
            lease, namespace, scope, key, source, approved, binary, limits,
        )
    };
    if !matches!(&objects, ObjectInspection::ObjectsCompleteButNotReady { run_id, .. } if run_id == run)
    {
        return Err(recovery(
            Some(run),
            format!("canonical flattened commit/tag unproved: {objects:?}"),
        ));
    }
    if !matches!(
        journal::read_bytes(lease, run, "provenance-pending"),
        Ok(None)
    ) || !matches!(journal::read_bytes(lease, run, "provenance"), Ok(None))
        || !matches!(journal::read_bytes(lease, run, "parent-pending"), Ok(None))
        || !matches!(journal::read_bytes(lease, run, "parent"), Ok(None))
    {
        return Err(recovery(
            Some(run),
            "foreign no-Gitlinks or parent receipt occupies populated run",
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
) -> Result<ReadySeal, PopulatedInspection> {
    let run = proof.run_id.as_str();
    let namespace = SubmissionNamespace::from_host_token(namespace_token.to_owned())
        .map_err(|e| recovery(Some(run), e.to_string()))?;
    let row = SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)
        .map_err(|e| recovery(Some(run), e.to_string()))?
        .ok_or_else(|| recovery(Some(run), "C.1 reservation vanished"))?;
    let plan_bytes =
        serde_json::to_vec(&proof.plan).map_err(|e| recovery(Some(run), e.to_string()))?;
    let semantic = proof
        .plan
        .digest()
        .map_err(|e| recovery(Some(run), e.to_string()))?;
    if row.retired || row.domain_run_id != run || row.plan_sha256 != sha(&plan_bytes) {
        return Err(recovery(Some(run), "C.1 run/plan changed"));
    }
    let stage = exact(lease, run, "stage")?;
    let tree = exact(lease, run, "tree")?;
    let aggregate_pending = exact(lease, run, "gitlink-aggregate-pending")?;
    let aggregate_landed = exact(lease, run, "gitlink-aggregate")?;
    let object_bytes = exact(lease, run, "objects")?;
    let object_row: ObjectReceipt =
        serde_json::from_slice(&object_bytes).map_err(|e| recovery(Some(run), e.to_string()))?;
    let provenance = ReadyProvenance::Populated {
        pending_sha256: sha(&aggregate_pending),
        landed_sha256: sha(&aggregate_landed),
        flattened_submodules_sha256: proof.plan.flattened_submodules_sha256.clone(),
    };
    let expected_object_arm = ObjectProvenance::Populated {
        pending_sha256: sha(&aggregate_pending),
        landed_sha256: sha(&aggregate_landed),
        flattened_submodules_sha256: proof.plan.flattened_submodules_sha256.clone(),
    };
    if object_row.format != journal::OBJECT_FORMAT
        || object_row.run_id != run
        || object_row.c1_plan_sha256 != sha(&plan_bytes)
        || object_row.semantic_plan_sha256 != semantic
        || object_row.intent_sha256 != proof.intent_sha256
        || object_row.payload_manifest_sha256 != proof.payload_manifest_sha256
        || object_row.tree_receipt_sha256 != sha(&tree)
        || object_row.provenance != expected_object_arm
        || object_row.parent_oid.is_some()
        || object_row.parent_receipt_sha256.is_some()
        || object_row.root_tree_oid != proof.plan.staged.git_tree_oid
        || object_row.commit_oid != proof.plan.staged.tag_target_commit_oid
        || object_row.tag_oid
            != proof
                .plan
                .staged
                .annotated_tag_oid
                .as_deref()
                .unwrap_or_default()
        || serde_json::to_vec(&object_row).ok().as_deref() != Some(object_bytes.as_slice())
    {
        return Err(recovery(
            Some(run),
            "e3 object receipt differs from exact populated plan/aggregate",
        ));
    }
    Ok(ReadySeal {
        format: journal::READY_FORMAT.into(),
        run_id: run.into(),
        namespace_id: namespace.id().into(),
        scope_context: scope.context.clone(),
        scope_target: scope.target.clone(),
        scope_principal: scope.principal.clone(),
        key_sha256: sha(key.as_bytes()),
        request_sha256: row.request_sha256,
        c1_plan_sha256: row.plan_sha256,
        semantic_plan_sha256: semantic,
        lease_root_token: lease
            .package_git_journal_project()
            .root_correlation_token()
            .map_err(|e| recovery(Some(run), e.to_string()))?,
        source_root_token: proof.manifest.source_root_token.clone(),
        vault_root_token: proof.manifest.destination_root_token.clone(),
        object_root_token: object_row.object_root_token,
        intent_sha256: proof.intent_sha256.clone(),
        payload_manifest_sha256: proof.payload_manifest_sha256.clone(),
        stage_receipt_sha256: sha(&stage),
        tree_receipt_sha256: sha(&tree),
        provenance,
        parent_pending_sha256: None,
        parent_landed_sha256: None,
        object_receipt_sha256: sha(&object_bytes),
        selected_tree_oid: proof.plan.staged.git_tree_oid.clone(),
        selected_commit_oid: proof.plan.staged.tag_target_commit_oid.clone(),
        selected_tag_oid: proof.plan.staged.annotated_tag_oid.clone().unwrap(),
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inspect_ready_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    approved: Option<&[ApprovedGitdir<'_>]>,
    binary: &Path,
    limits: SubmoduleLimits,
) -> PopulatedInspection {
    let namespace = match SubmissionNamespace::from_host_token(namespace_token.to_owned()) {
        Ok(value) => value,
        Err(e) => return recovery(None, e.to_string()),
    };
    let row = match SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key) {
        Ok(Some(value)) => value,
        Ok(None) => return PopulatedInspection::NotReady { run_id: None },
        Err(e) => return recovery(None, e.to_string()),
    };
    let run = row.domain_run_id.clone();
    if row.retired {
        return recovery(Some(&run), "C.1 reservation retired");
    }
    let bytes = match journal::read_bytes(lease, &run, SEAL) {
        Ok(Some(value)) => value,
        Ok(None) if row.journal_admission == JournalAdmission::Pending => {
            return PopulatedInspection::NotReady { run_id: Some(run) };
        }
        Ok(None) => return recovery(Some(&run), "C.1 Ready lacks populated seal"),
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    let seal: ReadySeal = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    let proof = match local_proof(
        lease,
        namespace_token,
        scope,
        key,
        source,
        approved,
        binary,
        limits,
        row.journal_admission,
    ) {
        Ok(value) => value,
        Err(status) => {
            return recovery(
                Some(&run),
                format!("populated seal proof unavailable: {status:?}"),
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
            "populated e2 Ready seal differs from exact retained proof",
        );
    }
    if !matches!(journal::read_bytes(lease, &run, SEAL), Ok(Some(ref now)) if now == &bytes) {
        return recovery(
            Some(&run),
            "populated Ready seal changed during cold inspection",
        );
    }
    let seal_sha256 = sha(&bytes);
    match row.journal_admission {
        JournalAdmission::Pending => PopulatedInspection::SealRetainedC1Pending {
            run_id: run,
            seal_sha256,
        },
        JournalAdmission::Ready => PopulatedInspection::ReadyForLocalProof {
            run_id: run,
            seal_sha256,
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn seal_parentless_ready(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    approved: &[ApprovedGitdir<'_>],
    binary: &Path,
    limits: SubmoduleLimits,
) -> PopulatedInspection {
    seal_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        approved,
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
    source: &Project,
    approved: &[ApprovedGitdir<'_>],
    binary: &Path,
    limits: SubmoduleLimits,
    stop_after_seal: bool,
) -> PopulatedInspection {
    let current = inspect_ready_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(source),
        Some(approved),
        binary,
        limits,
    );
    let run = match &current {
        PopulatedInspection::ReadyForLocalProof { .. } => return current,
        PopulatedInspection::SealRetainedC1Pending { run_id, .. }
        | PopulatedInspection::NotReady {
            run_id: Some(run_id),
        } => run_id.clone(),
        _ => return current,
    };
    if matches!(current, PopulatedInspection::NotReady { .. }) {
        let proof = match local_proof(
            lease,
            namespace_token,
            scope,
            key,
            Some(source),
            Some(approved),
            binary,
            limits,
            JournalAdmission::Pending,
        ) {
            Ok(value) if value.run_id == run => value,
            Ok(_) => return recovery(Some(&run), "C.1 run changed before populated seal"),
            Err(status) => return status,
        };
        let seal = match expected_seal(lease, namespace_token, scope, key, &proof) {
            Ok(value) => value,
            Err(status) => return status,
        };
        if let Err(e) = journal::publish(lease, &run, SEAL, &seal) {
            return recovery(
                Some(&run),
                format!("populated seal publication uncertain: {e:#}"),
            );
        }
    }
    let pending = inspect_ready_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(source),
        Some(approved),
        binary,
        limits,
    );
    if !matches!(&pending, PopulatedInspection::SealRetainedC1Pending { run_id, .. } if run_id == &run)
    {
        return recovery(
            Some(&run),
            format!("populated seal not cold-proved: {pending:?}"),
        );
    }
    if stop_after_seal {
        return pending;
    }
    let namespace = match SubmissionNamespace::from_host_token(namespace_token.to_owned()) {
        Ok(value) => value,
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    if let Err(e) = SubmissionReservations::mark_journal_ready(lease, &namespace, scope, key, &run)
    {
        return recovery(Some(&run), format!("C.1 Ready update uncertain: {e}"));
    }
    inspect_ready_cold(
        lease,
        namespace_token,
        scope,
        key,
        None,
        None,
        binary,
        limits,
    )
}

#[cfg(test)]
mod tests;
