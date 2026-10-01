//! One Gitlink's local byte-only object custody under package-Git C.1 Pending.
//! No remote fetch, parent-history claim, C.1 Ready or publication effect.

use std::path::Path;

use vibe_lifecycle::{LifecycleLease, SubmissionScope};
use vibe_safefs::Project;

use crate::package_git_gitlink::GitlinkCandidate;
use crate::package_git_operation::journal::{self, GitlinkLanded, GitlinkPending, sha};
use crate::package_git_operation::{self, PendingProof};
use crate::package_git_submodule::{self, RawInspection, SubmoduleInspection, SubmoduleLimits};
use crate::package_git_tree::git::LocalGit;
use crate::package_git_tree::{self, TreeInspection};

mod import;
mod receipt;
use receipt::{landed_row, matches_raw, path_sha, pending_row, suffixes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkStoreInspection {
    NotReady {
        run_id: Option<String>,
    },
    ByteStoreCompleteButNotReady {
        run_id: String,
        path_sha256: String,
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

fn uncertain(run: Option<&str>, reason: impl Into<String>) -> LinkStoreInspection {
    LinkStoreInspection::Uncertain {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

fn unavailable(run: Option<&str>, reason: impl Into<String>) -> LinkStoreInspection {
    LinkStoreInspection::Unavailable {
        run_id: run.map(str::to_owned),
        reason: reason.into(),
    }
}

fn prior(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: Option<&Project>,
    binary: &Path,
) -> Result<PendingProof, LinkStoreInspection> {
    let (status, proof) =
        package_git_operation::pending_proof_cold(lease, namespace, scope, key, package_source)
            .map_err(|error| uncertain(None, error.to_string()))?;
    let Some(proof) = proof else {
        return match status {
            package_git_operation::PendingInspection::Absent => {
                Err(LinkStoreInspection::NotReady { run_id: None })
            }
            package_git_operation::PendingInspection::PendingIncomplete { run_id } => {
                Err(LinkStoreInspection::NotReady {
                    run_id: Some(run_id),
                })
            }
            other => Err(uncertain(
                None,
                format!("C.1 Pending payload unavailable: {other:?}"),
            )),
        };
    };
    match package_git_tree::inspect_tree_cold(lease, namespace, scope, key, package_source, binary)
    {
        Ok(TreeInspection::TreeCompleteButNotReady { run_id, .. }) if run_id == proof.run_id => {
            Ok(proof)
        }
        Ok(TreeInspection::NotReady { run_id }) => Err(LinkStoreInspection::NotReady { run_id }),
        Ok(other) => Err(uncertain(
            Some(&proof.run_id),
            format!("retained package tree proof unavailable: {other:?}"),
        )),
        Err(error) => Err(uncertain(Some(&proof.run_id), error.to_string())),
    }
}

pub(crate) fn stage_one_pending(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    approved_gitdir: &Project,
    candidate: &GitlinkCandidate,
    binary: &Path,
    limits: SubmoduleLimits,
) -> LinkStoreInspection {
    stage_inner(
        lease,
        namespace,
        scope,
        key,
        package_source,
        approved_gitdir,
        candidate,
        binary,
        limits,
        |_| {},
    )
}

fn stage_inner(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    package_source: &Project,
    approved_gitdir: &Project,
    candidate: &GitlinkCandidate,
    binary: &Path,
    limits: SubmoduleLimits,
    after_object: impl FnMut(usize),
) -> LinkStoreInspection {
    let proof = match prior(lease, namespace, scope, key, Some(package_source), binary) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run_id = proof.run_id.clone();
    let path_hash = path_sha(&candidate.path);
    let (pending_name, landed_name) = suffixes(&path_hash);
    let pending_exists = journal::read_bytes(lease, &run_id, &pending_name);
    let landed_exists = journal::read_bytes(lease, &run_id, &landed_name);
    let store_exists = journal::gitlink_store(lease, &run_id, &path_hash, false);
    match (pending_exists, landed_exists, store_exists) {
        (Ok(None), Ok(None), Ok(None)) => {}
        (Ok(_), Ok(_), Ok(_)) => {
            return inspect_one_cold(
                lease,
                namespace,
                scope,
                key,
                &candidate.path,
                Some(package_source),
                Some(approved_gitdir),
                binary,
                limits,
            );
        }
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            return uncertain(Some(&run_id), error.to_string());
        }
    }
    let exact = match package_git_submodule::compare_one(
        package_source,
        &proof.vault,
        &proof.manifest,
        candidate,
        approved_gitdir,
        binary,
        limits,
    ) {
        value @ SubmoduleInspection::ExactCandidate { .. } => value,
        SubmoduleInspection::Unavailable(reason) => return unavailable(Some(&run_id), reason),
        SubmoduleInspection::Drift(reason) => return uncertain(Some(&run_id), reason),
    };
    if candidate.indexed_oid.len() == 64 && !matches!(LocalGit::probe_sha256(binary), Ok(true)) {
        return unavailable(
            Some(&run_id),
            "local Git does not support selected SHA-256 object format",
        );
    }
    let tree_bytes = match journal::read_bytes(lease, &run_id, "tree") {
        Ok(Some(value)) => value,
        other => {
            return uncertain(
                Some(&run_id),
                format!("tree receipt unavailable: {other:?}"),
            );
        }
    };
    let pending = match pending_row(&proof, &tree_bytes, candidate, &exact) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let pending_bytes = match journal::publish(lease, &run_id, &pending_name, &pending) {
        Ok(value) => value,
        Err(error) => {
            return uncertain(
                Some(&run_id),
                format!("Gitlink Pending publication uncertain: {error:#}"),
            );
        }
    };
    let store = match journal::gitlink_store(lease, &run_id, &path_hash, true) {
        Ok(Some(value)) => value,
        other => {
            return uncertain(
                Some(&run_id),
                format!("Gitlink store creation uncertain: {other:?}"),
            );
        }
    };
    let source_git = match LocalGit::new(binary, approved_gitdir, candidate.indexed_oid.len()) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let dest_git = match LocalGit::new(binary, &store, candidate.indexed_oid.len()) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if let Err(error) = dest_git.init() {
        return uncertain(
            Some(&run_id),
            format!("Gitlink byte store init partial: {error:#}"),
        );
    }
    let imported = match import::selected_commit_with_hook(
        &source_git,
        Some(&dest_git),
        &candidate.indexed_oid,
        &pending.commit_tree_oid,
        limits,
        after_object,
    ) {
        Ok(value) => value,
        Err(error) => {
            return uncertain(
                Some(&run_id),
                format!("Gitlink object import partial: {error:#}"),
            );
        }
    };
    let after = package_git_submodule::compare_one(
        package_source,
        &proof.vault,
        &proof.manifest,
        candidate,
        approved_gitdir,
        binary,
        limits,
    );
    let after_row = match pending_row(&proof, &tree_bytes, candidate, &after) {
        Ok(value) if value == pending => value,
        other => {
            return uncertain(
                Some(&run_id),
                format!("source/index/vault changed after object import: {other:?}"),
            );
        }
    };
    let _ = after_row;
    let retained = match package_git_submodule::compare_retained_byte_store(
        &proof.vault,
        &proof.manifest,
        &store,
        &candidate.path,
        &candidate.indexed_oid,
        binary,
        limits,
    ) {
        RawInspection::Exact(value) => value,
        other => {
            return uncertain(
                Some(&run_id),
                format!("retained byte store comparison uncertain: {other:?}"),
            );
        }
    };
    let landed = match landed_row(&pending, &pending_bytes, &store, &imported, &retained) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if let Err(error) = journal::publish(lease, &run_id, &landed_name, &landed) {
        return uncertain(
            Some(&run_id),
            format!("Gitlink Landed publication uncertain: {error:#}"),
        );
    }
    inspect_one_cold(
        lease,
        namespace,
        scope,
        key,
        &candidate.path,
        Some(package_source),
        Some(approved_gitdir),
        binary,
        limits,
    )
}

pub(crate) fn inspect_one_cold(
    lease: &LifecycleLease,
    namespace: &str,
    scope: &SubmissionScope,
    key: &str,
    path: &str,
    package_source: Option<&Project>,
    approved_gitdir: Option<&Project>,
    binary: &Path,
    limits: SubmoduleLimits,
) -> LinkStoreInspection {
    let proof = match prior(lease, namespace, scope, key, package_source, binary) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let run_id = proof.run_id.clone();
    let path_hash = path_sha(path);
    let (pending_name, landed_name) = suffixes(&path_hash);
    let pending_bytes = match journal::read_bytes(lease, &run_id, &pending_name) {
        Ok(Some(value)) => value,
        Ok(None) => {
            let landed = journal::read_bytes(lease, &run_id, &landed_name);
            let store = journal::gitlink_store(lease, &run_id, &path_hash, false);
            return match (landed, store) {
                (Ok(None), Ok(None)) => LinkStoreInspection::NotReady {
                    run_id: Some(run_id),
                },
                _ => uncertain(
                    Some(&run_id),
                    "Gitlink byte store or Landed exists without Pending",
                ),
            };
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let pending: GitlinkPending = match serde_json::from_slice(&pending_bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let tree_bytes = match journal::read_bytes(lease, &run_id, "tree") {
        Ok(Some(value)) => value,
        other => {
            return uncertain(
                Some(&run_id),
                format!("tree receipt unavailable cold: {other:?}"),
            );
        }
    };
    let c1_plan_bytes = match serde_json::to_vec(&proof.plan) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let semantic = match proof.plan.digest() {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if pending.format != journal::GITLINK_PENDING_FORMAT
        || pending.run_id != run_id
        || pending.path != path
        || pending.path_sha256 != path_hash
        || pending.source_root_token != proof.manifest.source_root_token
        || pending.c1_plan_sha256 != sha(&c1_plan_bytes)
        || pending.semantic_plan_sha256 != semantic
        || pending.intent_sha256 != proof.intent_sha256
        || pending.payload_manifest_sha256 != proof.payload_manifest_sha256
        || pending.tree_receipt_sha256 != sha(&tree_bytes)
        || serde_json::to_vec(&pending).ok().as_deref() != Some(pending_bytes.as_slice())
    {
        return uncertain(
            Some(&run_id),
            "Gitlink Pending differs from exact C.1/tree/plan bytes",
        );
    }
    let landed_bytes = match journal::read_bytes(lease, &run_id, &landed_name) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return uncertain(
                Some(&run_id),
                "Gitlink Pending has no Landed; never auto-import again",
            );
        }
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let landed: GitlinkLanded = match serde_json::from_slice(&landed_bytes) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let store = match journal::gitlink_store(lease, &run_id, &path_hash, false) {
        Ok(Some(value)) => value,
        other => {
            return uncertain(
                Some(&run_id),
                format!("retained Gitlink store absent: {other:?}"),
            );
        }
    };
    let retained = match package_git_submodule::compare_retained_byte_store(
        &proof.vault,
        &proof.manifest,
        &store,
        path,
        &pending.indexed_commit_oid,
        binary,
        limits,
    ) {
        RawInspection::Exact(value) => value,
        other => {
            return uncertain(
                Some(&run_id),
                format!("retained Gitlink raw bytes differ: {other:?}"),
            );
        }
    };
    if !matches_raw(&pending, &retained) {
        return uncertain(
            Some(&run_id),
            "retained Gitlink bytes differ from Pending candidate",
        );
    }
    let git = match LocalGit::new(binary, &store, pending.indexed_commit_oid.len()) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    let inventory = match import::selected_commit(
        &git,
        None,
        &pending.indexed_commit_oid,
        &pending.commit_tree_oid,
        limits,
    ) {
        Ok(value) => value,
        Err(error) => {
            return uncertain(
                Some(&run_id),
                format!("retained Gitlink object inventory differs: {error:#}"),
            );
        }
    };
    let expected = match landed_row(&pending, &pending_bytes, &store, &inventory, &retained) {
        Ok(value) => value,
        Err(error) => return uncertain(Some(&run_id), error.to_string()),
    };
    if landed != expected
        || serde_json::to_vec(&landed).ok().as_deref() != Some(landed_bytes.as_slice())
    {
        return uncertain(
            Some(&run_id),
            "Gitlink Landed differs from independently read retained bytes",
        );
    }
    if let (Some(source), Some(gitdir)) = (package_source, approved_gitdir) {
        let candidate = GitlinkCandidate {
            path: pending.path.clone(),
            indexed_oid: pending.indexed_commit_oid.clone(),
            checkout_head_oid: pending.indexed_commit_oid.clone(),
        };
        let observed = package_git_submodule::compare_one(
            source,
            &proof.vault,
            &proof.manifest,
            &candidate,
            gitdir,
            binary,
            limits,
        );
        if pending_row(&proof, &tree_bytes, &candidate, &observed)
            .ok()
            .as_ref()
            != Some(&pending)
        {
            return uncertain(
                Some(&run_id),
                "current package/Gitdir/index differs from historical Gitlink Pending",
            );
        }
    } else if package_source.is_some() || approved_gitdir.is_some() {
        return unavailable(
            Some(&run_id),
            "current Gitlink reobservation needs both held source and approved Git dir",
        );
    }
    match prior(lease, namespace, scope, key, package_source, binary) {
        Ok(again)
            if again.run_id == run_id
                && again.intent_sha256 == proof.intent_sha256
                && again.payload_manifest_sha256 == proof.payload_manifest_sha256
                && again.manifest == proof.manifest => {}
        other => {
            return uncertain(
                Some(&run_id),
                format!("C.1/payload/tree changed during Gitlink cold proof: {other:?}"),
            );
        }
    }
    if !matches!(journal::read_bytes(lease, &run_id, &pending_name), Ok(Some(ref now)) if now == &pending_bytes)
        || !matches!(journal::read_bytes(lease, &run_id, &landed_name), Ok(Some(ref now)) if now == &landed_bytes)
    {
        return uncertain(
            Some(&run_id),
            "Gitlink receipts changed during cold verification",
        );
    }
    LinkStoreInspection::ByteStoreCompleteButNotReady {
        run_id,
        path_sha256: path_hash,
        source_reobserved: package_source.is_some(),
    }
}

#[cfg(test)]
#[path = "package_git_submodule_store/tests.rs"]
mod tests;
