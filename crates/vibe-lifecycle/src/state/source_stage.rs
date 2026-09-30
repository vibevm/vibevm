//! Immutable, per-execution source stage before a protected script invocation.
//!
//! A stage says the effect MAY have started. Its presence never permits replay.
//! Source observation detects drift among cooperating lease users, not hostile
//! edits in the final interval before a process opens its script path.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use vibe_safefs::{FileIdentity, Project, PublishStage, StableFileSnapshot};
use vibe_wire::generated::lifecycle::e1::context::Context;

use super::run_journal::{LifecycleRunJournalObservation, inspect_reserved_lifecycle_run};
use super::submission::{
    JournalAdmission, ReservationDecision, SubmissionNamespace, SubmissionReservations,
    SubmissionScope,
};
use crate::LifecycleLease;

const ROOT: [&str; 2] = [".vibe", "lifecycle-outcomes"];
const FORMAT: &str = "vibevm.lifecycle.script-source-stage/1";
const SCRIPT_CAP: usize = 8 * 1024 * 1024;
const STAGE_CAP: usize = 16 * 1024;
const FIELD_CAP: usize = 4096;

#[derive(Debug, Error)]
pub enum ScriptSourceStageError {
    #[error("invalid protected script source-stage input: {0}")]
    InvalidInput(&'static str),
    #[error("script source cannot be witnessed before invocation: {0}")]
    SourceRefused(String),
    #[error("script source-stage requires recovery before invocation: {0}")]
    RecoveryRequired(String),
    #[error("script source-stage state is uncertain; invocation is forbidden: {0}")]
    Uncertain(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StageRecord {
    format: String,
    run_id: String,
    reservation_ref: String,
    pre_effect_plan_sha256: String,
    phase: String,
    execution_key: String,
    provider: String,
    script_relative: String,
    script_sha256: String,
    script_size: u64,
    stage: String,
}

/// A one-run, one-execution source observation. The eventual handler must
/// consume a successful immediate recheck before process spawn; this value
/// alone never grants an effect or makes a retry safe.
pub struct ScriptSourceStage {
    run_id: String,
    phase: String,
    execution_key: String,
    provider: String,
    provider_root: PathBuf,
    provider_root_identity: FileIdentity,
    script_relative: String,
    source: StableFileSnapshot,
    begun: AtomicBool,
}

impl ScriptSourceStage {
    pub(crate) fn claim_for_execution<'a>(
        &'a self,
        context: &Context,
        execution_key: &str,
        provider: &str,
        provider_root: &Path,
    ) -> Result<ScriptSourceAttempt<'a>, ScriptSourceStageError> {
        let run_id = Path::new(&context.io.scratch)
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("");
        self.claim(
            run_id,
            &context.run.phase,
            execution_key,
            provider,
            provider_root,
        )
    }

    /// Spend this stage at dispatch entry, including attempts that later fail
    /// before choosing an interpreter or discovering the script path.
    pub(crate) fn begin<'a>(
        &'a self,
        run_id: &str,
        phase: &str,
        execution_key: &str,
        provider: &str,
        provider_root: &Path,
        resolved_script: &Path,
    ) -> Result<ScriptSourceInvocation<'a>, ScriptSourceStageError> {
        self.claim(run_id, phase, execution_key, provider, provider_root)?
            .match_resolved(provider_root, resolved_script)
    }

    fn claim<'a>(
        &'a self,
        run_id: &str,
        phase: &str,
        execution_key: &str,
        provider: &str,
        provider_root: &Path,
    ) -> Result<ScriptSourceAttempt<'a>, ScriptSourceStageError> {
        if self.begun.swap(true, Ordering::AcqRel) {
            return Err(ScriptSourceStageError::RecoveryRequired(
                "script source stage was already consumed".into(),
            ));
        }
        let root = Project::open(provider_root).map_err(|error| {
            ScriptSourceStageError::RecoveryRequired(format!("selected provider root: {error:#}"))
        })?;
        let current_root = root.root_identity().map_err(|error| {
            ScriptSourceStageError::RecoveryRequired(format!(
                "selected provider identity: {error:#}"
            ))
        })?;
        if run_id != self.run_id
            || phase != self.phase
            || execution_key != self.execution_key
            || provider != self.provider
            || current_root != self.provider_root_identity
        {
            return Err(ScriptSourceStageError::RecoveryRequired(
                "selected provider or execution differs from the stage".into(),
            ));
        }
        Ok(ScriptSourceAttempt { stage: self })
    }

    pub(crate) fn recheck_before_spawn(&self) -> Result<(), ScriptSourceStageError> {
        let provider = Project::open(&self.provider_root).map_err(|error| {
            ScriptSourceStageError::RecoveryRequired(format!("provider root: {error:#}"))
        })?;
        let identity = provider.root_identity().map_err(|error| {
            ScriptSourceStageError::RecoveryRequired(format!("provider identity: {error:#}"))
        })?;
        if identity != self.provider_root_identity {
            return Err(ScriptSourceStageError::RecoveryRequired(
                "provider root identity changed".into(),
            ));
        }
        let observed = provider
            .read_file_snapshot_bounded(&self.script_relative, SCRIPT_CAP)
            .map_err(|error| {
                ScriptSourceStageError::RecoveryRequired(format!("script recheck: {error:#}"))
            })?
            .ok_or_else(|| ScriptSourceStageError::RecoveryRequired("script disappeared".into()))?;
        if observed.identity != self.source.identity
            || observed.size != self.source.size
            || observed.sha256 != self.source.sha256
            || observed.bytes != self.source.bytes
        {
            return Err(ScriptSourceStageError::RecoveryRequired(
                "script identity or bytes changed".into(),
            ));
        }
        Ok(())
    }
}

