//! Script and binary handler wires over one injectable process seam.

use std::path::{Path, PathBuf};

use specmark::spec;
use thiserror::Error;
use vibe_core::lifecycle::{ExtensionPoint, SlotPoint};
use vibe_core::manifest::ExtensionHandler;
use vibe_wire::generated::lifecycle::e1::context::Context;
use vibe_wire::generated::lifecycle::e1::reply::{Reply, ReplyStatus};
use vibe_workspace::hooks::{InterpreterProbe, Platform, select_invocation};

use crate::agent::{AgentBackend, AgentError, PreparedAgent};
use crate::process::{
    ProcessOutput, ProcessRunner, ProcessSpec, ScratchError, StreamMode, allocate_pending_reply,
    execution_scratch, minimal_environment, write_atomic_json,
};
use crate::{ExtensionProvider, ExtensionRegistryRow, HandlerExecution, SlotTarget};

const REPLY_CAP: usize = 1024 * 1024;

mod backends;
mod native;
mod reply;
pub use backends::{
    BinaryBackend, NativeBackend, NativeBackendRequest, NoBinaryBackend, NoNativeBackend,
    NoPackageBindingBackend, PackageBindingArtifact, PackageBindingBackend, PackageBindingOutcome,
};
use native::{lifecycle_reply, native_context};
use reply::parse_reply;
pub(crate) use reply::validate_reply;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct HandlerStreams {
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug, Clone, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub enum HandlerError {
    #[error(
        "extension `{key}` has unsafe handler path `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT; \
          fix: use a canonical provider-contained path without links/reparse points)"
    )]
    UnsafePath {
        key: String,
        path: String,
        reason: String,
    },
    #[error(
        "extension `{key}` cannot prepare scratch/context: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: restore a writable, link-free project .vibe directory and rerun)"
    )]
    Scratch { key: String, reason: String },
    #[error(
        "extension `{key}` cannot select a usable script for `{base}` \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT; \
          fix: ship a supported PROP-020 script variant or install its interpreter)"
    )]
    NoInterpreter { key: String, base: String },
    #[error(
        "extension `{key}` process failed: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT; \
          fix: make the selected executable spawnable in its declared provider)"
    )]
    Process { key: String, reason: String },
    #[error(
        "extension `{key}` exited nonzero ({code:?}); reply is ignored{stderr} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; \
          fix: correct the handler and rerun the stopped lifecycle)"
    )]
    NonZero {
        key: String,
        code: Option<i32>,
        stderr: String,
        streams: Box<HandlerStreams>,
    },
    #[error(
        "extension `{key}` emitted invalid reply: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE; \
          fix: emit exactly one strict epoch-1 Reply with valid artifacts and no tasks)"
    )]
    Reply {
        key: String,
        reason: String,
        streams: Option<Box<HandlerStreams>>,
    },
    #[error(
        "extension `{key}` binary resolution/build failed: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#H-BINARY; \
          fix: declare and build the named binary inside the exact provider slot)"
    )]
    Binary { key: String, reason: String },
    #[error(
        "extension `{key}` native execution failed: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#REF-WIRE-NATIVE; \
          fix: run `vibe build`, restore the admitted artifact, or correct the native extension)"
    )]
    Native { key: String, reason: String },
    #[error(
        "extension `{key}` agent execution failed: {error} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
          fix: apply the inner failure's remediation and rerun the stopped lifecycle)"
    )]
    Agent {
        key: String,
        #[source]
        error: Box<AgentError>,
    },
    #[error(
        "{error} (post-spawn streams retained; governed by \
         spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE; \
         fix: correct the typed inner handler failure and rerun)"
    )]
    Observed {
        error: Box<HandlerError>,
        streams: Box<HandlerStreams>,
    },
}

impl HandlerError {
    #[must_use]
    pub fn streams(&self) -> Option<&HandlerStreams> {
        match self {
            Self::NonZero { streams, .. } => Some(streams.as_ref()),
            Self::Reply {
                streams: Some(streams),
                ..
            } => Some(streams.as_ref()),
            Self::Observed { streams, .. } => Some(streams.as_ref()),
            _ => None,
        }
    }

    fn with_streams(self, streams: HandlerStreams) -> Self {
        match self {
            Self::Reply { key, reason, .. } => Self::Reply {
                key,
                reason,
                streams: Some(Box::new(streams)),
            },
            other => other,
        }
    }

    fn observed(self, streams: HandlerStreams) -> Self {
        if self.streams().is_some() {
            self
        } else {
            Self::Observed {
                error: Box::new(self),
                streams: Box::new(streams),
            }
        }
    }
}

