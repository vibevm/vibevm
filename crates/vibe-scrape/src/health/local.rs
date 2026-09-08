//! Local process-group backend for already materialized isolated phase views.
//!
//! This backend deliberately does not advertise network denial, restricted
//! custom-read policy, spawn prevention, atomic JSON-result publication, or
//! same-path COW. It does materialize sealed bundles and supports the explicit
//! transaction-owned final-path reproof mode.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use command_group::CommandGroup as _;
use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use super::backend::{HealthBackend, sealed};
use super::model::*;
use super::output::drain_concurrently;

mod assets;

use super::tree::TreeSeal;
pub(crate) use assets::verify_asset;
use assets::{
    materialize_arg, materialize_bundle, observe_tree, prove_phase_trees, recheck_asset,
    validate_command_cwd, validate_isolated_roots,
};

pub struct LocalProcessBackend;

impl LocalProcessBackend {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LocalProcessBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl sealed::Sealed for LocalProcessBackend {}

impl HealthBackend for LocalProcessBackend {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            exact_executable_identity: cfg!(windows),
            filesystem_isolation: true,
            read_policy_enforcement: false,
            process_tree_containment: cfg!(windows),
            // Windows Job Objects provide forced whole-tree termination. The
            // plan records this as transactional forced-tree behavior rather
            // than claiming a graceful signal phase.
            graceful_termination: cfg!(unix),
            forced_tree_termination: cfg!(windows),
            spawn_prevention: false,
            network_deny: false,
            bounded_output: true,
            atomic_result: false,
            bundle_materialization: true,
            same_display_path_view: false,
        }
    }

    fn execute(
        &mut self,
        request: BackendCommandRequest<'_>,
    ) -> Result<CommandExecution, HealthError> {
        if !request.transactional_tree_reproof {
            validate_isolated_roots(&request.phase_root, &request.protected_root)?;
        } else if request.phase_root != request.protected_root {
            return Err(HealthError::Preparation(
                "transactional-tree-reproof requires root == protected_root".to_owned(),
            ));
        }
        validate_command_cwd(&request.phase_root, &request.root)?;
        prove_phase_trees(
            &request.phase_root,
            &request.protected_root,
            request.expected_tree,
        )?;
        std::fs::create_dir_all(&request.scratch).map_err(|error| {
            HealthError::Execution(format!(
                "creating health scratch `{}`: {error}",
                request.scratch
            ))
        })?;

        let identity_project =
            Project::open(Path::new(&request.protected_root)).map_err(|error| {
                HealthError::Execution(format!("opening identity comparison capability: {error:#}"))
            })?;
        let mut held_assets = Vec::with_capacity(request.assets.len());
        for asset in request.assets {
            held_assets.push(verify_asset(asset, &identity_project)?);
        }
        let executable = PathBuf::from(&request.command.executable.display_path);
        if matches!(
            request.command.executable.source,
            AssetSource::Bundle { .. }
        ) {
            return Err(HealthError::Unsupported(
                "direct custom launch requires an atomically materialized bundle executable"
                    .to_owned(),
            ));
        }
        if let Some(bundle) = request.custom_bundle {
            materialize_bundle(bundle, &request.scratch)?;
        }
        let argv = request
            .command
            .argv
            .iter()
            .map(|arg| {
                materialize_arg(arg, request.assets, request.custom_bundle, &request.scratch)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let actual_argv = std::iter::once(executable.display().to_string())
            .chain(argv.clone())
            .collect::<Vec<_>>();

        let mut command = Command::new(&executable);
        command
            .args(&argv)
            .current_dir(&request.root)
            .env_clear()
            .envs(&request.command.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.group_spawn().map_err(|error| {
            HealthError::Execution(format!(
                "spawning sealed executable `{}` as a process group: {error}",
                executable.display()
            ))
        })?;
        for asset in request.assets {
            recheck_asset(asset, &identity_project)?;
        }
        let stdout = child.inner().stdout.take().ok_or_else(|| {
            HealthError::Execution("spawned health child has no stdout pipe".to_owned())
        })?;
        let stderr = child.inner().stderr.take().ok_or_else(|| {
            HealthError::Execution("spawned health child has no stderr pipe".to_owned())
        })?;
        let stdout_cap = usize::try_from(request.max_stdout_bytes)
            .map_err(|_| HealthError::Execution("stdout cap exceeds platform usize".to_owned()))?;
        let stderr_cap = usize::try_from(request.max_stderr_bytes)
            .map_err(|_| HealthError::Execution("stderr cap exceeds platform usize".to_owned()))?;
        let drain =
            std::thread::spawn(move || drain_concurrently(stdout, stderr, stdout_cap, stderr_cap));
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(request.timeout_seconds))
            .ok_or_else(|| HealthError::Execution("health timeout overflow".to_owned()))?;
        let mut leader_status = None;
        let status = loop {
            if leader_status.is_none() {
                leader_status = child.try_wait().map_err(|error| {
                    HealthError::Execution(format!("waiting for health process group: {error}"))
                })?;
            }
            // Leader exit alone is insufficient: a descendant may still own
            // the inherited pipe handles. Completion requires both the leader
            // and the two full-stream drains.
            if let Some(status) = leader_status.filter(|_| drain.is_finished()) {
                break status;
            }
            if request.cancellation.is_cancelled() {
                let (status, stdout, stderr) =
                    terminate_group(&mut child, drain, request.termination_grace_seconds)?;
                let execution = CommandExecution {
                    step: request.command.step,
                    actual_argv,
                    exit_code: status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                    result: None,
                };
                if let Err(error) = prove_phase_trees(
                    &request.phase_root,
                    &request.protected_root,
                    request.expected_tree,
                ) {
                    return Err(HealthError::CommandChangedTree {
                        check_id: request.check_id,
                        detail: error.to_string(),
                        prior_checks: Vec::new(),
                        prior_executions: Vec::new(),
                        execution: Box::new(execution),
                    });
                }
                return Err(HealthError::Cancelled {
                    phase: request.phase,
                    check_id: request.check_id,
                    disposition: match request.phase {
                        HealthPhase::Before => CancellationDisposition::RefuseBefore,
                        HealthPhase::After => CancellationDisposition::RollbackAfter,
                    },
                    prior_checks: Vec::new(),
                    prior_executions: Vec::new(),
                    execution: Box::new(execution),
                });
            }
            if Instant::now() >= deadline {
                let (status, stdout, stderr) =
                    terminate_group(&mut child, drain, request.termination_grace_seconds)?;
                let execution = CommandExecution {
                    step: request.command.step,
                    actual_argv,
                    exit_code: status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                    result: None,
                };
                if let Err(error) = prove_phase_trees(
                    &request.phase_root,
                    &request.protected_root,
                    request.expected_tree,
                ) {
                    return Err(HealthError::CommandChangedTree {
                        check_id: request.check_id,
                        detail: error.to_string(),
                        prior_checks: Vec::new(),
                        prior_executions: Vec::new(),
                        execution: Box::new(execution),
                    });
                }
                return Err(HealthError::TimedOut {
                    phase: request.phase,
                    check_id: request.check_id,
                    timeout_seconds: request.timeout_seconds,
                    prior_checks: Vec::new(),
                    prior_executions: Vec::new(),
                    execution: Box::new(execution),
                });
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let (stdout, stderr) = drain
            .join()
            .map_err(|_| HealthError::Execution("health output drain panicked".to_owned()))??;
        drop(held_assets);
        let execution = CommandExecution {
            step: request.command.step,
            actual_argv,
            exit_code: status.code().unwrap_or(-1),
            stdout,
            stderr,
            // The backend does not advertise atomic_result, so a structured
            // protocol can never reach execution through capability preflight.
            result: None,
        };
        if let Err(error) = prove_phase_trees(
            &request.phase_root,
            &request.protected_root,
            request.expected_tree,
        ) {
            return Err(HealthError::CommandChangedTree {
                check_id: request.check_id,
                detail: error.to_string(),
                prior_checks: Vec::new(),
                prior_executions: Vec::new(),
                execution: Box::new(execution),
            });
        }
        Ok(execution)
    }

    fn reprove_tree(&mut self, context: &PhaseContext) -> Result<TreeSeal, HealthError> {
        observe_tree(&context.protected_root)
    }
}

type DrainJoin = std::thread::JoinHandle<Result<(StreamEvidence, StreamEvidence), HealthError>>;

fn terminate_group(
    child: &mut command_group::GroupChild,
    drain: DrainJoin,
    grace_seconds: u64,
) -> Result<(ExitStatus, StreamEvidence, StreamEvidence), HealthError> {
    #[cfg(not(unix))]
    let _ = grace_seconds;
    #[cfg(unix)]
    {
        graceful_group(child)?;
        let grace_deadline = Instant::now()
            .checked_add(Duration::from_secs(grace_seconds))
            .ok_or_else(|| HealthError::Execution("termination grace overflow".to_owned()))?;
        while Instant::now() < grace_deadline && !drain.is_finished() {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    child.kill().map_err(|error| {
        HealthError::Execution(format!("forcing health process group termination: {error}"))
    })?;
    let status = child.wait().map_err(|error| {
        HealthError::Execution(format!("reaping terminated health process group: {error}"))
    })?;
    // Job/process-group termination is the ownership guarantee that makes this
    // join bounded. Returning while the reader lives would leak a descendant-
    // held pipe and is forbidden.
    let (stdout, stderr) = drain
        .join()
        .map_err(|_| HealthError::Execution("health output drain panicked".to_owned()))??;
    Ok((status, stdout, stderr))
}

#[cfg(unix)]
fn graceful_group(child: &mut command_group::GroupChild) -> Result<(), HealthError> {
    use command_group::{Signal, UnixChildExt as _};

    child.signal(Signal::SIGTERM).map_err(|error| {
        HealthError::Execution(format!("sending SIGTERM to health process group: {error}"))
    })
}