/// A spent attempt; even an interpreter-selection failure drops this without
/// restoring permission to use the durable stage again.
pub(crate) struct ScriptSourceAttempt<'a> {
    stage: &'a ScriptSourceStage,
}

impl<'a> ScriptSourceAttempt<'a> {
    pub(crate) fn match_resolved(
        self,
        provider_root: &Path,
        resolved_script: &Path,
    ) -> Result<ScriptSourceInvocation<'a>, ScriptSourceStageError> {
        let canonical_root = provider_root.canonicalize().map_err(|error| {
            ScriptSourceStageError::RecoveryRequired(format!("canonical provider root: {error}"))
        })?;
        let relative = resolved_script.strip_prefix(&canonical_root).map_err(|_| {
            ScriptSourceStageError::RecoveryRequired(
                "resolved script is outside the staged provider".into(),
            )
        })?;
        if relative != Path::new(&self.stage.script_relative) {
            return Err(ScriptSourceStageError::RecoveryRequired(
                "resolved script differs from the stage".into(),
            ));
        }
        self.stage.recheck_before_spawn()?;
        Ok(ScriptSourceInvocation { stage: self.stage })
    }
}

/// A one-use gate after the pre-scratch check. The last source recheck must
/// happen immediately before the process runner receives its specification.
pub(crate) struct ScriptSourceInvocation<'a> {
    stage: &'a ScriptSourceStage,
}

impl ScriptSourceInvocation<'_> {
    pub(crate) fn before_spawn(self) -> Result<(), ScriptSourceStageError> {
        self.stage.recheck_before_spawn()
    }
}

