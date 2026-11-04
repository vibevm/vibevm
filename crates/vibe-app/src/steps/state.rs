//! `steps.json` (`vibe-app-steps/1`): the only state of a work directory,
//! rewritten after each step (`PROP-065#execution`, `#resume`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#execution");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::ShellInterpreter;
use vibe_wire::generated::application::e1::steps as wire;

use crate::agents::RunMode;
mod versions;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub(crate) enum StepsDocument {
    Legacy(wire::StepsFile),
    Current(Box<vibe_wire::generated::application::e2::steps::StepsFile>),
}
use crate::error::{AppError, StepError};
use crate::index::{AuthoredCommands, EnvEntry, EnvOp, PackageCoordinate, StepOutcome, ValueKind};
use crate::installer::{ServedBy, ServedRow};
use crate::settings::write_atomic;

/// Where a run starts (`--resume`, `--from-step ID`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#resume")]
pub enum Start {
    /// A fresh work directory.
    #[default]
    Fresh,
    /// Continue from the first step that is not `done`.
    Resume,
    /// Restart at the named step.
    FromStep(String),
}

/// One reached step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepState {
    pub id: String,
    pub kind: String,
    pub outcome: StepOutcome,
    pub reason: Option<String>,
    pub started: Option<String>,
    pub finished: Option<String>,
    pub uninstall: AuthoredCommands,
    /// The agent run of an `agent` step (`PROP-064#record`).
    pub agent: Option<AgentRun>,
    /// The entries an `env` step applied (`PROP-065#env-step`).
    pub env: Vec<EnvEntry>,
    /// Which candidate served each verified input of the step, and the
    /// remap row that chose it (`PROP-066#provenance`).
    pub served: Vec<ServedBy>,
}

/// How an agent step's agent ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentStatus {
    Ok,
    Fail,
    /// Handed to the hosting agent (`PROP-064#parking`).
    Parked,
}

/// The agent, mode, envelope and answer of one agent step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentRun {
    pub agent: String,
    pub mode: RunMode,
    /// The step's run directory holding the envelope.
    pub envelope: String,
    pub duration_ms: Option<u32>,
    pub status: Option<AgentStatus>,
    pub message: Option<String>,
}

/// The whole `steps.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepsFile {
    pub operation: Option<vibe_wire::generated::shared::ApplicationPendingOperation>,
    pub package: PackageCoordinate,
    pub option: String,
    pub features: Vec<String>,
    pub steps: Vec<StepState>,
}

pub(crate) const STEPS_PROTOCOL: &str = "vibe-app-steps/1";

fn path(work: &Path) -> PathBuf {
    work.join("steps.json")
}