#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub struct HandlerRuntime<'a> {
    pub process: &'a dyn ProcessRunner,
    pub binary: &'a dyn BinaryBackend,
    /// Native ARTIFACT resolution plus ABI invocation. Production injects the
    /// one process-lifetime loader; tests may inject a fake.
    pub native: &'a dyn NativeBackend,
    pub package_binding: &'a dyn PackageBindingBackend,
    /// The transport-neutral owner of every paid agent execution. It is
    /// consulted only from the `agent` handler branch, so a project with no
    /// selected agent contribution never reaches a provider or a credential.
    pub agent: &'a dyn AgentBackend,
    pub probe: &'a dyn InterpreterProbe,
    pub streams: StreamMode,
}

impl HandlerRuntime<'_> {
    /// The state-blind entry: an agent row has no fingerprint-bound
    /// preparation here, so it is prepared on the spot. Only the untracked
    /// paths reach this; the freshness-aware runner always passes the exact
    /// bytes it already hashed.
    pub fn dispatch(
        &self,
        row: &ExtensionRegistryRow,
        context: &Context,
    ) -> Result<(Reply, HandlerStreams), HandlerError> {
        self.dispatch_inner(row, None, &row.key().to_string(), context, None)
    }

    pub fn dispatch_execution(
        &self,
        execution: &HandlerExecution,
        context: &Context,
        prepared: Option<&PreparedAgent>,
    ) -> Result<(Reply, HandlerStreams), HandlerError> {
        self.dispatch_inner(
            execution.row(),
            execution.slot_target(),
            &execution.key(),
            context,
            prepared,
        )
    }

    fn dispatch_inner(
        &self,
        row: &ExtensionRegistryRow,
        target: Option<&SlotTarget>,
        execution_key: &str,
        context: &Context,
        prepared: Option<&PreparedAgent>,
    ) -> Result<(Reply, HandlerStreams), HandlerError> {
        match &row.declaration().handler {
            ExtensionHandler::Script { base } => {
                self.script(row, target, execution_key, context, base)
            }
            ExtensionHandler::Binary { name } => {
                self.binary(row, target, execution_key, context, name)
            }
            ExtensionHandler::Native { .. } => {
                if !matches!(
                    row.declaration().point,
                    ExtensionPoint::Phase(_) | ExtensionPoint::Slot(_)
                ) {
                    return Err(HandlerError::Native {
                        key: execution_key.to_string(),
                        reason: format!(
                            "point `{}` is compiler-native and cannot enter lifecycle dispatch in R5.3",
                            row.declaration().point
                        ),
                    });
                }
                let native_context = native_context(context);
                let reply = self
                    .native
                    .invoke(NativeBackendRequest {
                        row,
                        extension_id: &row.declaration().id,
                        point: row.declaration().point,
                        ir_schema: None,
                        context: &native_context,
                    })
                    .map_err(|reason| HandlerError::Native {
                        key: execution_key.to_string(),
                        reason,
                    })?;
                Ok((lifecycle_reply(reply), HandlerStreams::default()))
            }
            ExtensionHandler::Agent { .. } => {
                let agent_error = |error: AgentError| HandlerError::Agent {
                    key: execution_key.to_string(),
                    error: Box::new(error),
                };
                let owned;
                let prepared = match prepared {
                    Some(prepared) => prepared,
                    None => {
                        owned = crate::agent::prepare(self.agent, row, context)
                            .map_err(agent_error)?
                            .ok_or_else(|| HandlerError::Process {
                                key: execution_key.to_string(),
                                reason: "an agent row lost its handler kind".into(),
                            })?;
                        &owned
                    }
                };
                crate::agent::execute(self.agent, execution_key, context, prepared)
                    .map(|reply| (reply, HandlerStreams::default()))
                    .map_err(agent_error)
            }
            other => Err(HandlerError::Process {
                key: row.key().to_string(),
                reason: format!("handler kind `{}` is not process-backed", other.kind()),
            }),
        }
    }

    fn script(
        &self,
        row: &ExtensionRegistryRow,
        target: Option<&SlotTarget>,
        execution_key: &str,
        context: &Context,
        base: &Path,
    ) -> Result<(Reply, HandlerStreams), HandlerError> {
        let key = execution_key.to_string();
        let provider = provider_root(row);
        let invocation = select_invocation(provider, base, Platform::current(), self.probe)
            .ok_or_else(|| HandlerError::NoInterpreter {
                key: key.clone(),
                base: base.display().to_string(),
            })?;
        let script = contained_existing(provider, &invocation.script, &key)?;
        let scratch = verified_scratch(context, &key)?;
        let context_path = write_atomic_json(&scratch, "context.json", context)
            .map_err(|error| scratch_error(&key, error))?;
        let mut reply_pending =
            allocate_pending_reply(&scratch).map_err(|error| scratch_error(&key, error))?;
        let cwd = target
            .map(|target| PathBuf::from(&target.root))
            .unwrap_or_else(|| PathBuf::from(&context.project.root));
        let env = script_env(row, target, context, &context_path, reply_pending.path());
        reply_pending.publish();
        let (program, args) = if invocation.interpreter == "powershell" {
            (
                PathBuf::from("powershell"),
                vec![
                    "-NoProfile".into(),
                    "-ExecutionPolicy".into(),
                    "Bypass".into(),
                    "-File".into(),
                    script.into_os_string(),
                ],
            )
        } else {
            (
                PathBuf::from(invocation.interpreter),
                vec![script.into_os_string()],
            )
        };
        let output = self
            .process
            .run(&ProcessSpec {
                program,
                args,
                cwd,
                env,
                stdin: None,
                stdout: self.streams,
                stderr: self.streams,
                scratch: scratch.clone(),
            })
            .map_err(|error| HandlerError::Process {
                key: key.clone(),
                reason: error.to_string(),
            })?;
        if output.code != Some(0) {
            let code = output.code;
            let streams = streams(output);
            return Err(HandlerError::NonZero {
                key,
                code,
                stderr: streams.stderr.clone(),
                streams: Box::new(streams),
            });
        }
        let streams = streams(output);
        let bytes = reply_pending
            .read_capped(REPLY_CAP)
            .map_err(|error| scratch_error(&key, error).observed(streams.clone()))?;
        let reply = if !bytes.is_empty() {
            parse_reply(&bytes, &key).map_err(|error| error.with_streams(streams.clone()))?
        } else {
            Reply {
                artifacts: Vec::new(),
                envelope: 1,
                status: ReplyStatus::Ok,
                tasks: Vec::new(),
                message: None,
            }
        };
        validate_reply(&reply, context, &key)
            .map_err(|error| error.with_streams(streams.clone()))?;
        write_atomic_json(&scratch, "reply.json", &reply)
            .map_err(|error| scratch_error(&key, error).observed(streams.clone()))?;
        reply_pending
            .consume()
            .map_err(|error| scratch_error(&key, error).observed(streams.clone()))?;
        Ok((reply, streams))
    }

    fn binary(
        &self,
        row: &ExtensionRegistryRow,
        target: Option<&SlotTarget>,
        execution_key: &str,
        context: &Context,
        name: &str,
    ) -> Result<(Reply, HandlerStreams), HandlerError> {
        let key = execution_key.to_string();
        let artifact =
            self.binary
                .resolve_or_build(row, name)
                .map_err(|reason| HandlerError::Binary {
                    key: key.clone(),
                    reason,
                })?;
        let artifact = contained_existing(provider_root(row), &artifact, &key)?;
        let scratch = verified_scratch(context, &key)?;
        write_atomic_json(&scratch, "context.json", context)
            .map_err(|error| scratch_error(&key, error))?;
        let stdin = serde_json::to_vec(context).map_err(|error| HandlerError::Reply {
            key: key.clone(),
            reason: error.to_string(),
            streams: None,
        })?;
        let output = self
            .process
            .run(&ProcessSpec {
                program: artifact,
                args: Vec::new(),
                cwd: target
                    .map(|target| PathBuf::from(&target.root))
                    .unwrap_or_else(|| PathBuf::from(&context.project.root)),
                env: minimal_environment(Vec::<(String, String)>::new()),
                stdin: Some(stdin),
                stdout: StreamMode::Capture,
                stderr: self.streams,
                scratch,
            })
            .map_err(|error| HandlerError::Process {
                key: key.clone(),
                reason: error.to_string(),
            })?;
        if output.code != Some(0) {
            let code = output.code;
            let stdout_truncated = output.stdout_truncated;
            let mut streams = streams(output);
            streams.stdout.clear();
            streams.stdout_truncated = stdout_truncated;
            return Err(HandlerError::NonZero {
                key,
                code,
                stderr: streams.stderr.clone(),
                streams: Box::new(streams),
            });
        }
        if output.stdout_truncated {
            let mut streams = streams(output);
            streams.stdout.clear();
            return Err(HandlerError::Reply {
                key,
                reason: "stdout reply exceeds 1 MiB".into(),
                streams: Some(Box::new(streams)),
            });
        }
        let reply_bytes = output.stdout.clone();
        let mut streams = streams(output);
        streams.stdout.clear();
        let reply =
            parse_reply(&reply_bytes, &key).map_err(|error| error.with_streams(streams.clone()))?;
        validate_reply(&reply, context, &key)
            .map_err(|error| error.with_streams(streams.clone()))?;
        Ok((reply, streams))
    }
}

