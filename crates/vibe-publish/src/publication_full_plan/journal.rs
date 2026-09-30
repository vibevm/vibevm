//! Bounded create-new records for a distinct full-plan fake publication run.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::LifecycleLease;
use vibe_lifecycle::process::is_valid_run_id;

use super::{Error, Result};

const HOME: [&str; 2] = [".vibe", "publication-operations"];
const CAP: usize = 128 * 1024;

pub(super) const INTENT_FORMAT: &str = "vibevm.publication.full-plan-intent/1";
pub(super) const STEP_FORMAT: &str = "vibevm.publication.full-plan-step/1";
pub(super) const RESULT_FORMAT: &str = "vibevm.publication.full-plan-result/1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub format: String,
    pub run_id: String,
    pub request_sha256: String,
    pub c1_plan_sha256: String,
    pub semantic_plan_sha256: String,
    pub subject_sha256: String,
    pub remote_generation_ref: String,
    pub target: String,
    pub partition_sha256: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Step {
    pub format: String,
    pub run_id: String,
    pub c1_plan_sha256: String,
    pub index: usize,
    pub partition_sha256: String,
    pub expected_generation_ref: String,
    pub resulting_generation_ref: Option<String>,
    pub receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FinalResult {
    pub format: String,
    pub run_id: String,
    pub reservation_ref: String,
    pub c1_plan_sha256: String,
    pub landed_sha256: String,
}

pub(super) fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn member(run_id: &str, suffix: &str) -> Result<String> {
    if !is_valid_run_id(run_id) {
        return Err(Error::Refused("invalid reserved run ID".into()));
    }
    Ok(format!("{run_id}.full-plan-{suffix}.json"))
}

pub(super) fn read_bytes(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
) -> Result<Option<Vec<u8>>> {
    let project = lease.publication_journal_project();
    let Some(dir) = project.dir_if_present(&HOME).map_err(Error::io)? else {
        return Ok(None);
    };
    project
        .read_file_bounded_in(&dir, &member(run_id, suffix)?, CAP)
        .map_err(Error::io)
}

pub(super) fn read<T: for<'de> Deserialize<'de>>(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
) -> Result<Option<T>> {
    read_bytes(lease, run_id, suffix)?
        .map(|bytes| serde_json::from_slice(&bytes).map_err(Error::from))
        .transpose()
}

pub(super) fn publish<T: Serialize>(
    lease: &LifecycleLease,
    run_id: &str,
    suffix: &str,
    value: &T,
) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > CAP {
        return Err(Error::Refused("owner record exceeds cap".into()));
    }
    let project = lease.publication_journal_project();
    let home = project.ensure_subproject(&HOME).map_err(Error::io)?;
    let dir = home.root_dir().map_err(Error::io)?;
    let name = member(run_id, suffix)?;
    home.publish_new_in(&dir, &name, &bytes)
        .map_err(|error| Error::Refused(format!("owner publication: {:#}", error.into_report())))?;
    let observed = home
        .read_file_bounded_in(&dir, &name, CAP)
        .map_err(Error::io)?;
    if observed.as_deref() != Some(bytes.as_slice()) {
        return Err(Error::Refused("owner record changed on readback".into()));
    }
    Ok(bytes)
}

pub(super) fn pending_name(index: usize) -> String {
    format!("{index:03}-pending")
}
pub(super) fn landed_name(index: usize) -> String {
    format!("{index:03}-landed")
}