/// Publish one immutable `invoke-may-effect` record beneath the held lifecycle
/// lease. The exact C.1 request and plan are re-matched against the existing
/// reservation before the stage is created; a Ready flag alone is insufficient.
#[allow(
    clippy::too_many_arguments,
    reason = "every argument is an exact owner identity"
)]
pub fn publish_script_source_stage(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    canonical_request: &[u8],
    canonical_pre_effect_plan: &[u8],
    phase: &str,
    execution_key: &str,
    provider: &str,
    provider_root: &Path,
    script_relative: &str,
) -> Result<ScriptSourceStage, ScriptSourceStageError> {
    if !crate::process::is_valid_run_id(run_id)
        || canonical_request.len() > 1024 * 1024
        || canonical_pre_effect_plan.len() > 1024 * 1024
        || !valid_field(phase)
        || !valid_field(execution_key)
        || !valid_field(provider)
        || !valid_field(script_relative)
        || !provider_root.is_absolute()
    {
        return Err(ScriptSourceStageError::InvalidInput(
            "run, plan, execution or path",
        ));
    }
    let admission = SubmissionReservations::journal_admission(lease, namespace, scope, key, run_id)
        .map_err(|error| ScriptSourceStageError::RecoveryRequired(error.to_string()))?;
    if admission != JournalAdmission::Ready {
        return Err(ScriptSourceStageError::RecoveryRequired(
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
            return Err(ScriptSourceStageError::RecoveryRequired(
                "reservation is no longer active for this run".into(),
            ));
        }
        Err(error) => return Err(ScriptSourceStageError::RecoveryRequired(error.to_string())),
    }
    let reservation_ref =
        SubmissionReservations::reservation_ref(lease, namespace, scope, key, run_id)
            .map_err(|error| ScriptSourceStageError::RecoveryRequired(error.to_string()))?;
    match inspect_reserved_lifecycle_run(lease, run_id) {
        Ok(LifecycleRunJournalObservation::RecoveryRequired { state })
            if state.run.chain.iter().any(|part| part == phase) => {}
        Ok(_) => {
            return Err(ScriptSourceStageError::RecoveryRequired(
                "matching durable lifecycle header is absent, unattributed or omits this phase"
                    .into(),
            ));
        }
        Err(error) => return Err(ScriptSourceStageError::RecoveryRequired(error.to_string())),
    }
    let provider_project = Project::open(provider_root).map_err(|error| {
        ScriptSourceStageError::SourceRefused(format!("provider root: {error:#}"))
    })?;
    let provider_root_identity = provider_project.root_identity().map_err(|error| {
        ScriptSourceStageError::SourceRefused(format!("provider identity: {error:#}"))
    })?;
    let source = provider_project
        .read_file_snapshot_bounded(script_relative, SCRIPT_CAP)
        .map_err(|error| ScriptSourceStageError::SourceRefused(format!("script: {error:#}")))?
        .ok_or_else(|| ScriptSourceStageError::SourceRefused("script is absent".into()))?;
    let record = StageRecord {
        format: FORMAT.into(),
        run_id: run_id.into(),
        reservation_ref,
        pre_effect_plan_sha256: format!("sha256:{:x}", Sha256::digest(canonical_pre_effect_plan)),
        phase: phase.into(),
        execution_key: execution_key.into(),
        provider: provider.into(),
        script_relative: script_relative.into(),
        script_sha256: format!("sha256:{}", source.sha256),
        script_size: source.size,
        stage: "invoke-may-effect".into(),
    };
    let bytes = serde_json::to_vec(&record).map_err(|error| {
        ScriptSourceStageError::InvalidInput(if error.is_data() {
            "record data"
        } else {
            "record encoding"
        })
    })?;
    if bytes.len() > STAGE_CAP {
        return Err(ScriptSourceStageError::InvalidInput("stage record size"));
    }
    let home = lease.project().ensure_subproject(&ROOT).map_err(|error| {
        ScriptSourceStageError::Uncertain(format!("source-stage home: {error:#}"))
    })?;
    let directory = home.root_dir().map_err(|error| {
        ScriptSourceStageError::Uncertain(format!("source-stage directory: {error:#}"))
    })?;
    let name = stage_name(run_id, phase, execution_key);
    match home.read_file_bounded_in(&directory, &name, STAGE_CAP) {
        Ok(Some(_)) => {
            return Err(ScriptSourceStageError::RecoveryRequired(
                "an invocation stage already exists for this execution".into(),
            ));
        }
        Ok(None) => {}
        Err(error) => {
            return Err(ScriptSourceStageError::Uncertain(format!(
                "stage occupant: {error:#}"
            )));
        }
    }
    home.publish_new_in(&directory, &name, &bytes)
        .map_err(|error| {
            let stage = error.stage;
            let reason = format!("{:#}", error.into_report());
            if stage == PublishStage::PossiblyPublished {
                ScriptSourceStageError::Uncertain(reason)
            } else {
                ScriptSourceStageError::RecoveryRequired(reason)
            }
        })?;
    let visible = home
        .read_file_snapshot_bounded_in(&directory, &name, STAGE_CAP)
        .map_err(|error| ScriptSourceStageError::Uncertain(format!("stage readback: {error:#}")))?
        .ok_or_else(|| {
            ScriptSourceStageError::Uncertain("stage absent after publication".into())
        })?;
    if visible.bytes != bytes {
        return Err(ScriptSourceStageError::Uncertain(
            "stage readback bytes differ".into(),
        ));
    }
    Ok(ScriptSourceStage {
        run_id: run_id.into(),
        phase: phase.into(),
        execution_key: execution_key.into(),
        provider: provider.into(),
        provider_root: provider_root.to_path_buf(),
        provider_root_identity,
        script_relative: script_relative.into(),
        source,
        begun: AtomicBool::new(false),
    })
}

fn valid_field(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= FIELD_CAP
        && !value.chars().any(char::is_control)
        && value.trim() == value
}

fn stage_name(run_id: &str, phase: &str, execution_key: &str) -> String {
    let mut hash = Sha256::new();
    for value in [phase, execution_key] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    format!("{run_id}.{:x}.script-source.json", hash.finalize())
}

#[cfg(test)]
#[path = "source_stage/tests.rs"]
mod tests;
