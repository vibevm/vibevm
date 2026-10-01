//! Cold-proved C.1/parent inputs and exact e2 control-object receipt.
use super::{ObjectInspection, ObjectProvenance, uncertain};
use crate::package_git_gitlink::CensusLimits;
use crate::package_git_operation::journal::{self, ObjectReceipt, sha};
use crate::package_git_operation::{self, PendingProof};
use crate::package_git_parent::{self, ParentInspection, ParentLimits};
use crate::package_git_plan::{
    RawRef, RepositoryObservation, RepositoryTarget, StagedCommit, TagRef,
};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_provenance_populated::{self, AggregateInspection, ApprovedGitdir};
use crate::package_git_submodule::SubmoduleLimits;
use crate::package_git_tree::git::LocalGit;
use sha2::{Digest, Sha256};
use std::path::Path;
use vibe_lifecycle::{JournalAdmission, LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

pub(super) struct Held {
    pub(super) run_id: String,
    pub(super) proof: PendingProof,
    pub(super) tree_bytes: Vec<u8>,
    pub(super) provenance: ObjectProvenance,
    pub(super) parent_receipt_sha256: Option<String>,
    pub(super) parent_oid: Option<String>,
    pub(super) objects: Project,
    pub(super) commit_bytes: Vec<u8>,
    pub(super) tag_bytes: Vec<u8>,
}

pub(super) fn held(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
    limits: CensusLimits,
    parent_limits: Option<ParentLimits>,
    local_bare_source: Option<&Project>,
    populated_limits: Option<SubmoduleLimits>,
    approved: Option<&[ApprovedGitdir<'_>]>,
    ready: bool,
) -> Result<Held, ObjectInspection> {
    let (run_id, provenance) = if let Some(populated_limits) = populated_limits {
        let admission = if ready {
            JournalAdmission::Ready
        } else {
            JournalAdmission::Pending
        };
        let observed = package_git_provenance_populated::inspect_complete_cold(
            lease,
            namespace_token,
            scope,
            key,
            source,
            approved,
            git_binary,
            populated_limits,
            admission,
        );
        let (run_id, flattened_submodules_sha256) = match observed {
            AggregateInspection::AggregateCompleteButNotReady {
                run_id,
                flattened_submodules_sha256,
                ..
            } => (run_id, flattened_submodules_sha256),
            AggregateInspection::NotReady { run_id } => {
                return Err(ObjectInspection::NotReady { run_id });
            }
            AggregateInspection::Unavailable { run_id, reason } => {
                return Err(ObjectInspection::Unavailable { run_id, reason });
            }
            AggregateInspection::Uncertain { run_id, reason } => {
                return Err(ObjectInspection::Uncertain { run_id, reason });
            }
        };
        let pending = journal::read_bytes(lease, &run_id, "gitlink-aggregate-pending")
            .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
            .ok_or_else(|| uncertain(Some(&run_id), "populated aggregate Pending vanished"))?;
        let landed = journal::read_bytes(lease, &run_id, "gitlink-aggregate")
            .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
            .ok_or_else(|| uncertain(Some(&run_id), "populated aggregate Landed vanished"))?;
        (
            run_id,
            ObjectProvenance::Populated {
                pending_sha256: sha(&pending),
                landed_sha256: sha(&landed),
                flattened_submodules_sha256,
            },
        )
    } else {
        let observed = if ready {
            package_git_provenance::inspect_no_gitlinks_ready_cold(
                lease,
                namespace_token,
                scope,
                key,
                source,
                git_binary,
                limits,
            )
        } else {
            package_git_provenance::inspect_no_gitlinks_cold(
                lease,
                namespace_token,
                scope,
                key,
                source,
                git_binary,
                limits,
            )
        };
        let run_id = match observed {
            ProvenanceInspection::NoGitlinksRetainedButNotReady { run_id, .. } => run_id,
            ProvenanceInspection::NotReady { run_id } => {
                return Err(ObjectInspection::NotReady { run_id });
            }
            ProvenanceInspection::NeedsCommitTreeProof { run_id } => {
                return Err(ObjectInspection::Unavailable {
                    run_id: Some(run_id),
                    reason: "populated Gitlinks require selected aggregate proof".into(),
                });
            }
            ProvenanceInspection::Unavailable { run_id, reason } => {
                return Err(ObjectInspection::Unavailable { run_id, reason });
            }
            ProvenanceInspection::Uncertain { run_id, reason } => {
                return Err(ObjectInspection::Uncertain { run_id, reason });
            }
        };
        let landed = journal::read_bytes(lease, &run_id, "provenance")
            .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
            .ok_or_else(|| uncertain(Some(&run_id), "no-Gitlinks provenance Landed vanished"))?;
        (
            run_id,
            ObjectProvenance::NoGitlinks {
                landed_sha256: sha(&landed),
            },
        )
    };
    let (status, proof) = if ready {
        package_git_operation::ready_proof_cold(lease, namespace_token, scope, key, source)
    } else {
        package_git_operation::pending_proof_cold(lease, namespace_token, scope, key, source)
    }
    .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
    let Some(proof) = proof else {
        return Err(uncertain(
            Some(&run_id),
            format!("C.1 Pending proof changed: {status:?}"),
        ));
    };
    if proof.run_id != run_id {
        return Err(uncertain(
            Some(&run_id),
            "C.1 run changed during local object admission",
        ));
    }
    if matches!(&provenance, ObjectProvenance::Populated { flattened_submodules_sha256, .. }
        if flattened_submodules_sha256 != &proof.plan.flattened_submodules_sha256)
    {
        return Err(uncertain(
            Some(&run_id),
            "populated aggregate digest differs from e2 plan",
        ));
    }
    let parentless = matches!(proof.plan.before_repo, RepositoryObservation::Absent)
        && matches!(proof.plan.before_refs.main, RawRef::Absent)
        && matches!(proof.plan.before_refs.version_tag, TagRef::Absent)
        && matches!(
            proof.plan.staged.commit,
            StagedCommit::New {
                parent_oid: None,
                ..
            }
        );
    let selected_parent = match (
        &proof.plan.target,
        &proof.plan.before_repo,
        &proof.plan.before_refs.main,
        &proof.plan.staged.commit,
    ) {
        (
            RepositoryTarget::DirectGitExisting { .. },
            RepositoryObservation::Present(_),
            RawRef::Present { oid: main },
            StagedCommit::New {
                parent_oid: Some(parent),
                ..
            },
        ) if parent == main => Some(main.clone()),
        _ => None,
    };
    if !parentless && selected_parent.is_none() {
        return Err(ObjectInspection::Unavailable { run_id: Some(run_id), reason: "only parentless new commit or exact DirectGitExisting child commit is supported; ReusedMain creates no object".into() });
    }
    if populated_limits.is_some() && !parentless {
        return Err(ObjectInspection::Unavailable {
            run_id: Some(run_id),
            reason: "populated Gitlinks currently support parentless New commit only".into(),
        });
    }
    let (parent_receipt_sha256, parent_oid) = if let Some(parent) = selected_parent {
        let Some(parent_limits) = parent_limits else {
            return Err(ObjectInspection::Unavailable {
                run_id: Some(run_id),
                reason: "held existing-main closure limits absent".into(),
            });
        };
        let parent_status = if ready {
            package_git_parent::inspect_parent_ready_cold(
                lease,
                namespace_token,
                scope,
                key,
                source,
                local_bare_source,
                git_binary,
                parent_limits,
            )
        } else {
            package_git_parent::inspect_parent_cold(
                lease,
                namespace_token,
                scope,
                key,
                source,
                local_bare_source,
                git_binary,
                parent_limits,
            )
        };
        match parent_status {
            ParentInspection::ClosureCompleteButNotReady {
                run_id: observed, ..
            } if observed == run_id => {}
            ParentInspection::NotReady { .. } | ParentInspection::Unavailable { .. } => {
                return Err(ObjectInspection::Unavailable {
                    run_id: Some(run_id),
                    reason: "exact existing-main parent closure is not retained".into(),
                });
            }
            other => {
                return Err(uncertain(
                    Some(&run_id),
                    format!("existing-main parent closure uncertain: {other:?}"),
                ));
            }
        }
        let bytes = journal::read_bytes(lease, &run_id, "parent")
            .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
            .ok_or_else(|| uncertain(Some(&run_id), "proved parent receipt vanished"))?;
        let row: journal::ParentReceipt = serde_json::from_slice(&bytes)
            .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
        if row.parent_oid != parent
            || row.run_id != run_id
            || row.semantic_plan_sha256
                != proof
                    .plan
                    .digest()
                    .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
            || serde_json::to_vec(&row).ok().as_deref() != Some(bytes.as_slice())
        {
            return Err(uncertain(
                Some(&run_id),
                "parent receipt changed after closure proof",
            ));
        }
        (Some(sha(&bytes)), Some(parent))
    } else {
        (None, None)
    };
    let commit_bytes = proof
        .plan
        .canonical_commit_bytes()
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "new commit bytes absent"))?;
    let tag_bytes = proof
        .plan
        .canonical_tag_bytes()
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "annotated tag bytes absent"))?;
    let tree_bytes = journal::read_bytes(lease, &run_id, "tree")
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved tree receipt vanished"))?;
    let objects = journal::objects(lease, &run_id, false)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved Git object root vanished"))?;
    if let Some(parent) = &parent_oid {
        let again = journal::read_bytes(lease, &run_id, "parent")
            .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
            .ok_or_else(|| {
                uncertain(
                    Some(&run_id),
                    "parent receipt vanished after object-root read",
                )
            })?;
        if parent_receipt_sha256.as_deref() != Some(sha(&again).as_str()) {
            return Err(uncertain(
                Some(&run_id),
                "parent receipt changed during child preparation",
            ));
        }
        let row: journal::ParentReceipt = serde_json::from_slice(&again)
            .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
        if row.parent_oid != *parent
            || row.object_root_token
                != objects
                    .root_correlation_token()
                    .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        {
            return Err(uncertain(
                Some(&run_id),
                "parent receipt object-root/parent identity differs",
            ));
        }
    }
    let _ = LocalGit::new(git_binary, &objects, proof.plan.oid_hex_len)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
    Ok(Held {
        run_id,
        proof,
        tree_bytes,
        provenance,
        parent_receipt_sha256,
        parent_oid,
        objects,
        commit_bytes,
        tag_bytes,
    })
}