impl StepsFile {
    pub(crate) fn pending(
        &self,
        settings: &crate::SettingsRoot,
    ) -> Result<&vibe_wire::generated::shared::ApplicationPendingOperation, AppError> {
        self.operation.as_ref().ok_or_else(|| {
            crate::operations::pending::failure(
                settings,
                &self.package,
                "expected retained operation metadata is absent".into(),
            )
        })
    }
    pub(crate) fn pending_mut(
        &mut self,
        settings: &crate::SettingsRoot,
    ) -> Result<&mut vibe_wire::generated::shared::ApplicationPendingOperation, AppError> {
        self.operation.as_mut().ok_or_else(|| {
            crate::operations::pending::failure(
                settings,
                &self.package,
                "expected retained operation metadata is absent".into(),
            )
        })
    }
    /// Read the kept state; `None` when the work directory has none.
    pub fn load(work: &Path) -> Result<Option<Self>, AppError> {
        let file = path(work);
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(mismatch(work, error.to_string())),
        };
        #[derive(serde::Deserialize)]
        struct Header {
            protocol: String,
        }
        let header: Header =
            serde_json::from_slice(&bytes).map_err(|e| mismatch(work, e.to_string()))?;
        match header.protocol.as_str() {
            STEPS_PROTOCOL => {
                let document: wire::StepsFile =
                    serde_json::from_slice(&bytes).map_err(|e| mismatch(work, e.to_string()))?;
                Ok(Some(Self::from_wire(document)))
            }
            "vibe-app-steps/2" => {
                let document = serde_json::from_slice::<
                    vibe_wire::generated::application::e2::steps::StepsFile,
                >(&bytes)
                .map_err(|e| mismatch(work, e.to_string()))?;
                let (rows, operation) = versions::split(document);
                if !crate::operations::pending::valid_operation(&operation.target.operation) {
                    return Err(mismatch(
                        work,
                        "invalid pending operation generation token".into(),
                    ));
                }
                let mut retired = std::collections::BTreeSet::new();
                if operation.kind
                    != vibe_wire::generated::shared::ApplicationPendingOperationKind::Update
                    && !operation.retirements.is_empty()
                    || operation.phase
                        == vibe_wire::generated::shared::ApplicationPendingOperationPhase::Retiring
                        && (operation.kind
                            != vibe_wire::generated::shared::ApplicationPendingOperationKind::Update
                            || operation.retirements.is_empty())
                    || operation.retirements.iter().any(|r| {
                        r.key == operation.target.key
                            || r.expected_record.is_empty()
                            || !retired.insert(&r.key)
                    })
                {
                    return Err(mismatch(
                        work,
                        "invalid bound retirement list for this operation".into(),
                    ));
                }
                let mut state = Self::from_wire(rows);
                if operation.target.key != state.package.key()
                    || operation.request.installer != state.option
                {
                    return Err(mismatch(
                        work,
                        "pending operation differs from its step owner".into(),
                    ));
                }
                state.operation = Some(operation);
                Ok(Some(state))
            }
            _ => Err(mismatch(work, "unsupported steps protocol".into())),
        }
    }

    /// Replace `steps.json` atomically.
    pub fn save(&self, work: &Path) -> Result<(), AppError> {
        let mut bytes = serde_json::to_vec_pretty(&self.to_wire())
            .map_err(|error| mismatch(work, error.to_string()))?;
        bytes.push(b'\n');
        Ok(write_atomic(&path(work), &bytes)?)
    }

    pub fn get(&self, id: &str) -> Option<&StepState> {
        self.steps.iter().find(|step| step.id == id)
    }

    /// Record a reached step, replacing an earlier entry of the same id.
    pub fn record(&mut self, state: StepState) {
        match self.steps.iter_mut().find(|step| step.id == state.id) {
            Some(slot) => *slot = state,
            None => self.steps.push(state),
        }
    }

    pub fn to_wire(&self) -> StepsDocument {
        let rows = self.legacy_rows();
        match &self.operation {
            Some(operation) => {
                StepsDocument::Current(Box::new(versions::promote(rows, operation.clone())))
            }
            None => StepsDocument::Legacy(rows),
        }
    }

    fn legacy_rows(&self) -> wire::StepsFile {
        wire::StepsFile {
            protocol: STEPS_PROTOCOL.into(),
            package: wire::Package {
                group: self.package.group.clone(),
                name: self.package.name.clone(),
                version: self.package.version.clone(),
            },
            option: self.option.clone(),
            features: self.features.clone(),
            steps: self
                .steps
                .iter()
                .map(|step| wire::Step {
                    id: step.id.clone(),
                    kind: step.kind.clone(),
                    outcome: match step.outcome {
                        StepOutcome::Done => wire::Outcome::Done,
                        StepOutcome::Skipped => wire::Outcome::Skipped,
                        StepOutcome::Failed => wire::Outcome::Failed,
                        StepOutcome::Delegated => wire::Outcome::Delegated,
                        StepOutcome::Interrupted => wire::Outcome::Interrupted,
                    },
                    reason: step.reason.clone(),
                    started: step.started.clone(),
                    finished: step.finished.clone(),
                    uninstall: step.uninstall.uninstall.clone(),
                    force_uninstall: step.uninstall.force_uninstall.clone(),
                    interpreter: step.uninstall.interpreter.map(|value| match value {
                        ShellInterpreter::PosixSh => wire::Interpreter::PosixSh,
                        ShellInterpreter::WindowsCmd => wire::Interpreter::WindowsCmd,
                        ShellInterpreter::WindowsPowershell => wire::Interpreter::WindowsPowershell,
                    }),
                    agent: step.agent.as_ref().map(|run| wire::AgentRun {
                        agent: run.agent.clone(),
                        mode: match run.mode {
                            RunMode::Interactive => wire::AgentMode::Interactive,
                            RunMode::Unattended => wire::AgentMode::Unattended,
                        },
                        envelope: run.envelope.clone(),
                        duration_ms: run.duration_ms,
                        status: run.status.map(|status| match status {
                            AgentStatus::Ok => wire::AgentStatus::Ok,
                            AgentStatus::Fail => wire::AgentStatus::Fail,
                            AgentStatus::Parked => wire::AgentStatus::Parked,
                        }),
                        message: run.message.clone(),
                    }),
                    env: step.env.iter().map(env_to_wire).collect(),
                    inputs: step.served.iter().map(served_to_wire).collect(),
                })
                .collect(),
        }
    }

    pub fn from_wire(document: wire::StepsFile) -> Self {
        Self {
            operation: None,
            package: PackageCoordinate {
                group: document.package.group,
                name: document.package.name,
                version: document.package.version,
            },
            option: document.option,
            features: document.features,
            steps: document
                .steps
                .into_iter()
                .map(|step| StepState {
                    id: step.id,
                    kind: step.kind,
                    outcome: match step.outcome {
                        wire::Outcome::Done => StepOutcome::Done,
                        wire::Outcome::Skipped => StepOutcome::Skipped,
                        wire::Outcome::Failed => StepOutcome::Failed,
                        wire::Outcome::Delegated => StepOutcome::Delegated,
                        wire::Outcome::Interrupted => StepOutcome::Interrupted,
                    },
                    reason: step.reason,
                    started: step.started,
                    finished: step.finished,
                    uninstall: AuthoredCommands {
                        uninstall: step.uninstall,
                        force_uninstall: step.force_uninstall,
                        update: None,
                        interpreter: step.interpreter.map(|value| match value {
                            wire::Interpreter::PosixSh => ShellInterpreter::PosixSh,
                            wire::Interpreter::WindowsCmd => ShellInterpreter::WindowsCmd,
                            wire::Interpreter::WindowsPowershell => {
                                ShellInterpreter::WindowsPowershell
                            }
                        }),
                    },
                    agent: step.agent.map(|run| AgentRun {
                        agent: run.agent,
                        mode: match run.mode {
                            wire::AgentMode::Interactive => RunMode::Interactive,
                            wire::AgentMode::Unattended => RunMode::Unattended,
                        },
                        envelope: run.envelope,
                        duration_ms: run.duration_ms,
                        status: run.status.map(|status| match status {
                            wire::AgentStatus::Ok => AgentStatus::Ok,
                            wire::AgentStatus::Fail => AgentStatus::Fail,
                            wire::AgentStatus::Parked => AgentStatus::Parked,
                        }),
                        message: run.message,
                    }),
                    env: step.env.into_iter().map(env_from_wire).collect(),
                    served: step
                        .inputs
                        .into_iter()
                        .filter_map(served_from_wire)
                        .collect(),
                })
                .collect(),
        }
    }
}

