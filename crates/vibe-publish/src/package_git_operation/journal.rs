//! Run-scoped create-new local records; never the distribution release journal.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::process::is_valid_run_id;
use vibe_lifecycle::{LifecycleLease, SubmissionScope};

use crate::package_git_payload::{PayloadLimits, ProvenanceStatus};

const HOME: [&str; 2] = [".vibe", "package-git-operations"];
const RECORD_CAP: usize = 1024 * 1024; // C.1-scale control record, not a payload-size cap.
pub(super) const INTENT_FORMAT: &str = "vibevm.package-git-intent/e1";
pub(super) const STAGE_FORMAT: &str = "vibevm.package-git-stage/e1";
pub(super) const REQUEST_FORMAT: &str = "vibevm.package-git-request/e1";
pub(crate) const TREE_FORMAT: &str = "vibevm.package-git-tree/e1";
pub(crate) const PROVENANCE_FORMAT: &str = "vibevm.package-git-no-gitlinks/e1";
pub(crate) const PROVENANCE_PENDING_FORMAT: &str = "vibevm.package-git-no-gitlinks-pending/e1";
pub(crate) const OBJECT_FORMAT: &str = "vibevm.package-git-control-objects/e2";
pub(crate) const PARENT_PENDING_FORMAT: &str = "vibevm.package-git-parent-pending/e1";
pub(crate) const PARENT_FORMAT: &str = "vibevm.package-git-parent/e1";
pub(crate) const GITLINK_PENDING_FORMAT: &str = "vibevm.package-git-link-pending/e1";
pub(crate) const GITLINK_FORMAT: &str = "vibevm.package-git-link/e1";
pub(crate) const POPULATED_PENDING_FORMAT: &str = "vibevm.package-git-populated-pending/e1";
pub(crate) const POPULATED_FORMAT: &str = "vibevm.package-git-populated/e1";
pub(crate) const READY_FORMAT: &str = "vibevm.package-git-ready-seal/e1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub format: String,
    pub semantic_plan_sha256: String,
    pub source_root_token: String,
    pub source_tree_sha256: String,
    pub skipped_gitmodules: bool,
    pub lease_root_token: String,
    pub scope_target: String,
    pub limits: PayloadLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub format: String,
    pub run_id: String,
    pub request_json: String,
    pub plan_json: String,
    pub request_sha256: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub source_root_token: String,
    pub source_tree_sha256: String,
    pub skipped_gitmodules: bool,
    pub lease_root_token: String,
    pub scope_context: String,
    pub scope_target: String,
    pub scope_principal: String,
    pub key_sha256: String,
    pub limits: PayloadLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StageReceipt {
    pub format: String,
    pub run_id: String,
    pub intent_sha256: String,
    pub manifest_sha256: String,
    pub tree_sha256: String,
    pub source_root_token: String,
    pub destination_root_token: String,
    pub provenance: ProvenanceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TreeReceipt {
    pub format: String,
    pub run_id: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub object_root_token: String,
    pub root_tree_oid: String,
    pub oid_hex_len: usize,
    pub files: Vec<TreeFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TreeFile {
    pub path: String,
    pub blob_oid: String,
    pub git_mode: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum NoGitlinksEvidence {
    NoGitRepository,
    NoGitlinks {
        repository_root_token: String,
        package_prefix: String,
        index_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProvenanceReceipt {
    pub format: String,
    pub pending_sha256: Option<String>,
    pub run_id: String,
    pub semantic_plan_sha256: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub tree_receipt_sha256: String,
    pub root_tree_oid: String,
    pub source_root_token: String,
    pub flattened_submodules_sha256: String,
    pub evidence: NoGitlinksEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObjectReceipt {
    pub format: String,
    pub run_id: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub tree_receipt_sha256: String,
    pub provenance_receipt_sha256: String,
    pub parent_receipt_sha256: Option<String>,
    pub parent_oid: Option<String>,
    pub object_root_token: String,
    pub root_tree_oid: String,
    pub oid_hex_len: usize,
    pub commit_oid: String,
    pub commit_size_bytes: usize,
    pub commit_sha256: String,
    pub tag_oid: String,
    pub tag_size_bytes: usize,
    pub tag_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParentPending {
    pub format: String,
    pub run_id: String,
    pub semantic_plan_sha256: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub tree_receipt_sha256: String,
    pub provenance_receipt_sha256: String,
    pub object_root_token: String,
    pub local_bare_identity_sha256: String,
    pub local_bare_root_token: String,
    pub private_prefix: String,
    pub observed_refs: crate::package_git_plan::RefSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParentReceipt {
    pub format: String,
    pub pending_sha256: String,
    pub run_id: String,
    pub semantic_plan_sha256: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub tree_receipt_sha256: String,
    pub provenance_receipt_sha256: String,
    pub object_root_token: String,
    pub local_bare_identity_sha256: String,
    pub local_bare_root_token: String,
    pub private_prefix: String,
    pub observed_refs: crate::package_git_plan::RefSnapshot,
    pub parent_oid: String,
    pub parent_tree_oid: String,
    pub closure_sha256: String,
    pub closure_objects: usize,
    pub closure_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GitlinkMetadataRow {
    pub path: String,
    pub git_oid: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GitlinkPending {
    pub format: String,
    pub run_id: String,
    pub path: String,
    pub path_sha256: String,
    pub indexed_commit_oid: String,
    pub index_sha256: String,
    pub index_gitlink_count: usize,
    pub index_repository_root_token: String,
    pub index_package_prefix: String,
    pub source_root_token: String,
    pub git_dir_root_token: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub tree_receipt_sha256: String,
    pub commit_tree_oid: String,
    pub commit_sha256: String,
    pub vault_subtree_sha256: String,
    pub files: usize,
    pub directories: usize,
    pub total_raw_bytes: u64,
    pub omitted_gitmodules: Vec<GitlinkMetadataRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GitlinkLanded {
    pub format: String,
    pub pending_sha256: String,
    pub run_id: String,
    pub path_sha256: String,
    pub indexed_commit_oid: String,
    pub store_root_token: String,
    pub imported_objects: usize,
    pub imported_total_bytes: u64,
    pub commit_tree_oid: String,
    pub commit_sha256: String,
    pub vault_subtree_sha256: String,
    pub omitted_gitmodules: Vec<GitlinkMetadataRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PopulatedIndexLink {
    pub path: String,
    pub indexed_oid: String,
    pub checkout_head_oid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PopulatedLinkReceipt {
    pub path: String,
    pub pending_sha256: String,
    pub landed_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PopulatedPending {
    pub format: String,
    pub run_id: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub tree_receipt_sha256: String,
    pub source_root_token: String,
    pub index_repository_root_token: String,
    pub index_package_prefix: String,
    pub index_sha256: String,
    pub links: Vec<PopulatedIndexLink>,
    pub link_receipts: Vec<PopulatedLinkReceipt>,
    pub flattened_submodules_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PopulatedLanded {
    pub format: String,
    pub run_id: String,
    pub pending_sha256: String,
    pub flattened_submodules_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadySeal {
    pub format: String,
    pub run_id: String,
    pub namespace_id: String,
    pub scope_context: String,
    pub scope_target: String,
    pub scope_principal: String,
    pub key_sha256: String,
    pub request_sha256: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub lease_root_token: String,
    pub source_root_token: String,
    pub vault_root_token: String,
    pub object_root_token: String,
    pub intent_sha256: String,
    pub payload_manifest_sha256: String,
    pub stage_receipt_sha256: String,
    pub tree_receipt_sha256: String,
    pub no_gitlinks_pending_sha256: String,
    pub no_gitlinks_landed_sha256: String,
    pub parent_pending_sha256: Option<String>,
    pub parent_landed_sha256: Option<String>,
    pub object_receipt_sha256: String,
    pub selected_tree_oid: String,
    pub selected_commit_oid: String,
    pub selected_tag_oid: String,
}

pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Narrow duplicate-preparation correlation under C.1 Ready. This does not
/// rehash Git objects or grant an effect; the Ready seal owner does that cold.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ready_duplicate_matches(
    lease: &LifecycleLease,
    run_id: &str,
    namespace_id: &str,
    scope: &SubmissionScope,
    key: &str,
    request_bytes: &[u8],
    plan: &crate::package_git_plan::PackageGitPlan,
    plan_bytes: &[u8],
    lease_root_token: &str,
    source_root_token: &str,
) -> bool {
    let Ok(Some(bytes)) = read_bytes(lease, run_id, "ready") else {
        return false;
    };
    let Ok(seal) = serde_json::from_slice::<ReadySeal>(&bytes) else {
        return false;
    };
    seal.format == READY_FORMAT
        && seal.run_id == run_id
        && seal.namespace_id == namespace_id
        && seal.scope_context == scope.context
        && seal.scope_target == scope.target
        && seal.scope_principal == scope.principal
        && seal.key_sha256 == sha(key.as_bytes())
        && seal.request_sha256 == sha(request_bytes)
        && seal.c1_plan_sha256 == sha(plan_bytes)
        && plan.digest().ok().as_deref() == Some(seal.semantic_plan_sha256.as_str())
        && seal.lease_root_token == lease_root_token
        && seal.source_root_token == source_root_token
        && seal.selected_tree_oid == plan.staged.git_tree_oid
        && seal.selected_commit_oid == plan.staged.tag_target_commit_oid
        && seal.selected_tag_oid == plan.staged.annotated_tag_oid.as_deref().unwrap_or_default()
        && serde_json::to_vec(&seal).ok().as_deref() == Some(bytes.as_slice())
}
fn name(run_id: &str, suffix: &str) -> anyhow::Result<String> {
    if !is_valid_run_id(run_id) {
        anyhow::bail!("invalid package Git run ID");
    }
    Ok(format!("{run_id}.package-git-{suffix}.json"))
}

pub(crate) fn read_bytes(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    let project = lease.package_git_journal_project();
    let Some(dir) = project.dir_if_present(&HOME)? else {
        return Ok(None);
    };
    project.read_file_bounded_in(&dir, &name(run_id, suffix)?, RECORD_CAP)
}

pub(crate) fn read<T: for<'de> Deserialize<'de>>(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
) -> anyhow::Result<Option<T>> {
    read_bytes(lease, run_id, suffix)?
        .map(|bytes| serde_json::from_slice(&bytes).map_err(Into::into))
        .transpose()
}

pub(crate) fn publish<T: Serialize>(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
    value: &T,
) -> anyhow::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > RECORD_CAP {
        anyhow::bail!("package Git owner control record exceeds C.1 cap");
    }
    let project = lease.package_git_journal_project();
    let home = project.ensure_subproject(&HOME)?;
    let root = home.root_dir()?;
    home.publish_new_in(&root, &name(run_id, suffix)?, &bytes)
        .map_err(|error| error.into_report())?;
    let readback = home.read_file_bounded_in(&root, &name(run_id, suffix)?, RECORD_CAP)?;
    if readback.as_deref() != Some(bytes.as_slice()) {
        anyhow::bail!("package Git owner record changed on readback");
    }
    Ok(bytes)
}

pub(super) fn vault(
    lease: &LifecycleLease,
    run_id: &str,
    create: bool,
) -> anyhow::Result<Option<vibe_safefs::Project>> {
    if !is_valid_run_id(run_id) {
        anyhow::bail!("invalid package Git run ID");
    }
    let project = lease.package_git_journal_project();
    let name = format!("{run_id}.payloads");
    let path = [HOME[0], HOME[1], name.as_str()];
    if create {
        Ok(Some(project.ensure_subproject(&path)?))
    } else if project.dir_if_present(&path)?.is_none() {
        Ok(None)
    } else {
        project.subproject(&path).map(Some)
    }
}

pub(crate) fn objects(
    lease: &LifecycleLease,
    run_id: &str,
    create: bool,
) -> anyhow::Result<Option<vibe_safefs::Project>> {
    if !is_valid_run_id(run_id) {
        anyhow::bail!("invalid package Git run ID");
    }
    let project = lease.package_git_journal_project();
    let name = format!("{run_id}.objects");
    let path = [HOME[0], HOME[1], name.as_str()];
    if create {
        Ok(Some(project.ensure_subproject(&path)?))
    } else if project.dir_if_present(&path)?.is_none() {
        Ok(None)
    } else {
        project.subproject(&path).map(Some)
    }
}

pub(crate) fn gitlink_store(
    lease: &LifecycleLease,
    run_id: &str,
    path_sha256: &str,
    create: bool,
) -> anyhow::Result<Option<vibe_safefs::Project>> {
    if !is_valid_run_id(run_id)
        || path_sha256.len() != 64
        || !path_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    {
        anyhow::bail!("invalid package Gitlink run/path identity");
    }
    let project = lease.package_git_journal_project();
    let name = format!("{run_id}.gitlink-{path_sha256}.objects");
    let path = [HOME[0], HOME[1], name.as_str()];
    if create {
        Ok(Some(project.ensure_subproject(&path)?))
    } else if project.dir_if_present(&path)?.is_none() {
        Ok(None)
    } else {
        project.subproject(&path).map(Some)
    }
}