pub(super) fn expected(held: &Held) -> anyhow::Result<ObjectReceipt> {
    let tag_oid = held
        .proof
        .plan
        .staged
        .annotated_tag_oid
        .clone()
        .ok_or_else(|| anyhow::anyhow!("annotated tag OID absent"))?;
    Ok(ObjectReceipt {
        format: journal::OBJECT_FORMAT.into(),
        run_id: held.run_id.clone(),
        c1_plan_sha256: sha(&serde_json::to_vec(&held.proof.plan)?),
        semantic_plan_sha256: held.proof.plan.digest()?,
        intent_sha256: held.proof.intent_sha256.clone(),
        payload_manifest_sha256: held.proof.payload_manifest_sha256.clone(),
        tree_receipt_sha256: sha(&held.tree_bytes),
        provenance: held.provenance.clone(),
        parent_receipt_sha256: held.parent_receipt_sha256.clone(),
        parent_oid: held.parent_oid.clone(),
        object_root_token: held.objects.root_correlation_token()?,
        root_tree_oid: held.proof.plan.staged.git_tree_oid.clone(),
        oid_hex_len: held.proof.plan.oid_hex_len,
        commit_oid: held.proof.plan.staged.tag_target_commit_oid.clone(),
        commit_size_bytes: held.commit_bytes.len(),
        commit_sha256: format!("sha256:{:x}", Sha256::digest(&held.commit_bytes)),
        tag_oid,
        tag_size_bytes: held.tag_bytes.len(),
        tag_sha256: format!("sha256:{:x}", Sha256::digest(&held.tag_bytes)),
    })
}