fn provider_root(row: &ExtensionRegistryRow) -> &Path {
    match row.provider() {
        ExtensionProvider::Dependency(provider) => &provider.root,
        ExtensionProvider::Host(provider) => &provider.root,
    }
}

fn verified_scratch(context: &Context, key: &str) -> Result<PathBuf, HandlerError> {
    let advertised = Path::new(&context.io.scratch);
    let run_id = advertised
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| HandlerError::Scratch {
            key: key.into(),
            reason: "scratch has no run component".into(),
        })?;
    let actual = execution_scratch(Path::new(&context.project.root), run_id, key)
        .map_err(|error| scratch_error(key, error))?;
    let expected = advertised
        .canonicalize()
        .map_err(|error| HandlerError::Scratch {
            key: key.into(),
            reason: error.to_string(),
        })?;
    if actual != expected {
        return Err(HandlerError::Scratch {
            key: key.into(),
            reason: "advertised scratch does not match key digest".into(),
        });
    }
    Ok(actual)
}

fn contained_existing(root: &Path, path: &Path, key: &str) -> Result<PathBuf, HandlerError> {
    let root = root
        .canonicalize()
        .map_err(|error| HandlerError::UnsafePath {
            key: key.into(),
            path: root.display().to_string(),
            reason: error.to_string(),
        })?;
    let path = path
        .canonicalize()
        .map_err(|error| HandlerError::UnsafePath {
            key: key.into(),
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
    if !path.starts_with(&root) {
        return Err(HandlerError::UnsafePath {
            key: key.into(),
            path: path.display().to_string(),
            reason: "canonical path escapes provider root".into(),
        });
    }
    Ok(path)
}

fn script_env(
    row: &ExtensionRegistryRow,
    target: Option<&SlotTarget>,
    context: &Context,
    context_path: &Path,
    reply: &Path,
) -> std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    let (group, name, version, kind, dir) = if let Some(target) = target {
        (
            target.group.clone(),
            target.name.clone(),
            target.version.clone(),
            target.kind.clone(),
            target.root.clone(),
        )
    } else {
        match row.provider() {
            ExtensionProvider::Dependency(provider) => (
                provider.id.group().to_string(),
                provider.id.name().to_string(),
                provider.version.clone(),
                provider.kind.to_string(),
                machine_path(&provider.root),
            ),
            ExtensionProvider::Host(provider) => (
                "__host__".into(),
                provider.identity.to_string(),
                provider.version.clone(),
                provider
                    .kind
                    .map(|kind| kind.to_string())
                    .unwrap_or_else(|| "project".into()),
                machine_path(&provider.root),
            ),
        }
    };
    let hook_phase = match row.declaration().point {
        ExtensionPoint::Slot(SlotPoint::PreInstall) => "pre-install",
        ExtensionPoint::Slot(SlotPoint::PostInstall) => "post-install",
        _ => "phase",
    };
    minimal_environment([
        ("VIBE_PACKAGE_GROUP".into(), group),
        ("VIBE_PACKAGE_NAME".into(), name),
        ("VIBE_PACKAGE_VERSION".into(), version),
        ("VIBE_PACKAGE_KIND".into(), kind),
        ("VIBE_PACKAGE_DIR".into(), dir),
        ("VIBE_HOOK_PHASE".into(), hook_phase.into()),
        ("VIBE_PROJECT_ROOT".into(), context.project.root.clone()),
        ("VIBE_EXTENSION_PROVIDER".into(), row.provider().to_string()),
        ("VIBE_CONTEXT".into(), machine_path(context_path)),
        ("VIBE_REPLY".into(), machine_path(reply)),
    ])
}

fn streams(output: ProcessOutput) -> HandlerStreams {
    HandlerStreams {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        stdout_truncated: output.stdout_truncated,
        stderr_truncated: output.stderr_truncated,
    }
}

fn scratch_error(key: &str, error: ScratchError) -> HandlerError {
    HandlerError::Scratch {
        key: key.into(),
        reason: error.to_string(),
    }
}

fn machine_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests;
