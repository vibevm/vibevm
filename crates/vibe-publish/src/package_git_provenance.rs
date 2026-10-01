//! Retained no-Gitlinks provenance for a package-Git C.1 Pending run.
//! A populated submodule needs separate exact commit-tree byte proof.

use std::path::Path;

use sha2::{Digest, Sha256};
use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_gitlink::{self, CensusLimits, GitIndexCensus};
use crate::package_git_operation::journal::{
    self, NoGitlinksEvidence, ProvenanceReceipt, TreeReceipt, sha,
};
use crate::package_git_operation::{self, PendingProof};
use crate::package_git_tree::{self, TreeInspection};

const DIGEST_DOMAIN: &[u8] = b"vibevm/package-git-no-gitlinks/e1\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProvenanceInspection {
    NotReady {
        run_id: Option<String>,
    },
    NeedsCommitTreeProof {
        run_id: String,
    },
    NoGitlinksRetainedButNotReady {
        run_id: String,
        flattened_submodules_sha256: String,
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

fn uncertain(run_id: Option<&str>, reason: impl Into<String>) -> ProvenanceInspection {
    ProvenanceInspection::Uncertain {
        run_id: run_id.map(str::to_owned),
        reason: reason.into(),
    }
}

fn evidence(
    census: GitIndexCensus,
    run_id: &str,
) -> Result<NoGitlinksEvidence, ProvenanceInspection> {
    match census {
        GitIndexCensus::NoGitRepository => Ok(NoGitlinksEvidence::NoGitRepository),
        GitIndexCensus::NoGitlinks {
            repository_root_token,
            package_prefix,
            index_sha256,
        } => Ok(NoGitlinksEvidence::NoGitlinks {
            repository_root_token,
            package_prefix,
            index_sha256,
        }),
        GitIndexCensus::PopulatedCandidate { .. } => {
            Err(ProvenanceInspection::NeedsCommitTreeProof {
                run_id: run_id.into(),
            })
        }
        GitIndexCensus::Uninitialized { path, .. } => Err(ProvenanceInspection::Unavailable {
            run_id: Some(run_id.into()),
            reason: format!("indexed Gitlink `{path}` has no initialized checkout"),
        }),
        GitIndexCensus::Unavailable(reason) | GitIndexCensus::Drift(reason) => {
            Err(uncertain(Some(run_id), reason))
        }
    }
}

pub(crate) fn digest(evidence: &NoGitlinksEvidence) -> anyhow::Result<String> {
    let bytes = serde_json::to_vec(evidence)?;
    let mut hash = Sha256::new();
    hash.update(DIGEST_DOMAIN);
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(&bytes);
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn tree_status(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
) -> Result<String, ProvenanceInspection> {
    let status =
        package_git_tree::inspect_tree_cold(lease, namespace_token, scope, key, source, git_binary)
            .map_err(|error| uncertain(None, error.to_string()))?;
    match status {
        TreeInspection::TreeCompleteButNotReady { run_id, .. } => Ok(run_id),
        TreeInspection::NotReady {
            run_id: Some(run_id),
        } => {
            let pending = journal::read_bytes(lease, &run_id, "provenance-pending");
            let landed = journal::read_bytes(lease, &run_id, "provenance");
            match (pending, landed) {
                (Ok(None), Ok(None)) => Err(ProvenanceInspection::NotReady {
                    run_id: Some(run_id),
                }),
                (Ok(_), Ok(_)) => Err(uncertain(
                    Some(&run_id),
                    "provenance journal exists while Git tree proof is missing",
                )),
                (Err(error), _) | (_, Err(error)) => {
                    Err(uncertain(Some(&run_id), error.to_string()))
                }
            }
        }
        TreeInspection::NotReady { run_id: None } => {
            Err(ProvenanceInspection::NotReady { run_id: None })
        }
        TreeInspection::Unavailable { run_id, reason } => Err(ProvenanceInspection::Unavailable {
            run_id: Some(run_id),
            reason,
        }),
        TreeInspection::Uncertain { run_id, reason } => Err(uncertain(Some(&run_id), reason)),
    }
}

fn held_proof(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
) -> Result<PendingProof, ProvenanceInspection> {
    let (status, proof) =
        package_git_operation::pending_proof_cold(lease, namespace_token, scope, key, source)
            .map_err(|error| uncertain(None, error.to_string()))?;
    proof.ok_or_else(|| uncertain(None, format!("C.1 Pending proof unavailable: {status:?}")))
}

fn expected(
    proof: &PendingProof,
    tree_receipt_bytes: &[u8],
    tree: &TreeReceipt,
    observed: NoGitlinksEvidence,
    format: &str,
    pending_sha256: Option<String>,
) -> anyhow::Result<ProvenanceReceipt> {
    let flattened_submodules_sha256 = digest(&observed)?;
    if flattened_submodules_sha256 != proof.plan.flattened_submodules_sha256 {
        anyhow::bail!("selected plan no-Gitlinks digest differs from captured evidence");
    }
    if tree.run_id != proof.run_id || tree.root_tree_oid != proof.plan.staged.git_tree_oid {
        anyhow::bail!("tree receipt identity differs from C.1 plan");
    }
    Ok(ProvenanceReceipt {
        format: format.into(),
        pending_sha256,
        run_id: proof.run_id.clone(),
        semantic_plan_sha256: proof.plan.digest()?,
        intent_sha256: proof.intent_sha256.clone(),
        payload_manifest_sha256: proof.payload_manifest_sha256.clone(),
        tree_receipt_sha256: sha(tree_receipt_bytes),
        root_tree_oid: tree.root_tree_oid.clone(),
        source_root_token: proof.manifest.source_root_token.clone(),
        flattened_submodules_sha256,
        evidence: observed,
    })
}

fn tree_receipt(lease: &LifecycleLease, run_id: &str) -> anyhow::Result<(Vec<u8>, TreeReceipt)> {
    let bytes = journal::read_bytes(lease, run_id, "tree")?
        .ok_or_else(|| anyhow::anyhow!("proved tree receipt vanished"))?;
    let row: TreeReceipt = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&row)? != bytes {
        anyhow::bail!("tree receipt is noncanonical");
    }
    Ok((bytes, row))
}

/// Stage one immutable no-Gitlinks receipt. Any existing receipt is inspected,
/// never replaced or recaptured as a second claim.
pub(crate) fn retain_no_gitlinks_pending(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    git_binary: &Path,
    limits: CensusLimits,
) -> ProvenanceInspection {
    retain_inner(
        lease,
        namespace_token,
        scope,
        key,
        source,
        git_binary,
        limits,
        || {},
    )
}

fn retain_inner(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    git_binary: &Path,
    limits: CensusLimits,
    after_pending: impl FnOnce(),
) -> ProvenanceInspection {
    let run_id = match tree_status(lease, namespace_token, scope, key, Some(source), git_binary) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let proof = match held_proof(lease, namespace_token, scope, key, Some(source)) {
        Ok(value) if value.run_id == run_id => value,
        Ok(_) => return uncertain(Some(&run_id), "C.1 run changed after tree proof"),
        Err(status) => return status,
    };
    let pending = match journal::read_bytes(lease, &run_id, "provenance-pending") {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let landed = match journal::read_bytes(lease, &run_id, "provenance") {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if pending.is_some() || landed.is_some() {
        return inspect_no_gitlinks_cold(
            lease,
            namespace_token,
            scope,
            key,
            Some(source),
            git_binary,
            limits,
        );
    }
    let first = match evidence(
        package_git_gitlink::census(source, git_binary, limits),
        &run_id,
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let (tree_bytes, tree) = match tree_receipt(lease, &run_id) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let pending_row = match expected(
        &proof,
        &tree_bytes,
        &tree,
        first.clone(),
        journal::PROVENANCE_PENDING_FORMAT,
        None,
    ) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let pending_bytes = match journal::publish(lease, &run_id, "provenance-pending", &pending_row) {
        Ok(value) => value,
        Err(error) => {
            return uncertain(
                Some(&run_id),
                format!("no-Gitlinks Pending publication uncertain: {error:#}"),
            );
        }
    };
    after_pending();
    let second = match evidence(
        package_git_gitlink::census(source, git_binary, limits),
        &run_id,
    ) {
        Ok(value) => value,
        Err(status) => {
            return uncertain(
                Some(&run_id),
                format!("source changed after provenance Pending: {status:?}"),
            );
        }
    };
    if second != first {
        return uncertain(
            Some(&run_id),
            "Git index state changed across receipt publication",
        );
    }
    let landed_row = match expected(
        &proof,
        &tree_bytes,
        &tree,
        first,
        journal::PROVENANCE_FORMAT,
        Some(sha(&pending_bytes)),
    ) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if let Err(error) = journal::publish(lease, &run_id, "provenance", &landed_row) {
        return uncertain(
            Some(&run_id),
            format!("no-Gitlinks Landed publication uncertain: {error:#}"),
        );
    }
    inspect_no_gitlinks_cold(
        lease,
        namespace_token,
        scope,
        key,
        Some(source),
        git_binary,
        limits,
    )
}

/// Cold historical proof is distinguished from a fresh held-source recheck.
/// Even with a current source, this is not a commit-tree or C.1 Ready proof.
pub(crate) fn inspect_no_gitlinks_cold(
    lease: &LifecycleLease,
    namespace_token: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    git_binary: &Path,
    limits: CensusLimits,
) -> ProvenanceInspection {
    let run_id = match tree_status(lease, namespace_token, scope, key, source, git_binary) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let proof = match held_proof(lease, namespace_token, scope, key, source) {
        Ok(value) if value.run_id == run_id => value,
        Ok(_) => {
            return uncertain(
                Some(&run_id),
                "C.1 run changed during cold provenance inspection",
            );
        }
        Err(status) => return status,
    };
    let pending_bytes = match journal::read_bytes(lease, &run_id, "provenance-pending") {
        Ok(Some(value)) => value,
        Ok(None) => {
            return match journal::read_bytes(lease, &run_id, "provenance") {
                Ok(None) => ProvenanceInspection::NotReady {
                    run_id: Some(run_id),
                },
                Ok(Some(_)) => uncertain(
                    Some(&run_id),
                    "Landed no-Gitlinks receipt lacks its Pending predecessor",
                ),
                Err(error) => uncertain(Some(&run_id), error.to_string()),
            };
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let pending_row: ProvenanceReceipt = match serde_json::from_slice(&pending_bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let bytes = match journal::read_bytes(lease, &run_id, "provenance") {
        Ok(Some(value)) => value,
        Ok(None) => {
            return uncertain(
                Some(&run_id),
                "no-Gitlinks Pending lacks an independently verified Landed record",
            );
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let row: ProvenanceReceipt = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let (tree_bytes, tree) = match tree_receipt(lease, &run_id) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let expected_pending = match expected(
        &proof,
        &tree_bytes,
        &tree,
        pending_row.evidence.clone(),
        journal::PROVENANCE_PENDING_FORMAT,
        None,
    ) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if pending_row != expected_pending
        || serde_json::to_vec(&pending_row).ok().as_deref() != Some(pending_bytes.as_slice())
    {
        return uncertain(
            Some(&run_id),
            "no-Gitlinks Pending differs from exact C.1 proof",
        );
    }
    let expected = match expected(
        &proof,
        &tree_bytes,
        &tree,
        pending_row.evidence.clone(),
        journal::PROVENANCE_FORMAT,
        Some(sha(&pending_bytes)),
    ) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if row != expected || serde_json::to_vec(&row).ok().as_deref() != Some(bytes.as_slice()) {
        return uncertain(
            Some(&run_id),
            "no-Gitlinks receipt differs from exact Pending proof",
        );
    }
    if let Some(source) = source {
        let current = match evidence(
            package_git_gitlink::census(source, git_binary, limits),
            &run_id,
        ) {
            Ok(value) => value,
            Err(status) => {
                return uncertain(
                    Some(&run_id),
                    format!("current source census differs: {status:?}"),
                );
            }
        };
        if current != row.evidence {
            return uncertain(
                Some(&run_id),
                "current Git index differs from retained evidence",
            );
        }
    }
    if !matches!(journal::read_bytes(lease, &run_id, "provenance"), Ok(Some(ref now)) if now == &bytes)
    {
        return uncertain(
            Some(&run_id),
            "no-Gitlinks receipt changed during cold inspection",
        );
    }
    if !matches!(journal::read_bytes(lease, &run_id, "provenance-pending"), Ok(Some(ref now)) if now == &pending_bytes)
    {
        return uncertain(
            Some(&run_id),
            "no-Gitlinks Pending changed during cold inspection",
        );
    }
    ProvenanceInspection::NoGitlinksRetainedButNotReady {
        run_id,
        flattened_submodules_sha256: row.flattened_submodules_sha256,
        source_reobserved: source.is_some(),
    }
}

#[cfg(test)]
#[path = "package_git_provenance/tests.rs"]
mod tests;