/// The `PROP-066#layers` label of a layer, both ways.
fn layer_to_wire(label: &str) -> wire::RemapRowLayer {
    match label {
        "invocation" => wire::RemapRowLayer::Invocation,
        "project-package" => wire::RemapRowLayer::ProjectPackage,
        "user-package" => wire::RemapRowLayer::UserPackage,
        _ => wire::RemapRowLayer::Local,
    }
}

fn layer_from_wire(layer: &wire::RemapRowLayer) -> &'static str {
    match layer {
        wire::RemapRowLayer::Invocation => "invocation",
        wire::RemapRowLayer::ProjectPackage => "project-package",
        wire::RemapRowLayer::UserPackage => "user-package",
        wire::RemapRowLayer::Local => "local",
    }
}

fn served_to_wire(served: &ServedBy) -> wire::StepInput {
    wire::StepInput {
        name: served.input.clone(),
        served_by: Some(wire::ServedBy {
            target: served.target.clone(),
            row: served.row.as_ref().map(|row| wire::RemapRow {
                layer: layer_to_wire(&row.layer),
                source: row.source.clone(),
                index: u32::try_from(row.index).unwrap_or(u32::MAX),
                reason: row.reason.clone(),
            }),
        }),
    }
}

/// An input entry without a `servedBy` records nothing about provenance,
/// so it does not come back as a served record.
fn served_from_wire(input: wire::StepInput) -> Option<ServedBy> {
    let served = input.served_by?;
    Some(ServedBy {
        input: input.name,
        target: served.target,
        row: served.row.map(|row| ServedRow {
            layer: layer_from_wire(&row.layer).to_owned(),
            source: row.source,
            index: row.index as usize,
            reason: row.reason,
        }),
    })
}

