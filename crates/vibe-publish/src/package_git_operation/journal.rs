//! Run-scoped create-new local records; never the distribution release journal.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::LifecycleLease;
use vibe_lifecycle::process::is_valid_run_id;

use crate::package_git_payload::{PayloadLimits, ProvenanceStatus};

const HOME: [&str; 2] = [".vibe", "package-git-operations"];
const RECORD_CAP: usize = 1024 * 1024; // C.1-scale control record, not a payload-size cap.
pub(super) const INTENT_FORMAT: &str = "vibevm.package-git-intent/e1";
pub(super) const STAGE_FORMAT: &str = "vibevm.package-git-stage/e1";
pub(super) const REQUEST_FORMAT: &str = "vibevm.package-git-request/e1";
pub(crate) const TREE_FORMAT: &str = "vibevm.package-git-tree/e1";
pub(crate) const PROVENANCE_FORMAT: &str = "vibevm.package-git-no-gitlinks/e1";
pub(crate) const PROVENANCE_PENDING_FORMAT: &str = "vibevm.package-git-no-gitlinks-pending/e1";

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

pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
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
