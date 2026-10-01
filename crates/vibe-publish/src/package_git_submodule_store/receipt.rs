//! Canonical per-link Pending/Landed identity from C.1 and raw-byte witnesses.
use super::import;
use crate::package_git_gitlink::GitlinkCandidate;
use crate::package_git_operation::PendingProof;
use crate::package_git_operation::journal::{
    self, GitlinkLanded, GitlinkMetadataRow, GitlinkPending, sha,
};
use crate::package_git_submodule::{MetadataBlob, RawComparison, SubmoduleInspection};
use sha2::{Digest, Sha256};
use vibe_safefs::Project;

const PATH_DOMAIN: &[u8] = b"vibevm/package-git-link-path/e1\0";

pub(super) fn path_sha(path: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(PATH_DOMAIN);
    hash.update((path.len() as u64).to_be_bytes());
    hash.update(path.as_bytes());
    format!("{:x}", hash.finalize())
}
pub(super) fn suffixes(path_hash: &str) -> (String, String) {
    (
        format!("gitlink-{path_hash}-pending"),
        format!("gitlink-{path_hash}"),
    )
}
fn metadata(rows: &[MetadataBlob]) -> Vec<GitlinkMetadataRow> {
    rows.iter()
        .map(|row| GitlinkMetadataRow {
            path: row.path.clone(),
            git_oid: row.git_oid.clone(),
            sha256: row.sha256.clone(),
            size_bytes: row.size_bytes,
        })
        .collect()
}

pub(super) fn pending_row(
    proof: &PendingProof,
    tree_bytes: &[u8],
    candidate: &GitlinkCandidate,
    comparison: &SubmoduleInspection,
) -> anyhow::Result<GitlinkPending> {
    let SubmoduleInspection::ExactCandidate {
        path,
        indexed_commit_oid,
        index_sha256,
        index_gitlink_count,
        index_repository_root_token,
        index_package_prefix,
        commit_tree_oid,
        commit_sha256,
        git_dir_root_token,
        vault_subtree_sha256,
        files,
        directories,
        total_raw_bytes,
        omitted_gitmodules,
    } = comparison
    else {
        anyhow::bail!("not an exact selected Gitlink comparison")
    };
    if path != &candidate.path
        || indexed_commit_oid != &candidate.indexed_oid
        || candidate.checkout_head_oid != candidate.indexed_oid
    {
        anyhow::bail!("Gitlink comparison differs from selected candidate");
    }
    Ok(GitlinkPending {
        format: journal::GITLINK_PENDING_FORMAT.into(),
        run_id: proof.run_id.clone(),
        path: path.clone(),
        path_sha256: path_sha(path),
        indexed_commit_oid: indexed_commit_oid.clone(),
        index_sha256: index_sha256.clone(),
        index_gitlink_count: *index_gitlink_count,
        index_repository_root_token: index_repository_root_token.clone(),
        index_package_prefix: index_package_prefix.clone(),
        source_root_token: proof.manifest.source_root_token.clone(),
        git_dir_root_token: git_dir_root_token.clone(),
        c1_plan_sha256: sha(&serde_json::to_vec(&proof.plan)?),
        semantic_plan_sha256: proof.plan.digest()?,
        intent_sha256: proof.intent_sha256.clone(),
        payload_manifest_sha256: proof.payload_manifest_sha256.clone(),
        tree_receipt_sha256: sha(tree_bytes),
        commit_tree_oid: commit_tree_oid.clone(),
        commit_sha256: commit_sha256.clone(),
        vault_subtree_sha256: vault_subtree_sha256.clone(),
        files: *files,
        directories: *directories,
        total_raw_bytes: *total_raw_bytes,
        omitted_gitmodules: metadata(omitted_gitmodules),
    })
}

pub(super) fn matches_raw(row: &GitlinkPending, raw: &RawComparison) -> bool {
    row.indexed_commit_oid == raw.indexed_commit_oid
        && row.commit_tree_oid == raw.commit_tree_oid
        && row.commit_sha256 == raw.commit_sha256
        && row.vault_subtree_sha256 == raw.vault_subtree_sha256
        && row.files == raw.files
        && row.directories == raw.directories
        && row.total_raw_bytes == raw.total_raw_bytes
        && row.omitted_gitmodules == metadata(&raw.omitted_gitmodules)
}

pub(super) fn landed_row(
    row: &GitlinkPending,
    pending_bytes: &[u8],
    store: &Project,
    imported: &import::ImportSummary,
    raw: &RawComparison,
) -> anyhow::Result<GitlinkLanded> {
    if !matches_raw(row, raw) {
        anyhow::bail!("retained byte store differs from initial exact candidate");
    }
    Ok(GitlinkLanded {
        format: journal::GITLINK_FORMAT.into(),
        pending_sha256: sha(pending_bytes),
        run_id: row.run_id.clone(),
        path_sha256: row.path_sha256.clone(),
        indexed_commit_oid: row.indexed_commit_oid.clone(),
        store_root_token: store.root_correlation_token()?,
        imported_objects: imported.objects,
        imported_total_bytes: imported.total_bytes,
        commit_tree_oid: raw.commit_tree_oid.clone(),
        commit_sha256: raw.commit_sha256.clone(),
        vault_subtree_sha256: raw.vault_subtree_sha256.clone(),
        omitted_gitmodules: metadata(&raw.omitted_gitmodules),
    })
}