fn env_to_wire(entry: &EnvEntry) -> wire::EnvEntry {
    wire::EnvEntry {
        step: entry.step.clone(),
        op: match entry.op {
            EnvOp::Set => wire::EnvOp::Set,
            EnvOp::Unset => wire::EnvOp::Unset,
            EnvOp::PathPrepend => wire::EnvOp::PathPrepend,
        },
        name: entry.name.clone(),
        value: entry.value.clone(),
        previous: entry.previous.clone(),
        previous_kind: entry.previous_kind.map(|kind| match kind {
            ValueKind::String => wire::ValueKind::String,
            ValueKind::ExpandString => wire::ValueKind::ExpandString,
        }),
    }
}

fn env_from_wire(entry: wire::EnvEntry) -> EnvEntry {
    EnvEntry {
        step: entry.step,
        op: match entry.op {
            wire::EnvOp::Set => EnvOp::Set,
            wire::EnvOp::Unset => EnvOp::Unset,
            wire::EnvOp::PathPrepend => EnvOp::PathPrepend,
        },
        name: entry.name,
        value: entry.value,
        previous: entry.previous,
        previous_kind: entry.previous_kind.map(|kind| match kind {
            wire::ValueKind::String => ValueKind::String,
            wire::ValueKind::ExpandString => ValueKind::ExpandString,
        }),
    }
}

fn mismatch(work: &Path, reason: String) -> AppError {
    StepError::ResumeMismatch {
        work: work.display().to_string(),
        reason: format!("steps.json is unreadable: {reason}"),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_corpus_round_trips_through_the_domain_and_disk() {
        let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/application/e1/steps.json");
        let document: wire::StepsFile = serde_json::from_slice(&fs::read(corpus).unwrap()).unwrap();
        let state = StepsFile::from_wire(document.clone());
        assert_eq!(state.to_wire(), StepsDocument::Legacy(document));
        assert_eq!(state.get("fix").unwrap().outcome, StepOutcome::Skipped);
        assert_eq!(
            state.get("register").unwrap().uninstall.interpreter,
            Some(ShellInterpreter::PosixSh)
        );

        let dir = tempfile::tempdir().unwrap();
        assert_eq!(StepsFile::load(dir.path()).unwrap(), None);
        let mut state = state;
        state.record(StepState {
            outcome: StepOutcome::Done,
            reason: None,
            ..state.get("build").unwrap().clone()
        });
        state.record(StepState {
            outcome: StepOutcome::Delegated,
            agent: Some(AgentRun {
                agent: "claude-code".into(),
                mode: RunMode::Unattended,
                envelope: "/w/steps/fix".into(),
                duration_ms: None,
                status: Some(AgentStatus::Parked),
                message: None,
            }),
            ..state.get("fix").unwrap().clone()
        });
        state.save(dir.path()).unwrap();
        let loaded = StepsFile::load(dir.path()).unwrap().unwrap();
        assert_eq!(loaded, state);
        assert_eq!(loaded.steps.len(), 4, "a re-recorded step keeps its place");
        fs::write(dir.path().join("steps.json"), b"{}").unwrap();
        assert!(StepsFile::load(dir.path()).is_err());
    }
}
