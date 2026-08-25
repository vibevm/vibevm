//! Canonical lifecycle-envelope construction and handler dispatch.
//!
//! The generated JTD types are the runtime types. This module owns only the
//! execution session which fills those types and the closed R2.4 builtin
//! registry which consumes them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW");

use std::collections::BTreeMap;

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::{ExtensionHandler, ExtensionKey};
use vibe_wire::generated::lifecycle::e1::context::{
    Artifact, Context, Execution, Io, Project, Run, RunAgentMode, World,
};
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyStatus};
use vibe_wire::generated::lifecycle_state::StateArtifact;

use crate::ExtensionRegistryRow;

mod builtin;

pub use builtin::BuiltinRegistry;

/// Immutable request facts shared by every envelope in one lifecycle run.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMetadata {
    /// Exact requested lifecycle spelling (`build`, `clean`, ...).
    pub requested: String,
    /// Complete inclusive chain, including a leading clean when composed.
    pub chain: Vec<String>,
    /// Effective offline posture after all configuration layers.
    pub offline: bool,
    /// Whether confirmation is already waived for this invocation.
    pub assume_yes: bool,
    /// Handler-facing agent mode. R2.4 supplies `cli`.
    pub agent_mode: RunAgentMode,
    /// Freshness override. R2.4 supplies false; R2.5 owns the flag.
    pub force: bool,
    /// Machine-safe id used to isolate scratch paths within this run.
    pub run_id: String,
    /// Injected RFC3339 run-start clock value persisted in lifecycle state.
    pub started: String,
}

/// Mutable state shared by contribution invocations in one world epoch.
///
/// Artifacts are retained here so a later handler receives everything
/// attached by successful earlier replies. R2.4's sole builtin attaches none,
/// leaving the canonical list present and empty.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
#[derive(Debug, Clone)]
pub struct ExecutionSession {
    project: Project,
    world: World,
    run: RunMetadata,
    artifacts: Vec<Artifact>,
}

impl ExecutionSession {
    /// Start one execution epoch from already selected project/world facts.
    #[must_use]
    pub const fn new(project: Project, world: World, run: RunMetadata) -> Self {
        Self {
            project,
            world,
            run,
            artifacts: Vec::new(),
        }
    }

    /// Build the exact generated epoch-1 context for one contribution.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
    pub fn envelope_for(
        &self,
        phase: &str,
        row: &ExtensionRegistryRow,
    ) -> Result<Context, DispatchError> {
        let config = effective_config(row)?;
        Ok(Context {
            artifacts: self.artifacts.clone(),
            envelope: 1,
            execution: Execution {
                config,
                id: row.declaration().id.clone(),
                package: row.provider().to_string(),
            },
            io: Io {
                scratch: scratch_path(&self.project.root, &self.run.run_id, row.key()),
            },
            point: row.declaration().point.to_string(),
            project: self.project.clone(),
            run: Run {
                agent_mode: self.run.agent_mode.clone(),
                assume_yes: self.run.assume_yes,
                chain: self.run.chain.clone(),
                force: self.run.force,
                offline: self.run.offline,
                phase: phase.to_string(),
                requested: self.run.requested.clone(),
            },
            world: self.world.clone(),
        })
    }

    /// Dispatch a phase's canonical plan and stop at its first failure.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
    pub fn dispatch_phase(&mut self, phase: &str, rows: &[ExtensionRegistryRow]) -> DispatchBatch {
        let mut outcomes = Vec::with_capacity(rows.len());
        for row in rows {
            match self.dispatch_one(phase, row) {
                Ok(outcome) => outcomes.push(outcome),
                Err(failure) => {
                    return DispatchBatch {
                        outcomes,
                        failure: Some(failure),
                    };
                }
            }
        }
        DispatchBatch {
            outcomes,
            failure: None,
        }
    }

    pub fn dispatch_one(
        &mut self,
        phase: &str,
        row: &ExtensionRegistryRow,
    ) -> Result<ContributionOutcome, DispatchError> {
        let envelope = self.envelope_for(phase, row)?;
        self.dispatch_prepared(row, envelope)
    }

