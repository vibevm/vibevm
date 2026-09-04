//! Local process-group backend for already materialized isolated phase views.
//!
//! This backend deliberately does not advertise network denial, custom read
//! policy, spawn prevention, atomic JSON-result publication, or same-path COW.

use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use command_group::CommandGroup as _;
use sha2::{Digest, Sha256};
use vibe_safefs::Project;

use super::backend::{HealthBackend, sealed};
use super::model::*;
use super::output::drain_concurrently;
use super::tree::TreeSeal;

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
            graceful_termination: cfg!(unix),
            spawn_prevention: false,
            network_deny: false,
            bounded_output: true,
            atomic_result: false,
            bundle_materialization: false,
            same_display_path_view: false,
        }
    }

    fn execute(
        &mut self,
        request: BackendCommandRequest<'_>,
    ) -> Result<CommandExecution, HealthError> {
        validate_isolated_roots(&request.root, &request.protected_root)?;
        prove_tree(&request.protected_root, request.expected_tree)?;
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
        let argv = request
            .command
            .argv
            .iter()
            .map(|arg| {
                materialize_arg(arg, request.assets, request.custom_bundle, &request.scratch)
            })
            .collect::<Result<Vec<_>, _>>()?;

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
                terminate_group(&mut child, drain, request.termination_grace_seconds)?;
                return Err(HealthError::Cancelled {
                    phase: request.phase,
                    check_id: request.check_id,
                    disposition: match request.phase {
                        HealthPhase::Before => CancellationDisposition::RefuseBefore,
                        HealthPhase::After => CancellationDisposition::RollbackAfter,
                    },
                });
            }
            if Instant::now() >= deadline {
                terminate_group(&mut child, drain, request.termination_grace_seconds)?;
                return Err(HealthError::Execution(format!(
                    "healthcheck `{}` timed out after {} seconds; process group was terminated after a {}-second graceful phase where supported",
                    request.check_id, request.timeout_seconds, request.termination_grace_seconds
                )));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let (stdout, stderr) = drain
            .join()
            .map_err(|_| HealthError::Execution("health output drain panicked".to_owned()))??;
        drop(held_assets);
        prove_tree(&request.protected_root, request.expected_tree)?;
        Ok(CommandExecution {
            exit_code: status.code().unwrap_or(-1),
            stdout,
            stderr,
            // The backend does not advertise atomic_result, so a structured
            // protocol can never reach execution through capability preflight.
            result: None,
        })
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
) -> Result<(), HealthError> {
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
    child.wait().map_err(|error| {
        HealthError::Execution(format!("reaping terminated health process group: {error}"))
    })?;
    // Job/process-group termination is the ownership guarantee that makes this
    // join bounded. Returning while the reader lives would leak a descendant-
    // held pipe and is forbidden.
    drain
        .join()
        .map_err(|_| HealthError::Execution("health output drain panicked".to_owned()))??;
    Ok(())
}

#[cfg(unix)]
fn graceful_group(child: &mut command_group::GroupChild) -> Result<(), HealthError> {
    use command_group::{Signal, UnixChildExt as _};

    child.signal(Signal::SIGTERM).map_err(|error| {
        HealthError::Execution(format!("sending SIGTERM to health process group: {error}"))
    })
}

pub(crate) fn verify_asset(
    asset: &AssetIdentity,
    identity_project: &Project,
) -> Result<File, HealthError> {
    let path = Path::new(&asset.display_path);
    let mut file = open_identity_locked(path).map_err(|error| {
        HealthError::Execution(format!(
            "opening sealed health asset `{}`: {error}",
            path.display()
        ))
    })?;
    verify_named_asset(asset, identity_project)?;
    // Read the held handle as a second digest pass. On Windows its share mode
    // prevents replacement/write until the launch and final-name recheck end.
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let used = file.read(&mut buffer).map_err(|error| {
            HealthError::Execution(format!("hashing held asset `{}`: {error}", path.display()))
        })?;
        if used == 0 {
            break;
        }
        hash.update(&buffer[..used]);
    }
    if format!("sha256:{:x}", hash.finalize()) != asset.sha256 {
        return Err(HealthError::Execution(format!(
            "held asset `{}` digest changed",
            path.display()
        )));
    }
    Ok(file)
}

