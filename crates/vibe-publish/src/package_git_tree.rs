//! Retained local Git blob/tree objects under the package-Git C.1 Pending run.
//! No commit/tag, Gitlink proof, C.1 Ready, remote or public publication.

use std::path::Path;

use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_operation::journal::{self, TreeFile, TreeReceipt, sha};
use crate::package_git_operation::{self, PendingInspection, PendingProof};

mod git;
use git::LocalGit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TreeInspection {
    NotReady {
        run_id: Option<String>,
    },
    TreeCompleteButNotReady {
        run_id: String,
        root_tree_oid: String,
    },
    Uncertain {
        run_id: String,
        reason: String,
    },
    Unavailable {
        run_id: String,
        reason: String,
    },
}

fn no_proof(status: PendingInspection) -> TreeInspection {
    match status {
        PendingInspection::Uncertain { run_id, reason } => {
            TreeInspection::Uncertain { run_id, reason }
        }
        PendingInspection::RecoveryRequired { run_id } => TreeInspection::Uncertain {
            run_id,
            reason: "package Git C.1 recovery required".into(),
        },
        PendingInspection::PendingIncomplete { run_id } => TreeInspection::NotReady {
            run_id: Some(run_id),
        },
        PendingInspection::Absent | PendingInspection::Retired { .. } => {
            TreeInspection::NotReady { run_id: None }
        }
        PendingInspection::VaultCompleteButNotReady { run_id, .. } => TreeInspection::Uncertain {
            run_id,
            reason: "cold proof omitted complete payload".into(),
        },
    }
}

fn trusted_git(binary: &Path) -> bool {
    binary.is_absolute()
        && std::fs::symlink_metadata(binary)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

/// One local stage from a fully rechecked Pending vault. Existing object state
/// is only inspected: an incomplete prior attempt is never automatically run.
pub(crate) fn stage_tree_pending(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
) -> Result<TreeInspection, package_git_operation::OperationError> {
    let (status, proof) =
        package_git_operation::pending_proof_cold(lease, namespace_token, scope, key, source)?;
    let Some(proof) = proof else {
        return Ok(no_proof(status));
    };
    let run_id = proof.run_id.clone();
    if !trusted_git(git_binary) {
        return Ok(TreeInspection::Unavailable {
            run_id,
            reason: "trusted absolute regular Git binary is unavailable".into(),
        });
    }
    let existing_receipt = match journal::read_bytes(lease, &run_id, "tree") {
        Ok(value) => value,
        Err(error) => {
            return Ok(TreeInspection::Uncertain {
                run_id,
                reason: error.to_string(),
            });
        }
    };
    let existing_objects = match journal::objects(lease, &run_id, false) {
        Ok(value) => value,
        Err(error) => {
            return Ok(TreeInspection::Uncertain {
                run_id,
                reason: error.to_string(),
            });
        }
    };
    if existing_receipt.is_some() || existing_objects.is_some() {
        return inspect_tree_cold(lease, namespace_token, scope, key, source, git_binary);
    }
    if proof.plan.oid_hex_len == 64 {
        match LocalGit::probe_sha256(git_binary) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(TreeInspection::Unavailable {
                    run_id,
                    reason: "trusted local Git does not support the selected SHA-256 object format"
                        .into(),
                });
            }
            Err(error) => {
                return Ok(TreeInspection::Unavailable {
                    run_id,
                    reason: format!("SHA-256 Git format probe unavailable: {error:#}"),
                });
            }
        }
    }
    let objects = match journal::objects(lease, &run_id, true) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return Ok(TreeInspection::Uncertain {
                run_id,
                reason: "object root absent after create".into(),
            });
        }
        Err(error) => {
            return Ok(TreeInspection::Uncertain {
                run_id,
                reason: error.to_string(),
            });
        }
    };
    let staged = (|| -> anyhow::Result<TreeReceipt> {
        let git = LocalGit::new(git_binary, &objects, proof.plan.oid_hex_len)?;
        git.init()?;
        let mut files = Vec::with_capacity(proof.manifest.files.len());
        for file in &proof.manifest.files {
            files.push(git.hash_file(&proof.vault, file, proof.manifest.limits.max_file_bytes)?);
        }
        let root_tree_oid = git.make_tree(&files)?;
        if root_tree_oid != proof.plan.staged.git_tree_oid {
            anyhow::bail!("retained Git root tree differs from approved staged OID");
        }
        let output_cap = proof
            .manifest
            .limits
            .max_files
            .checked_mul(proof.manifest.limits.max_path_bytes.saturating_add(128))
            .ok_or_else(|| anyhow::anyhow!("Git tree listing cap overflow"))?;
        git.verify(
            &files,
            &root_tree_oid,
            proof.manifest.limits.max_file_bytes,
            output_cap,
        )?;
        Ok(receipt(&proof, &objects, files, root_tree_oid)?)
    })();
    let receipt = match staged {
        Ok(value) => value,
        Err(error) => {
            return Ok(TreeInspection::Uncertain {
                run_id,
                reason: format!("local Git tree stage incomplete: {error:#}"),
            });
        }
    };
    if let Err(error) = journal::publish(lease, &run_id, "tree", &receipt) {
        return Ok(TreeInspection::Uncertain {
            run_id,
            reason: format!("tree receipt publication uncertain: {error:#}"),
        });
    }
    inspect_tree_cold(lease, namespace_token, scope, key, source, git_binary)
}

