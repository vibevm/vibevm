//! C.1 Ready retains exact local bytes; it never grants repository or Git effects.

use std::path::Path;

use vibe_lifecycle::{
    JournalAdmission, LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;

use crate::package_git_gitlink::{self, CensusLimits, GitIndexCensus};
use crate::package_git_objects::{self, ObjectInspection, ObjectProvenance};
use crate::package_git_operation::journal::{self, ObjectReceipt, ReadyProvenance, ReadySeal, sha};
use crate::package_git_operation::{self, PendingInspection, PendingProof};
use crate::package_git_parent::ParentLimits;
use crate::package_git_plan::{
    RawRef, RepositoryObservation, RepositoryTarget, StagedCommit, TagRef,
};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_tree::{self, TreeInspection};

const SEAL: &str = "ready";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReadyInspection {
    NotReady {
        run_id: Option<String>,
    },
    SealRetainedC1Pending {
        run_id: String,
        seal_sha256: String,
    },
    ReadyForApproval {
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
fn recovery(run: Option<&str>, reason: impl Into<String>) -> ReadyInspection {
    ReadyInspection::RecoveryRequired {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn unavailable(run: Option<&str>, reason: impl Into<String>) -> ReadyInspection {
    ReadyInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ReadyLimits {
    pub census: CensusLimits,
    pub parent: Option<ParentLimits>,
}

fn exact_bytes(
    lease: &LifecycleLease,
    run: &str,
    suffix: &str,
) -> Result<Vec<u8>, ReadyInspection> {
    journal::read_bytes(lease, run, suffix)
        .map_err(|e| recovery(Some(run), e.to_string()))?
        .ok_or_else(|| recovery(Some(run), format!("{suffix} owner receipt absent")))
}

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
) -> Result<PendingProof, ReadyInspection> {
    let result = if admission == JournalAdmission::Ready {
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
    let (status, proof) = result;
    let Some(proof) = proof else {
        return match status {
            PendingInspection::Absent => Err(ReadyInspection::NotReady { run_id: None }),
            PendingInspection::PendingIncomplete { run_id } => Err(ReadyInspection::NotReady {
                run_id: Some(run_id),
            }),
            other => Err(recovery(
                None,
                format!("C.1 exact local proof unavailable: {other:?}"),
            )),
        };
    };
    let run = proof.run_id.as_str();
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
            format!("exact retained Git tree not proved: {tree:?}"),
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
    match provenance {
        ProvenanceInspection::NoGitlinksRetainedButNotReady {
            run_id,
            flattened_submodules_sha256,
            ..
        } if run_id == run
            && flattened_submodules_sha256 == proof.plan.flattened_submodules_sha256 => {}
        ProvenanceInspection::NeedsCommitTreeProof { .. } => {
            return Err(unavailable(
                Some(run),
                "populated Gitlinks are not admitted to this Ready seal",
            ));
        }
        status @ ProvenanceInspection::NotReady { .. } if package_source.is_some() => {
            // A populated run has no no-Gitlinks receipt; this only classifies refusal.
            if matches!(
                package_git_gitlink::census(package_source.unwrap(), binary, limits.census),
                GitIndexCensus::PopulatedCandidate { .. }
            ) {
                return Err(unavailable(
                    Some(run),
                    "populated Gitlinks are not admitted to this Ready seal",
                ));
            }
            return Err(recovery(
                Some(run),
                format!("no-Gitlinks Landed proof unavailable: {status:?}"),
            ));
        }
        other => {
            return Err(recovery(
                Some(run),
                format!("no-Gitlinks Landed proof unavailable: {other:?}"),
            ));
        }
    }
    let object = match (
        &proof.plan.before_repo,
        &proof.plan.before_refs.main,
        &proof.plan.staged.commit,
    ) {
        (
            RepositoryObservation::Absent,
            RawRef::Absent,
            StagedCommit::New {
                parent_oid: None, ..
            },
        ) if matches!(proof.plan.before_refs.version_tag, TagRef::Absent) => {
            if limits.parent.is_some() || local_bare_source.is_some() {
                return Err(unavailable(
                    Some(run),
                    "parentless seal received a foreign parent source/limit",
                ));
            }
            if admission == JournalAdmission::Ready {
                package_git_objects::inspect_objects_ready_cold(
                    lease,
                    namespace_token,
                    scope,
                    key,
                    package_source,
                    binary,
                    limits.census,
                )
            } else {
                package_git_objects::inspect_objects_cold(
                    lease,
                    namespace_token,
                    scope,
                    key,
                    package_source,
                    binary,
                    limits.census,
                )
            }
        }
        (
            RepositoryObservation::Present(_),
            RawRef::Present { oid: main },
            StagedCommit::New {
                parent_oid: Some(parent),
                ..
            },
        ) if matches!(
            proof.plan.target,
            RepositoryTarget::DirectGitExisting { .. }
        ) && main == parent =>
        {
            let Some(parent_limits) = limits.parent else {
                return Err(unavailable(Some(run), "exact parent limits absent"));
            };
            if admission == JournalAdmission::Ready {
                package_git_objects::inspect_existing_main_objects_ready_cold(
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
                package_git_objects::inspect_existing_main_objects_cold(
                    lease,
                    namespace_token,
                    scope,
                    key,
                    package_source,
                    local_bare_source,
                    binary,
                    parent_limits,
                )
            }
        }
        (.., StagedCommit::ReusedMain { .. }) => {
            return Err(unavailable(
                Some(run),
                "ReusedMain no-op has no new-object receipt or no-effect terminal seal",
            ));
        }
        _ => {
            return Err(unavailable(
                Some(run),
                "package Git Ready supports only fully held new-commit profiles",
            ));
        }
    };
    if !matches!(&object, ObjectInspection::ObjectsCompleteButNotReady { run_id, .. } if run_id == run)
    {
        return Err(recovery(
            Some(run),
            format!("canonical commit/tag objects unproved: {object:?}"),
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
) -> Result<ReadySeal, ReadyInspection> {
    let run = proof.run_id.as_str();
    let namespace = SubmissionNamespace::from_host_token(namespace_token.to_owned())
        .map_err(|e| recovery(Some(run), e.to_string()))?;
    let row = SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key)
        .map_err(|e| recovery(Some(run), e.to_string()))?
        .ok_or_else(|| recovery(Some(run), "C.1 reservation vanished"))?;
    if row.retired || row.domain_run_id != run {
        return Err(recovery(
            Some(run),
            "C.1 reservation retired or changed run",
        ));
    }
    let plan_bytes =
        serde_json::to_vec(&proof.plan).map_err(|e| recovery(Some(run), e.to_string()))?;
    let semantic = proof
        .plan
        .digest()
        .map_err(|e| recovery(Some(run), e.to_string()))?;
    if row.plan_sha256 != sha(&plan_bytes) {
        return Err(recovery(Some(run), "C.1 plan differs from owner proof"));
    }
    let stage = exact_bytes(lease, run, "stage")?;
    let tree = exact_bytes(lease, run, "tree")?;
    let no_gitlinks_pending = exact_bytes(lease, run, "provenance-pending")?;
    let no_gitlinks_landed = exact_bytes(lease, run, "provenance")?;
    let objects = exact_bytes(lease, run, "objects")?;
    let object_row: ObjectReceipt =
        serde_json::from_slice(&objects).map_err(|e| recovery(Some(run), e.to_string()))?;
    let parent = match &proof.plan.staged.commit {
        StagedCommit::New {
            parent_oid: Some(_),
            ..
        } => Some((
            exact_bytes(lease, run, "parent-pending")?,
            exact_bytes(lease, run, "parent")?,
        )),
        _ => None,
    };
    let tag = proof
        .plan
        .staged
        .annotated_tag_oid
        .clone()
        .ok_or_else(|| unavailable(Some(run), "annotated tag object absent"))?;
    if object_row.format != journal::OBJECT_FORMAT
        || object_row.run_id != run
        || object_row.commit_oid != proof.plan.staged.tag_target_commit_oid
        || object_row.tag_oid != tag
        || object_row.root_tree_oid != proof.plan.staged.git_tree_oid
        || object_row.parent_oid
            != match &proof.plan.staged.commit {
                StagedCommit::New { parent_oid, .. } => parent_oid.clone(),
                _ => None,
            }
        || object_row.parent_receipt_sha256 != parent.as_ref().map(|(_, landed)| sha(landed))
        || object_row.provenance
            != (ObjectProvenance::NoGitlinks {
                landed_sha256: sha(&no_gitlinks_landed),
            })
        || serde_json::to_vec(&object_row).ok().as_deref() != Some(objects.as_slice())
    {
        return Err(recovery(
            Some(run),
            "e2 object receipt identity differs from plan/parent",
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
        provenance: ReadyProvenance::NoGitlinks {
            pending_sha256: sha(&no_gitlinks_pending),
            landed_sha256: sha(&no_gitlinks_landed),
        },
        parent_pending_sha256: parent.as_ref().map(|(pending, _)| sha(pending)),
        parent_landed_sha256: parent.as_ref().map(|(_, landed)| sha(landed)),
        object_receipt_sha256: sha(&objects),
        selected_tree_oid: proof.plan.staged.git_tree_oid.clone(),
        selected_commit_oid: proof.plan.staged.tag_target_commit_oid.clone(),
        selected_tag_oid: tag,
    })
}

pub(crate) fn inspect_ready_seal_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ReadyLimits,
) -> ReadyInspection {
    let namespace = match SubmissionNamespace::from_host_token(namespace_token.to_owned()) {
        Ok(v) => v,
        Err(e) => return recovery(None, e.to_string()),
    };
    let row = match SubmissionReservations::inspect_existing_key(lease, &namespace, scope, key) {
        Ok(Some(v)) => v,
        Ok(None) => return ReadyInspection::NotReady { run_id: None },
        Err(e) => return recovery(None, e.to_string()),
    };
    let run = row.domain_run_id.clone();
    if row.retired {
        return recovery(Some(&run), "C.1 reservation retired");
    }
    let bytes = match journal::read_bytes(lease, &run, SEAL) {
        Ok(Some(v)) => v,
        Ok(None) if row.journal_admission == JournalAdmission::Pending => {
            return ReadyInspection::NotReady { run_id: Some(run) };
        }
        Ok(None) => return recovery(Some(&run), "C.1 Ready lacks owner seal"),
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    let sealed: ReadySeal = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
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
        Ok(v) => v,
        Err(s) => return recovery(Some(&run), format!("sealed proof unavailable: {s:?}")),
    };
    let expected = match expected_seal(lease, namespace_token, scope, key, &proof) {
        Ok(v) => v,
        Err(s) => return s,
    };
    if sealed != expected || serde_json::to_vec(&sealed).ok().as_deref() != Some(bytes.as_slice()) {
        return recovery(
            Some(&run),
            "Ready seal differs from exact retained owner evidence",
        );
    }
    if !matches!(journal::read_bytes(lease, &run, SEAL), Ok(Some(ref now)) if now == &bytes) {
        return recovery(Some(&run), "Ready seal changed during cold inspection");
    }
    let seal_sha256 = sha(&bytes);
    match row.journal_admission {
        JournalAdmission::Pending => ReadyInspection::SealRetainedC1Pending {
            run_id: run,
            seal_sha256,
        },
        JournalAdmission::Ready => ReadyInspection::ReadyForApproval {
            run_id: run,
            seal_sha256,
        },
    }
}

pub(crate) fn seal_local_ready(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ReadyLimits,
) -> ReadyInspection {
    seal_inner(
        lease,
        namespace_token,
        scope,
        key,
        package_source,
        local_bare_source,
        binary,
        limits,
        || false,
    )
}

fn seal_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    local_bare_source: Option<&Project>,
    binary: &Path,
    limits: ReadyLimits,
    stop_after_seal: impl FnOnce() -> bool,
) -> ReadyInspection {
    let current = inspect_ready_seal_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(package_source),
        local_bare_source,
        binary,
        limits,
    );
    let run = match &current {
        ReadyInspection::ReadyForApproval { .. } => return current,
        ReadyInspection::SealRetainedC1Pending { run_id, .. } => run_id.clone(),
        ReadyInspection::NotReady {
            run_id: Some(run_id),
        } => run_id.clone(),
        other => return other.clone(),
    };
    if matches!(current, ReadyInspection::NotReady { .. }) {
        let proof = match local_proof(
            lease,
            namespace_token,
            scope,
            key,
            Some(package_source),
            local_bare_source,
            binary,
            limits,
            JournalAdmission::Pending,
        ) {
            Ok(v) if v.run_id == run => v,
            Ok(_) => return recovery(Some(&run), "C.1 run changed before Ready seal"),
            Err(s) => return s,
        };
        let seal = match expected_seal(lease, namespace_token, scope, key, &proof) {
            Ok(v) => v,
            Err(s) => return s,
        };
        if let Err(e) = journal::publish(lease, &run, SEAL, &seal) {
            return recovery(
                Some(&run),
                format!("Ready seal publication uncertain: {e:#}"),
            );
        }
    }
    let inspected = inspect_ready_seal_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(package_source),
        local_bare_source,
        binary,
        limits,
    );
    let ReadyInspection::SealRetainedC1Pending { run_id, .. } = &inspected else {
        return inspected;
    };
    if run_id != &run {
        return recovery(Some(&run), "Ready seal changed run before C.1 update");
    }
    if stop_after_seal() {
        return inspected;
    }
    let namespace = match SubmissionNamespace::from_host_token(namespace_token.to_owned()) {
        Ok(v) => v,
        Err(e) => return recovery(Some(&run), e.to_string()),
    };
    if let Err(e) = SubmissionReservations::mark_journal_ready(lease, &namespace, scope, key, &run)
    {
        return recovery(Some(&run), format!("C.1 Ready update uncertain: {e}"));
    }
    inspect_ready_seal_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(package_source),
        local_bare_source,
        binary,
        limits,
    )
}

#[cfg(test)]
#[path = "package_git_ready/tests.rs"]
mod tests;