fn recheck_asset(asset: &AssetIdentity, identity_project: &Project) -> Result<(), HealthError> {
    verify_named_asset(asset, identity_project)
}

fn verify_named_asset(
    asset: &AssetIdentity,
    identity_project: &Project,
) -> Result<(), HealthError> {
    let path = Path::new(&asset.display_path);
    let pinned = Project::pin_absolute_file(path).map_err(|error| {
        HealthError::Execution(format!(
            "pinning named health asset `{}`: {error:#}",
            path.display()
        ))
    })?;
    let snapshot = pinned
        .read_snapshot_bounded(identity_project, 64 * 1024 * 1024)
        .map_err(|error| {
            HealthError::Execution(format!(
                "rechecking named health asset `{}`: {error:#}",
                path.display()
            ))
        })?;
    if snapshot.size != asset.bytes
        || format!("sha256:{}", snapshot.sha256) != asset.sha256
        || snapshot.unix_mode != asset.mode
    {
        return Err(HealthError::Execution(format!(
            "named health asset `{}` changed bytes, size, or mode",
            path.display()
        )));
    }
    let planned = asset.live_identity.ok_or_else(|| {
        HealthError::Unsupported(format!(
            "asset `{}` has no live opaque FileIdentity for exact launch",
            asset.id
        ))
    })?;
    if snapshot.identity != planned {
        return Err(HealthError::Execution(format!(
            "named health asset `{}` changed opaque filesystem identity",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn open_identity_locked(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;

    // FILE_SHARE_READ only: no compatible writer/deleter/renamer can acquire
    // the name until the held launch epoch ends.
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001)
        .open(path)
}

#[cfg(not(windows))]
fn open_identity_locked(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

fn materialize_arg(
    arg: &ExpandedArg,
    assets: &[AssetIdentity],
    bundle: Option<&CustomBundle>,
    scratch: &str,
) -> Result<String, HealthError> {
    match arg {
        ExpandedArg::Value(value) => Ok(value.clone()),
        ExpandedArg::AssetPath(id) => assets
            .iter()
            .find(|asset| &asset.id == id)
            .map(|asset| asset.display_path.clone())
            .ok_or_else(|| {
                HealthError::Execution(format!("argv refers to absent sealed asset `{id}`"))
            }),
        ExpandedArg::BundlePath(path) => {
            let _ = (path, bundle, scratch);
            Err(HealthError::Unsupported(
                "capability-relative atomic verifier-bundle materialization is unavailable"
                    .to_owned(),
            ))
        }
    }
}

fn validate_isolated_roots(root: &str, protected: &str) -> Result<(), HealthError> {
    let root = Path::new(root);
    let protected = Path::new(protected);
    if !root.is_absolute() || !protected.is_absolute() || root == protected {
        return Err(HealthError::Unsupported(
            "local health requires distinct absolute phase-view and protected roots".to_owned(),
        ));
    }
    if root.starts_with(protected) || protected.starts_with(root) {
        return Err(HealthError::Unsupported(
            "local health phase-view and protected roots must be disjoint".to_owned(),
        ));
    }
    Ok(())
}

fn prove_tree(root: &str, expected: &TreeSeal) -> Result<(), HealthError> {
    let observed = observe_tree(root)?;
    let differences = expected.compare(&observed);
    if differences.is_empty() {
        Ok(())
    } else {
        Err(HealthError::Tree(format!(
            "protected tree differs from its seal: {differences:?}"
        )))
    }
}

fn observe_tree(root: &str) -> Result<TreeSeal, HealthError> {
    let project = Project::open(Path::new(root)).map_err(|error| {
        HealthError::Tree(format!("opening protected tree `{root}`: {error:#}"))
    })?;
    let inventory = crate::inventory::collect(&project)
        .map_err(|error| HealthError::Tree(error.to_string()))?;
    Ok(TreeSeal::from_inventory(&inventory))
}
