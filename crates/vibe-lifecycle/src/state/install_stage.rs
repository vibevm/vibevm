//! Immutable per-run entry record before a protected prerequisite install.
//!
//! Presence means install MAY have effected. It blocks replay; it does not
//! describe partial package/cache state or grant a second install invocation.

use std::path::Path;

use crate::state::current::LifecycleState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use vibe_safefs::PublishStage;

use super::run_journal::{LifecycleRunJournalObservation, inspect_reserved_lifecycle_run};
use super::submission::{
    JournalAdmission, ReservationDecision, SubmissionNamespace, SubmissionReservations,
    SubmissionScope,
};
use crate::LifecycleLease;
use crate::process::{SelectedManifestLockWitness, recheck_selected_manifest_lock};

const ROOT: [&str; 2] = [".vibe", "lifecycle-outcomes"];
const FORMAT: &str = "vibevm.lifecycle.install-entered/1";
const STAGE_CAP: usize = 16 * 1024;
const REQUEST_PLAN_CAP: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum InstallStageError {
    #[error("invalid install-entered input: {0}")]
    InvalidInput(&'static str),
    #[error("install-entered source refused: {0}")]
    SourceRefused(String),
    #[error("install-entered requires recovery before install: {0}")]
    RecoveryRequired(String),
    #[error("install-entered publication is uncertain; install is forbidden: {0}")]
    Uncertain(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StageRecord {
    format: String,
    run_id: String,
    reservation_ref: String,
    pre_effect_plan_sha256: String,
    stage: String,
    effect_truth: String,
}

/// Evidence that this invocation published one exact entry record. This value
/// is not an effect permit, nor proof of an install result.
#[derive(Debug)]
pub struct InstallEnteredStage {
    pub(crate) run_id: String,
    pub(crate) reservation_ref: String,
    pub(crate) pre_effect_plan_sha256: String,
}

/// Establish an immutable, read-back install entry after C.1 and the lifecycle
/// header are ready. Root-only for now: a member run needs a separate proven
/// relationship between its selected node and the leased workspace root.
#[allow(clippy::too_many_arguments)]
pub fn publish_install_entered_stage(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    canonical_request: &[u8],
    canonical_pre_effect_plan: &[u8],
    selected_root: &Path,
    selected_witness: &SelectedManifestLockWitness,
    workspace_root: &Path,
    workspace_witness: &SelectedManifestLockWitness,
) -> Result<InstallEnteredStage, InstallStageError> {
    if !crate::process::is_valid_run_id(run_id)
        || canonical_request.len() > REQUEST_PLAN_CAP
        || canonical_pre_effect_plan.len() > REQUEST_PLAN_CAP
        || selected_root != workspace_root
        || workspace_root != lease.root()
    {
        return Err(InstallStageError::InvalidInput(
            "run ID, payload bound or root-only binding",
        ));
    }
    if SubmissionReservations::journal_admission(lease, namespace, scope, key, run_id)
        .map_err(|error| InstallStageError::RecoveryRequired(error.to_string()))?
        != JournalAdmission::Ready
    {
        return Err(InstallStageError::RecoveryRequired(
            "reservation journal is not ready".into(),
        ));
    }
    match SubmissionReservations::reserve(
        lease,
        namespace,
        scope,
        key,
        canonical_request,
        canonical_pre_effect_plan,
        run_id,
    ) {
        Ok(ReservationDecision::Existing { domain_run_id }) if domain_run_id == run_id => {}
        Ok(_) => {
            return Err(InstallStageError::RecoveryRequired(
                "reservation is not active for this run".into(),
            ));
        }
        Err(error) => return Err(InstallStageError::RecoveryRequired(error.to_string())),
    }
    let reservation_ref =
        SubmissionReservations::reservation_ref(lease, namespace, scope, key, run_id)
            .map_err(|error| InstallStageError::RecoveryRequired(error.to_string()))?;
    match inspect_reserved_lifecycle_run(lease, run_id) {
        Ok(LifecycleRunJournalObservation::RecoveryRequired { state })
            if matches_root_install_header(&state) => {}
        Ok(_) => {
            return Err(InstallStageError::RecoveryRequired(
                "matching root Install or Generate lifecycle header is absent".into(),
            ));
        }
        Err(error) => return Err(InstallStageError::RecoveryRequired(error.to_string())),
    }
    selected_witness
        .validate()
        .map_err(|error| InstallStageError::SourceRefused(error.to_string()))?;
    workspace_witness
        .validate()
        .map_err(|error| InstallStageError::SourceRefused(error.to_string()))?;
    recheck_selected_manifest_lock(selected_root, selected_witness)
        .map_err(|error| InstallStageError::SourceRefused(error.to_string()))?;
    recheck_selected_manifest_lock(workspace_root, workspace_witness)
        .map_err(|error| InstallStageError::SourceRefused(error.to_string()))?;

    let pre_effect_plan_sha256 = format!("sha256:{:x}", Sha256::digest(canonical_pre_effect_plan));
    let record = StageRecord {
        format: FORMAT.into(),
        run_id: run_id.into(),
        reservation_ref: reservation_ref.clone(),
        pre_effect_plan_sha256: pre_effect_plan_sha256.clone(),
        stage: "install-entered".into(),
        effect_truth: "may-effect-uncertain-after-entry".into(),
    };
    let bytes = serde_json::to_vec(&record).map_err(|error| {
        InstallStageError::InvalidInput(if error.is_data() {
            "record data"
        } else {
            "record encoding"
        })
    })?;
    if bytes.len() > STAGE_CAP {
        return Err(InstallStageError::InvalidInput("stage record size"));
    }
    let home = lease
        .project()
        .ensure_subproject(&ROOT)
        .map_err(|error| InstallStageError::Uncertain(format!("install-stage home: {error:#}")))?;
    let directory = home.root_dir().map_err(|error| {
        InstallStageError::Uncertain(format!("install-stage directory: {error:#}"))
    })?;
    let name = format!("{run_id}.install-entered.json");
    match home.read_file_bounded_in(&directory, &name, STAGE_CAP) {
        Ok(Some(_)) => {
            return Err(InstallStageError::RecoveryRequired(
                "install-entered stage already exists".into(),
            ));
        }
        Ok(None) => {}
        Err(error) => {
            return Err(InstallStageError::Uncertain(format!(
                "stage occupant: {error:#}"
            )));
        }
    }
    home.publish_new_in(&directory, &name, &bytes)
        .map_err(|error| {
            let stage = error.stage;
            let reason = format!("{:#}", error.into_report());
            if stage == PublishStage::PossiblyPublished {
                InstallStageError::Uncertain(reason)
            } else {
                InstallStageError::RecoveryRequired(reason)
            }
        })?;
    let visible = home
        .read_file_snapshot_bounded_in(&directory, &name, STAGE_CAP)
        .map_err(|error| InstallStageError::Uncertain(format!("stage readback: {error:#}")))?
        .ok_or_else(|| InstallStageError::Uncertain("stage absent after publication".into()))?;
    if visible.bytes != bytes {
        return Err(InstallStageError::Uncertain(
            "stage readback bytes differ".into(),
        ));
    }
    Ok(InstallEnteredStage {
        run_id: run_id.into(),
        reservation_ref,
        pre_effect_plan_sha256,
    })
}

fn matches_root_install_header(state: &LifecycleState) -> bool {
    if state.run.selected.as_deref() != Some(".") {
        return false;
    }
    match (state.run.requested.as_str(), state.run.chain.as_slice()) {
        ("install", [validate, install]) => validate == "validate" && install == "install",
        ("generate", [validate, install, generate]) => {
            validate == "validate" && install == "install" && generate == "generate"
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "install_stage/tests.rs"]
mod tests;
