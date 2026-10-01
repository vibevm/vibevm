//! Bounded create-new records for a distinct full-plan fake publication run.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_lifecycle::LifecycleLease;
use vibe_lifecycle::process::is_valid_run_id;
use vibe_wire::generated::publication_effect_plan::PublicationEffectPlan;
use vibe_wire::generated::publication_subject::PublicationSubject;

use super::{Error, RemoteObservation, Result};

const HOME: [&str; 2] = [".vibe", "publication-operations"];
const CAP: usize = 128 * 1024;
const INPUT_CAP: usize = 1024 * 1024;

pub(super) const INTENT_FORMAT: &str = "vibevm.publication.full-plan-intent/1";
pub(super) const STEP_FORMAT: &str = "vibevm.publication.full-plan-step/1";
pub(super) const RESULT_FORMAT: &str = "vibevm.publication.full-plan-result/1";
pub(super) const INPUT_FORMAT: &str = "vibevm.publication.full-plan-input/1";

/// Canonical request and selected metadata, without asset payload bytes or an
/// explicit token field. Smoke runner/evidence references are caller-supplied
/// nonblank strings and may themselves contain sensitive information. Protect
/// this capsule as local operation evidence; never publish or echo it whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InputCapsule {
    pub format: String,
    pub run_id: String,
    pub context: String,
    pub target: String,
    pub principal: String,
    pub key_sha256: String,
    pub request_json: String,
    pub subject: PublicationSubject,
    pub plan: PublicationEffectPlan,
    /// Portable subject-derived upload metadata, never a payload witness.
    pub upload_catalogue_sha256: String,
    /// Same-host held-root correlation, not a portable authority or secret.
    pub root_correlation_token: String,
    pub publish: bool,
    pub allow_replace_published: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_ref: Option<String>,
}

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
    pub upload_catalogue_sha256: String,
    pub root_correlation_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_ref: Option<String>,
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
    pub post_observation_sha256: Option<String>,
    pub post_observation: Option<RemoteObservation>,
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

pub(super) fn read_input(lease: &LifecycleLease, run_id: &str) -> Result<Option<InputCapsule>> {
    let project = lease.publication_journal_project();
    let Some(dir) = project.dir_if_present(&HOME).map_err(Error::io)? else {
        return Ok(None);
    };
    let Some(bytes) = project
        .read_file_bounded_in(&dir, &member(run_id, "input")?, INPUT_CAP)
        .map_err(Error::io)?
    else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

pub(super) fn publish_input(
    lease: &LifecycleLease,
    run_id: &str,
    value: &InputCapsule,
) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > INPUT_CAP {
        return Err(Error::Refused(
            "publication input capsule exceeds 1 MiB".into(),
        ));
    }
    let project = lease.publication_journal_project();
    let home = project.ensure_subproject(&HOME).map_err(Error::io)?;
    let dir = home.root_dir().map_err(Error::io)?;
    let name = member(run_id, "input")?;
    home.publish_new_in(&dir, &name, &bytes).map_err(|error| {
        Error::Refused(format!(
            "input capsule publication: {:#}",
            error.into_report()
        ))
    })?;
    let observed = home
        .read_file_bounded_in(&dir, &name, INPUT_CAP)
        .map_err(Error::io)?;
    if observed.as_deref() != Some(bytes.as_slice()) {
        return Err(Error::Refused("input capsule changed before Ready".into()));
    }
    Ok(())
}

pub(super) fn pending_name(index: usize) -> String {
    format!("{index:03}-pending")
}
pub(super) fn landed_name(index: usize) -> String {
    format!("{index:03}-landed")
}
