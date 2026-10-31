//! `steps.json` (`vibe-app-steps/1`): the only state of a work directory,
//! rewritten after each step (`PROP-065#execution`, `#resume`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#execution");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::ShellInterpreter;
use vibe_wire::generated::application::e1::steps as wire;

use crate::error::{AppError, StepError};
use crate::index::{AuthoredCommands, PackageCoordinate, StepOutcome};
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
}

/// The whole `steps.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepsFile {
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
    /// Read the kept state; `None` when the work directory has none.
    pub fn load(work: &Path) -> Result<Option<Self>, AppError> {
        let file = path(work);
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(mismatch(work, error.to_string())),
        };
        let document: wire::StepsFile =
            serde_json::from_slice(&bytes).map_err(|error| mismatch(work, error.to_string()))?;
        if document.protocol != STEPS_PROTOCOL {
            return Err(mismatch(
                work,
                format!("protocol `{}` is not `{STEPS_PROTOCOL}`", document.protocol),
            ));
        }
        Ok(Some(Self::from_wire(document)))
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

    pub fn to_wire(&self) -> wire::StepsFile {
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
                })
                .collect(),
        }
    }

    pub fn from_wire(document: wire::StepsFile) -> Self {
        Self {
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
                })
                .collect(),
        }
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
        assert_eq!(state.to_wire(), document);
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
        state.save(dir.path()).unwrap();
        let loaded = StepsFile::load(dir.path()).unwrap().unwrap();
        assert_eq!(loaded, state);
        assert_eq!(loaded.steps.len(), 4, "a re-recorded step keeps its place");
        fs::write(dir.path().join("steps.json"), b"{}").unwrap();
        assert!(StepsFile::load(dir.path()).is_err());
    }
}