    /// Dispatch the exact generated envelope whose stable fields were already
    /// fingerprinted by the caller.
    pub fn dispatch_prepared(
        &mut self,
        row: &ExtensionRegistryRow,
        envelope: Context,
    ) -> Result<ContributionOutcome, DispatchError> {
        let reply = match &row.declaration().handler {
            ExtensionHandler::Builtin { name } => BuiltinRegistry::dispatch(name, row, &envelope)?,
            handler => {
                return Err(DispatchError::UnsupportedHandler {
                    key: row.key().clone(),
                    kind: handler.kind().to_string(),
                });
            }
        };
        if reply.envelope != 1 {
            return Err(DispatchError::UnsupportedReplyEpoch {
                key: row.key().clone(),
                epoch: reply.envelope,
            });
        }
        if reply.status == ReplyStatus::Fail {
            return Err(DispatchError::FailedReply {
                key: row.key().clone(),
                status: "fail".to_string(),
                message: reply.message,
            });
        }
        for artifact in &reply.artifacts {
            self.artifacts.push(Artifact {
                id: artifact.id.clone(),
                kind: artifact.kind.clone(),
                path: artifact.path.clone(),
                phase: envelope.run.phase.clone(),
            });
        }
        Ok(ContributionOutcome { envelope, reply })
    }

    /// Rehydrate artifacts retained by a fresh execution before downstream
    /// envelopes/fingerprints are built.
    pub fn hydrate_artifacts(&mut self, phase: &str, artifacts: &[StateArtifact]) {
        self.artifacts
            .extend(artifacts.iter().map(|artifact| Artifact {
                id: artifact.id.clone(),
                kind: artifact.kind.clone(),
                path: artifact.path.clone(),
                phase: phase.to_string(),
            }));
    }
}

/// Successful contribution result and the exact envelope that produced it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionOutcome {
    pub envelope: Context,
    pub reply: Reply,
}

/// Prefix outcomes plus an optional first failure.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
#[derive(Debug)]
pub struct DispatchBatch {
    pub outcomes: Vec<ContributionOutcome>,
    pub failure: Option<DispatchError>,
}

/// Actionable failure at the handler boundary.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
#[derive(Debug, Error)]
pub enum DispatchError {
    #[error(
        "extension `{key}` uses handler kind `{kind}`, but R2.4 supports builtin handlers only \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-BUILTIN; \
          fix: use `handler = {{ kind = \"builtin\", name = \"log\" }}` or wait for the handler's implementation)"
    )]
    UnsupportedHandler { key: ExtensionKey, kind: String },

    #[error(
        "extension `{key}` names unknown builtin `{name}`; supported builtins: log \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-BUILTIN; \
          fix: name the closed builtin `log`)"
    )]
    UnknownBuiltin { key: ExtensionKey, name: String },

    #[error(
        "extension `{key}` has invalid builtin `log` config: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-BUILTIN; \
          fix: set `config.message` to a string)"
    )]
    InvalidLogConfig { key: ExtensionKey, reason: String },

    #[error(
        "extension `{key}` config cannot enter the lifecycle envelope: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: use TOML values representable in the epoch-1 JSON envelope)"
    )]
    ConfigEncoding { key: ExtensionKey, reason: String },

    #[error(
        "extension `{key}` returned reply envelope epoch {epoch}, but this build supports epoch 1 \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: return `envelope = 1`)"
    )]
    UnsupportedReplyEpoch { key: ExtensionKey, epoch: u32 },

    #[error(
        "extension `{key}` returned status `{status}`{detail}; the lifecycle stopped at the first failure \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; \
          fix: correct the named handler or its effective config)",
        detail = message.as_deref().map_or(String::new(), |message| format!(": {message}"))
    )]
    FailedReply {
        key: ExtensionKey,
        status: String,
        message: Option<String>,
    },
}

fn effective_config(
    row: &ExtensionRegistryRow,
) -> Result<BTreeMap<String, Option<serde_json::Value>>, DispatchError> {
    row.effective_config()
        .map(|config| {
            config
                .as_table()
                .iter()
                .map(|(key, value)| {
                    serde_json::to_value(value)
                        .map(|value| (key.clone(), Some(value)))
                        .map_err(|error| DispatchError::ConfigEncoding {
                            key: row.key().clone(),
                            reason: error.to_string(),
                        })
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(BTreeMap::new()))
}

fn scratch_path(project_root: &str, run_id: &str, key: &ExtensionKey) -> String {
    let execution = key
        .to_string()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "{}/.vibe/lifecycle/{run_id}/{execution}/",
        project_root.trim_end_matches('/'),
    )
}

#[cfg(test)]
mod tests;