fn receipt(
    proof: &PendingProof,
    objects: &Project,
    files: Vec<TreeFile>,
    root_tree_oid: String,
) -> anyhow::Result<TreeReceipt> {
    Ok(TreeReceipt {
        format: journal::TREE_FORMAT.into(),
        run_id: proof.run_id.clone(),
        c1_plan_sha256: sha(&serde_json::to_vec(&proof.plan)?),
        semantic_plan_sha256: proof.plan.digest()?,
        intent_sha256: proof.intent_sha256.clone(),
        payload_manifest_sha256: proof.payload_manifest_sha256.clone(),
        object_root_token: objects.root_correlation_token()?,
        root_tree_oid,
        oid_hex_len: proof.plan.oid_hex_len,
        files,
    })
}

/// Cold proof recomputes every blob SHA/size from actual Git object bytes and
/// compares the recursively listed tree to the rehashed payload manifest.
pub(crate) fn inspect_tree_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
) -> Result<TreeInspection, package_git_operation::OperationError> {
    let (status, proof) =
        package_git_operation::pending_proof_cold(lease, namespace_token, scope, key, source)?;
    let Some(proof) = proof else {
        return Ok(no_proof(status));
    };
    let run_id = proof.run_id.clone();
    let uncertain = |reason: String| TreeInspection::Uncertain {
        run_id: run_id.clone(),
        reason,
    };
    let bytes = match journal::read_bytes(lease, &run_id, "tree") {
        Ok(value) => value,
        Err(error) => return Ok(uncertain(error.to_string())),
    };
    let objects = match journal::objects(lease, &run_id, false) {
        Ok(value) => value,
        Err(error) => return Ok(uncertain(error.to_string())),
    };
    let both_absent = bytes.is_none() && objects.is_none();
    let (Some(bytes), Some(objects)) = (bytes, objects) else {
        return Ok(if both_absent {
            TreeInspection::NotReady {
                run_id: Some(run_id),
            }
        } else {
            uncertain("tree objects or receipt missing; never restage automatically".into())
        });
    };
    let row: TreeReceipt = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => return Ok(uncertain(error.to_string())),
    };
    let expected = match receipt(
        &proof,
        &objects,
        row.files.clone(),
        row.root_tree_oid.clone(),
    ) {
        Ok(value) => value,
        Err(error) => return Ok(uncertain(error.to_string())),
    };
    if row != expected
        || serde_json::to_vec(&row).ok().as_deref() != Some(bytes.as_slice())
        || row.root_tree_oid != proof.plan.staged.git_tree_oid
        || row.files.len() != proof.manifest.files.len()
        || row
            .files
            .iter()
            .zip(&proof.manifest.files)
            .any(|(tree, file)| {
                tree.path != file.path
                    || tree.size_bytes != file.size_bytes
                    || tree.sha256 != file.sha256
                    || tree.git_mode
                        != if file.unix_mode.is_some_and(|bits| bits & 0o111 != 0) {
                            "100755"
                        } else {
                            "100644"
                        }
            })
    {
        return Ok(uncertain(
            "tree receipt differs from exact C.1 payload/plan".into(),
        ));
    }
    if !trusted_git(git_binary) {
        return Ok(TreeInspection::Unavailable {
            run_id,
            reason: "trusted local Git binary unavailable for cold proof".into(),
        });
    }
    let git = match LocalGit::new(git_binary, &objects, row.oid_hex_len) {
        Ok(value) => value,
        Err(error) => return Ok(uncertain(error.to_string())),
    };
    let output_cap = proof
        .manifest
        .limits
        .max_files
        .checked_mul(proof.manifest.limits.max_path_bytes.saturating_add(128))
        .unwrap_or(0);
    if output_cap == 0
        || git
            .verify(
                &row.files,
                &row.root_tree_oid,
                proof.manifest.limits.max_file_bytes,
                output_cap,
            )
            .is_err()
    {
        return Ok(uncertain(
            "retained Git tree/blob objects differ from vaulted payload".into(),
        ));
    }
    if !matches!(journal::read_bytes(lease, &run_id, "tree"), Ok(Some(ref current)) if current == &bytes)
    {
        return Ok(uncertain("tree receipt changed after object proof".into()));
    }
    Ok(TreeInspection::TreeCompleteButNotReady {
        run_id,
        root_tree_oid: row.root_tree_oid,
    })
}

#[cfg(test)]
#[path = "package_git_tree/tests.rs"]
mod tests;
