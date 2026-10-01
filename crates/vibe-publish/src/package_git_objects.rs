//! Retained local commit and annotated tag for an e2 parentless package plan.
//! C.1 remains Pending; no remote repository or publication effect is invoked.

use std::path::Path;

use sha2::{Digest, Sha256};
use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_gitlink::CensusLimits;
use crate::package_git_operation::journal::{self, ObjectReceipt, sha};
use crate::package_git_operation::{self, PendingProof};
use crate::package_git_plan::{RawRef, RepositoryObservation, StagedCommit, TagRef};
use crate::package_git_provenance::{self, ProvenanceInspection};
use crate::package_git_tree::git::LocalGit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObjectInspection {
    NotReady {
        run_id: Option<String>,
    },
    ObjectsCompleteButNotReady {
        run_id: String,
        source_reobserved: bool,
    },
    Unavailable {
        run_id: Option<String>,
        reason: String,
    },
    Uncertain {
        run_id: Option<String>,
        reason: String,
    },
}

fn uncertain(run: Option<&str>, reason: impl Into<String>) -> ObjectInspection {
    ObjectInspection::Uncertain {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

struct Held {
    run_id: String,
    proof: PendingProof,
    tree_bytes: Vec<u8>,
    provenance_bytes: Vec<u8>,
    objects: Project,
    commit_bytes: Vec<u8>,
    tag_bytes: Vec<u8>,
}

fn held(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
    limits: CensusLimits,
) -> Result<Held, ObjectInspection> {
    let run_id = match package_git_provenance::inspect_no_gitlinks_cold(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits,
    ) {
        ProvenanceInspection::NoGitlinksRetainedButNotReady { run_id, .. } => run_id,
        ProvenanceInspection::NotReady { run_id } => {
            return Err(ObjectInspection::NotReady { run_id });
        }
        ProvenanceInspection::NeedsCommitTreeProof { run_id } => {
            return Err(ObjectInspection::Unavailable {
                run_id: Some(run_id),
                reason: "populated Gitlinks need exact commit-tree byte proof".into(),
            });
        }
        ProvenanceInspection::Unavailable { run_id, reason } => {
            return Err(ObjectInspection::Unavailable { run_id, reason });
        }
        ProvenanceInspection::Uncertain { run_id, reason } => {
            return Err(ObjectInspection::Uncertain { run_id, reason });
        }
    };
    let (status, proof) =
        package_git_operation::pending_proof_cold(lease, namespace_token, scope, key, source)
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
    let supported = matches!(proof.plan.before_repo, RepositoryObservation::Absent)
        && matches!(proof.plan.before_refs.main, RawRef::Absent)
        && matches!(proof.plan.before_refs.version_tag, TagRef::Absent)
        && matches!(
            proof.plan.staged.commit,
            StagedCommit::New {
                parent_oid: None,
                ..
            }
        );
    if !supported {
        return Err(ObjectInspection::Unavailable {
            run_id: Some(run_id),
            reason: "existing-main parent or reused-main object custody is not available".into(),
        });
    }
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
    let provenance_bytes = journal::read_bytes(lease, &run_id, "provenance")
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved provenance Landed vanished"))?;
    let objects = journal::objects(lease, &run_id, false)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?
        .ok_or_else(|| uncertain(Some(&run_id), "proved Git object root vanished"))?;
    let _ = LocalGit::new(git_binary, &objects, proof.plan.oid_hex_len)
        .map_err(|error| uncertain(Some(&run_id), error.to_string()))?;
    Ok(Held {
        run_id,
        proof,
        tree_bytes,
        provenance_bytes,
        objects,
        commit_bytes,
        tag_bytes,
    })
}

fn expected(held: &Held) -> anyhow::Result<ObjectReceipt> {
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
        provenance_receipt_sha256: sha(&held.provenance_bytes),
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

/// Write only absent commit/tag objects in the already staged, cold-proved
/// bare Git object root. A prior object without receipt is uncertain, not ours.
pub(crate) fn stage_parentless_objects_pending(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    git_binary: &Path,
    limits: CensusLimits,
) -> ObjectInspection {
    let held = match held(
        lease,
        namespace_token,
        scope,
        key,
        Some(source),
        git_binary,
        limits,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run_id = held.run_id.clone();
    match journal::read_bytes(lease, &run_id, "objects") {
        Ok(Some(_)) => {
            return inspect_objects_cold(
                lease,
                namespace_token,
                scope,
                key,
                Some(source),
                git_binary,
                limits,
            );
        }
        Ok(None) => {}
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    }
    let git = match LocalGit::new(git_binary, &held.objects, held.proof.plan.oid_hex_len) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let planned = match expected(&held) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let stage = (|| -> anyhow::Result<()> {
        if git.object_present(&planned.commit_oid)? || git.object_present(&planned.tag_oid)? {
            anyhow::bail!("unreceipted planned commit/tag object already exists; never restage");
        }
        if git.write_control_object("commit", &held.commit_bytes)? != planned.commit_oid {
            anyhow::bail!("Git wrote a commit OID different from e2 plan");
        }
        if git.verify_control_object("commit", &planned.commit_oid, &held.commit_bytes)?
            != planned.commit_sha256
        {
            anyhow::bail!("retained commit SHA-256 differs from e2 plan");
        }
        if git.write_control_object("tag", &held.tag_bytes)? != planned.tag_oid {
            anyhow::bail!("Git wrote a tag OID different from e2 plan");
        }
        if git.verify_control_object("tag", &planned.tag_oid, &held.tag_bytes)?
            != planned.tag_sha256
        {
            anyhow::bail!("retained tag SHA-256 differs from e2 plan");
        }
        Ok(())
    })();
    if let Err(error) = stage {
        return uncertain(
            Some(&run_id),
            format!("local commit/tag stage incomplete: {error:#}"),
        );
    }
    if let Err(error) = journal::publish(lease, &run_id, "objects", &planned) {
        return uncertain(
            Some(&run_id),
            format!("object receipt publication uncertain: {error:#}"),
        );
    }
    inspect_objects_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(source),
        git_binary,
        limits,
    )
}

/// Read-only cold proof reopens both retained Git object contents. Historical
/// source-less attach reports its narrower witness explicitly.
pub(crate) fn inspect_objects_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
    limits: CensusLimits,
) -> ObjectInspection {
    let held = match held(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run_id = held.run_id.clone();
    let bytes = match journal::read_bytes(lease, &run_id, "objects") {
        Ok(Some(value)) => value,
        Ok(None) => {
            let git = match LocalGit::new(git_binary, &held.objects, held.proof.plan.oid_hex_len) {
                Ok(value) => value,
                Err(error) => return uncertain(Some(&run_id), error.to_string()),
            };
            match (
                git.object_present(&held.proof.plan.staged.tag_target_commit_oid),
                held.proof
                    .plan
                    .staged
                    .annotated_tag_oid
                    .as_deref()
                    .map(|oid| git.object_present(oid)),
            ) {
                (Ok(false), Some(Ok(false))) => {
                    return ObjectInspection::NotReady {
                        run_id: Some(run_id),
                    };
                }
                _ => {
                    return uncertain(
                        Some(&run_id),
                        "planned Git object exists or occupancy uncertain without a receipt",
                    );
                }
            }
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let row: ObjectReceipt = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let expected = match expected(&held) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if row != expected || serde_json::to_vec(&row).ok().as_deref() != Some(bytes.as_slice()) {
        return uncertain(
            Some(&run_id),
            "object receipt differs from exact C.1/e2 plan witnesses",
        );
    }
    let git = match LocalGit::new(git_binary, &held.objects, held.proof.plan.oid_hex_len) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if git
        .verify_control_object("commit", &row.commit_oid, &held.commit_bytes)
        .ok()
        .as_deref()
        != Some(row.commit_sha256.as_str())
        || git
            .verify_control_object("tag", &row.tag_oid, &held.tag_bytes)
            .ok()
            .as_deref()
            != Some(row.tag_sha256.as_str())
    {
        return uncertain(
            Some(&run_id),
            "actual retained commit/tag type, size or bytes differ from e2 plan",
        );
    }
    if !matches!(journal::read_bytes(lease, &run_id, "objects"), Ok(Some(ref current)) if current == &bytes)
    {
        return uncertain(
            Some(&run_id),
            "object receipt changed during cold verification",
        );
    }
    ObjectInspection::ObjectsCompleteButNotReady {
        run_id,
        source_reobserved: source.is_some(),
    }
}

#[cfg(test)]
#[path = "package_git_objects/tests.rs"]
mod tests;
