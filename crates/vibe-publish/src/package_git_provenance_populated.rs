//! Aggregate, run-retained populated-Gitlink byte provenance under C.1 Pending.
//! Every indexed link must have its own cold-proved byte store first.

use std::path::Path;

use vibe_lifecycle::{JournalAdmission, LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_gitlink::{self, GitIndexCensus};
use crate::package_git_operation::journal::{
    self, PopulatedIndexLink, PopulatedLanded, PopulatedLinkReceipt, PopulatedPending, sha,
};
use crate::package_git_operation::{self, PendingProof};
use crate::package_git_submodule::{SubmoduleInspection, SubmoduleLimits};
use crate::package_git_submodule_store::{self, LinkStoreInspection};
use crate::package_git_tree::{self, TreeInspection};

mod digest;
pub(crate) use digest::semantic_digest;
use digest::{index_links, proof_from_pending};

const PENDING: &str = "gitlink-aggregate-pending";
const LANDED: &str = "gitlink-aggregate";

pub(crate) struct ApprovedGitdir<'a> {
    pub path: &'a str,
    pub gitdir: &'a Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AggregateInspection {
    NotReady {
        run_id: Option<String>,
    },
    AggregateCompleteButNotReady {
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
fn uncertain(run: Option<&str>, reason: impl Into<String>) -> AggregateInspection {
    AggregateInspection::Uncertain {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}
fn unavailable(run: Option<&str>, reason: impl Into<String>) -> AggregateInspection {
    AggregateInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

fn prior(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    source: Option<&Project>,
    binary: &Path,
    admission: JournalAdmission,
) -> Result<PendingProof, AggregateInspection> {
    let (status, proof) = if admission == JournalAdmission::Ready {
        package_git_operation::ready_proof_cold(lease, namespace, scope, key, source)
    } else {
        package_git_operation::pending_proof_cold(lease, namespace, scope, key, source)
    }
    .map_err(|error| uncertain(None, error.to_string()))?;
    let Some(proof) = proof else {
        return match status {
            package_git_operation::PendingInspection::Absent => {
                Err(AggregateInspection::NotReady { run_id: None })
            }
            package_git_operation::PendingInspection::PendingIncomplete { run_id } => {
                Err(AggregateInspection::NotReady {
                    run_id: Some(run_id),
                })
            }
            other => Err(uncertain(
                None,
                format!("C.1 selected proof unavailable: {other:?}"),
            )),
        };
    };
    let tree = if admission == JournalAdmission::Ready {
        package_git_tree::inspect_tree_ready_cold(lease, namespace, scope, key, source, binary)
    } else {
        package_git_tree::inspect_tree_cold(lease, namespace, scope, key, source, binary)
    };
    match tree {
        Ok(TreeInspection::TreeCompleteButNotReady { run_id, .. }) if run_id == proof.run_id => {
            Ok(proof)
        }
        Ok(TreeInspection::NotReady { run_id }) => Err(AggregateInspection::NotReady { run_id }),
        Ok(other) => Err(uncertain(
            Some(&proof.run_id),
            format!("package tree proof unavailable: {other:?}"),
        )),
        Err(error) => Err(uncertain(Some(&proof.run_id), error.to_string())),
    }
}

fn index_row(
    index: &GitIndexCensus,
) -> Result<(String, String, String, Vec<PopulatedIndexLink>), String> {
    let GitIndexCensus::PopulatedCandidate {
        repository_root_token,
        package_prefix,
        index_sha256,
        links,
    } = index
    else {
        return Err("complete populated index witness absent".into());
    };
    if links.is_empty() || links.windows(2).any(|pair| pair[0].path >= pair[1].path) {
        return Err("populated Gitlink rows empty, duplicated, or unordered".into());
    }
    Ok((
        repository_root_token.clone(),
        package_prefix.clone(),
        index_sha256.clone(),
        links
            .iter()
            .map(|link| PopulatedIndexLink {
                path: link.path.clone(),
                indexed_oid: link.indexed_oid.clone(),
                checkout_head_oid: link.checkout_head_oid.clone(),
            })
            .collect(),
    ))
}

struct Collected {
    index: GitIndexCensus,
    link_receipts: Vec<PopulatedLinkReceipt>,
    flattened_sha256: String,
}

fn collect(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    proof: &PendingProof,
    source: &Project,
    approved: &[ApprovedGitdir<'_>],
    binary: &Path,
    limits: SubmoduleLimits,
    admission: JournalAdmission,
) -> Result<Collected, AggregateInspection> {
    let run = proof.run_id.as_str();
    let first = package_git_gitlink::census(source, binary, limits.census);
    let GitIndexCensus::PopulatedCandidate { links, .. } = &first else {
        return Err(unavailable(
            Some(run),
            format!("complete populated index unavailable: {first:?}"),
        ));
    };
    if approved.len() != links.len() || approved.iter().zip(links).any(|(a, b)| a.path != b.path) {
        return Err(unavailable(
            Some(run),
            "approved Git-dir set differs from every sorted indexed Gitlink",
        ));
    }
    let mut receipts = Vec::with_capacity(links.len());
    let mut proofs = Vec::with_capacity(links.len());
    for (link, held) in links.iter().zip(approved) {
        let row = match package_git_submodule_store::proved_link_receipt(
            lease,
            namespace,
            scope,
            key,
            &link.path,
            Some(source),
            Some(held.gitdir),
            binary,
            limits,
            admission,
        ) {
            Ok(value) => value,
            Err(LinkStoreInspection::NotReady { .. }) => {
                return Err(AggregateInspection::NotReady {
                    run_id: Some(run.into()),
                });
            }
            Err(LinkStoreInspection::Unavailable { reason, .. }) => {
                return Err(unavailable(Some(run), reason));
            }
            Err(other) => {
                return Err(uncertain(
                    Some(run),
                    format!("indexed Gitlink not cold-proved: {other:?}"),
                ));
            }
        };
        if row.path != link.path {
            return Err(uncertain(
                Some(run),
                "proved link path differs from full index",
            ));
        }
        receipts.push(PopulatedLinkReceipt {
            path: link.path.clone(),
            pending_sha256: sha(&row.pending_bytes),
            landed_sha256: sha(&row.landed_bytes),
        });
        proofs.push(proof_from_pending(&row.pending));
    }
    let digest = semantic_digest(&first, &proofs).map_err(|error| uncertain(Some(run), error))?;
    if digest != proof.plan.flattened_submodules_sha256 {
        return Err(uncertain(
            Some(run),
            "complete populated Gitlink digest differs from selected pre-C1 plan",
        ));
    }
    let second = package_git_gitlink::census(source, binary, limits.census);
    if first != second {
        return Err(uncertain(
            Some(run),
            "full Gitlink index changed while collecting per-link proof",
        ));
    }
    Ok(Collected {
        index: first,
        link_receipts: receipts,
        flattened_sha256: digest,
    })
}

fn pending_row(
    proof: &PendingProof,
    tree_bytes: &[u8],
    collected: &Collected,
) -> Result<PopulatedPending, String> {
    let (root, prefix, index_sha, links) = index_row(&collected.index)?;
    Ok(PopulatedPending {
        format: journal::POPULATED_PENDING_FORMAT.into(),
        run_id: proof.run_id.clone(),
        c1_plan_sha256: sha(&serde_json::to_vec(&proof.plan).map_err(|e| e.to_string())?),
        semantic_plan_sha256: proof.plan.digest().map_err(|e| e.to_string())?,
        intent_sha256: proof.intent_sha256.clone(),
        payload_manifest_sha256: proof.payload_manifest_sha256.clone(),
        tree_receipt_sha256: sha(tree_bytes),
        source_root_token: proof.manifest.source_root_token.clone(),
        index_repository_root_token: root,
        index_package_prefix: prefix,
        index_sha256: index_sha,
        links,
        link_receipts: collected.link_receipts.clone(),
        flattened_submodules_sha256: collected.flattened_sha256.clone(),
    })
}

fn landed_row(row: &PopulatedPending, bytes: &[u8]) -> PopulatedLanded {
    PopulatedLanded {
        format: journal::POPULATED_FORMAT.into(),
        run_id: row.run_id.clone(),
        pending_sha256: sha(bytes),
        flattened_submodules_sha256: row.flattened_submodules_sha256.clone(),
    }
}

pub(crate) fn retain_complete_pending(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    approved: &[ApprovedGitdir<'_>],
    binary: &Path,
    limits: SubmoduleLimits,
) -> AggregateInspection {
    retain_inner(
        lease,
        namespace,
        scope,
        key,
        source,
        approved,
        binary,
        limits,
        || {},
    )
}

fn retain_inner(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    source: &Project,
    approved: &[ApprovedGitdir<'_>],
    binary: &Path,
    limits: SubmoduleLimits,
    after_pending: impl FnOnce(),
) -> AggregateInspection {
    let proof = match prior(
        lease,
        namespace,
        scope,
        key,
        Some(source),
        binary,
        JournalAdmission::Pending,
    ) {
        Ok(v) => v,
        Err(s) => return s,
    };
    let run = proof.run_id.clone();
    let pending_existing = journal::read_bytes(lease, &run, PENDING);
    let landed_existing = journal::read_bytes(lease, &run, LANDED);
    match (pending_existing, landed_existing) {
        (Ok(None), Ok(None)) => {}
        (Ok(_), Ok(_)) => {
            return inspect_complete_cold(
                lease,
                namespace,
                scope,
                key,
                Some(source),
                Some(approved),
                binary,
                limits,
                JournalAdmission::Pending,
            );
        }
        (Err(error), _) | (_, Err(error)) => return uncertain(Some(&run), error.to_string()),
    }
    let first = match collect(
        lease,
        namespace,
        scope,
        key,
        &proof,
        source,
        approved,
        binary,
        limits,
        JournalAdmission::Pending,
    ) {
        Ok(v) => v,
        Err(s) => return s,
    };
    let tree_bytes = match journal::read_bytes(lease, &run, "tree") {
        Ok(Some(v)) => v,
        other => return uncertain(Some(&run), format!("tree receipt unavailable: {other:?}")),
    };
    let row = match pending_row(&proof, &tree_bytes, &first) {
        Ok(v) => v,
        Err(e) => return uncertain(Some(&run), e),
    };
    let pending_bytes = match journal::publish(lease, &run, PENDING, &row) {
        Ok(v) => v,
        Err(e) => {
            return uncertain(
                Some(&run),
                format!("populated Pending publication uncertain: {e:#}"),
            );
        }
    };
    after_pending();
    let second = match collect(
        lease,
        namespace,
        scope,
        key,
        &proof,
        source,
        approved,
        binary,
        limits,
        JournalAdmission::Pending,
    ) {
        Ok(v) => v,
        Err(s) => {
            return uncertain(
                Some(&run),
                format!("populated proof changed after Pending: {s:?}"),
            );
        }
    };
    if pending_row(&proof, &tree_bytes, &second).ok().as_ref() != Some(&row) {
        return uncertain(
            Some(&run),
            "complete populated Gitlink proof changed across Pending publication",
        );
    }
    if let Err(error) = journal::publish(lease, &run, LANDED, &landed_row(&row, &pending_bytes)) {
        return uncertain(
            Some(&run),
            format!("populated Landed publication uncertain: {error:#}"),
        );
    }
    inspect_complete_cold(
        lease,
        namespace,
        scope,
        key,
        Some(source),
        Some(approved),
        binary,
        limits,
        JournalAdmission::Pending,
    )
}

mod cold;
pub(crate) use cold::inspect_complete_cold;

#[cfg(test)]
#[path = "package_git_provenance_populated/tests.rs"]
mod tests;
