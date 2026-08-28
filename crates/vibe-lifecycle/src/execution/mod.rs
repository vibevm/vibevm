//! Canonical lifecycle-envelope construction and handler dispatch.
//!
//! The generated JTD types are the runtime types. This module owns only the
//! execution session which fills those types and the closed R2.4 builtin
//! registry which consumes them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW");

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::{ExtensionHandler, ExtensionKey};
use vibe_wire::generated::lifecycle::e1::context::{
    Artifact, Context, Execution, Io, Project, Run, RunAgentMode, World,
};
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyStatus};
use vibe_wire::generated::lifecycle_state::StateArtifact;

use crate::ExtensionRegistryRow;
use crate::agent::PreparedAgent;
use crate::handlers::{HandlerError, HandlerRuntime, HandlerStreams};

mod builtin;
mod descriptor;

pub use builtin::BuiltinRegistry;
pub use descriptor::{HandlerExecution, SlotTarget};

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
    /// Whether this run compiles under the trace observer — the STICKY
    /// activation bit the identity selector computes (`current request
    /// OR the adopted run's persisted bit`, PROP-054 `##OBS-TRACE`,
    /// R3.4). Host observation, not handler input: the generated
    /// envelope's `run` member never carries it, and the handler context
    /// gains no field for it.
    pub trace_compile: bool,
    /// Machine-safe id used to isolate scratch paths within this run.
    pub run_id: String,
    /// Injected RFC3339 run-start clock value persisted in lifecycle state.
    pub started: String,
    /// The canonical workspace-relative identity of the selected node this
    /// run executes from — `"."` for the workspace root, else the member's
    /// authored forward-slashed rel. Host observation, not handler input:
    /// the generated envelope's `run` member never carries it and no
    /// handler sees it. It exists so [`crate::LifecycleStateStore::begin`]
    /// can write the state header's `selected` (the ownership key the
    /// identity selector judges parks by) and the post-clean reload can
    /// prove the identity survived the wipe.
    pub selected: String,
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
        self.envelope_for_execution(phase, &HandlerExecution::from_row(row))
    }

    pub fn envelope_for_execution(
        &self,
        phase: &str,
        execution: &HandlerExecution,
    ) -> Result<Context, DispatchError> {
        let row = execution.row();
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
                scratch: scratch_path(&self.project.root, &self.run.run_id, &execution.key()),
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
            slot_target: execution.slot_target().cloned(),
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
            ExtensionHandler::Builtin { name } => BuiltinRegistry::dispatch(
                name,
                row,
                &envelope,
                &crate::handlers::NoPackageBindingBackend,
            )?,
            handler => {
                return Err(DispatchError::UnsupportedHandler {
                    key: row.key().clone(),
                    kind: handler.kind().to_string(),
                });
            }
        };
        self.accept_reply(
            &row.key().to_string(),
            envelope,
            reply,
            HandlerStreams::default(),
        )
    }

    pub fn dispatch_prepared_with(
        &mut self,
        row: &ExtensionRegistryRow,
        envelope: Context,
        runtime: &HandlerRuntime<'_>,
    ) -> Result<ContributionOutcome, DispatchError> {
        if let ExtensionHandler::Builtin { name } = &row.declaration().handler {
            let reply = BuiltinRegistry::dispatch(name, row, &envelope, runtime.package_binding)?;
            return self.accept_reply(
                &row.key().to_string(),
                envelope,
                reply,
                HandlerStreams::default(),
            );
        }
        let (reply, streams) = runtime.dispatch(row, &envelope).map_err(|error| {
            let streams = error.streams().cloned().unwrap_or_default();
            DispatchError::Handler {
                key: row.key().to_string(),
                error: Box::new(error),
                streams: Box::new(streams),
            }
        })?;
        self.accept_reply(&row.key().to_string(), envelope, reply, streams)
    }

    /// Dispatch one execution with the exact credential-free preparation the
    /// caller already folded into its fingerprint. Passing it — rather than
    /// letting the handler resolve again — is what makes the freshness
    /// decision and the paid call agree on the same bytes.
    pub fn dispatch_execution(
        &mut self,
        execution: &HandlerExecution,
        envelope: Context,
        runtime: &HandlerRuntime<'_>,
        prepared: Option<&PreparedAgent>,
    ) -> Result<ContributionOutcome, DispatchError> {
        if let ExtensionHandler::Builtin { name } = &execution.row().declaration().handler {
            let reply = BuiltinRegistry::dispatch(
                name,
                execution.row(),
                &envelope,
                runtime.package_binding,
            )?;
            return self.accept_reply(&execution.key(), envelope, reply, HandlerStreams::default());
        }
        let (reply, streams) = runtime
            .dispatch_execution(execution, &envelope, prepared)
            .map_err(|error| {
                let streams = error.streams().cloned().unwrap_or_default();
                DispatchError::Handler {
                    key: execution.key(),
                    error: Box::new(error),
                    streams: Box::new(streams),
                }
            })?;
        self.accept_reply(&execution.key(), envelope, reply, streams)
    }

    fn accept_reply(
        &mut self,
        key: &str,
        envelope: Context,
        reply: Reply,
        streams: HandlerStreams,
    ) -> Result<ContributionOutcome, DispatchError> {
        crate::handlers::validate_reply(&reply, &envelope, key).map_err(|error| {
            DispatchError::InvalidReply {
                key: key.to_string(),
                reason: error.to_string(),
                streams: Box::new(streams.clone()),
            }
        })?;
        if reply.envelope != 1 {
            return Err(DispatchError::UnsupportedReplyEpoch {
                key: key.to_string(),
                epoch: reply.envelope,
            });
        }
        if reply.status == ReplyStatus::Fail {
            return Err(DispatchError::FailedReply {
                key: key.to_string(),
                status: "fail".to_string(),
                message: reply.message,
                streams: Box::new(streams),
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
        Ok(ContributionOutcome {
            envelope,
            reply,
            streams,
        })
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

    /// Replace the selected durable world without losing run identity or the
    /// artifact registry accumulated before the install barrier.
    pub fn rebind_world(&mut self, project: Project, world: World) {
        self.project = project;
        self.world = world;
    }
}

/// Successful contribution result and the exact envelope that produced it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionOutcome {
    pub envelope: Context,
    pub reply: Reply,
    pub streams: HandlerStreams,
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
        "package binding `{key}` failed: {reason}; the lifecycle stopped before every later contribution \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; \
          fix: repair the declared skill source or its project-local target and rerun)"
    )]
    PackageBinding { key: String, reason: String },

    #[error(
        "extension `{key}` returned an invalid reply: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE; \
          fix: emit unique, project-contained, existing artifacts)"
    )]
    InvalidReply {
        key: String,
        reason: String,
        streams: Box<HandlerStreams>,
    },

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
    UnsupportedReplyEpoch { key: String, epoch: u32 },

    #[error(
        "extension `{key}` returned status `{status}`{detail}; the lifecycle stopped at the first failure \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; \
          fix: correct the named handler or its effective config)",
        detail = message.as_deref().map_or(String::new(), |message| format!(": {message}"))
    )]
    FailedReply {
        key: String,
        status: String,
        message: Option<String>,
        streams: Box<HandlerStreams>,
    },

    #[error(
        "extension `{key}` handler failed: {error} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; \
          fix: correct the named handler or its process/reply wire)"
    )]
    Handler {
        key: String,
        error: Box<HandlerError>,
        streams: Box<HandlerStreams>,
    },
}

impl DispatchError {
    #[must_use]
    pub fn streams(&self) -> Option<&HandlerStreams> {
        match self {
            Self::FailedReply { streams, .. }
            | Self::InvalidReply { streams, .. }
            | Self::Handler { streams, .. } => Some(streams.as_ref()),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_durable_soft_post(&self) -> bool {
        match self {
            Self::FailedReply { .. } => true,
            Self::Handler { error, .. } => {
                matches!(error.as_ref(), HandlerError::NonZero { .. })
            }
            _ => false,
        }
    }
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

fn scratch_path(project_root: &str, run_id: &str, key: &str) -> String {
    let execution = format!("{:x}", Sha256::digest(key.as_bytes()));
    format!(
        "{}/.vibe/lifecycle/{run_id}/{execution}/",
        project_root.trim_end_matches('/'),
    )
}

#[cfg(test)]
mod tests;
