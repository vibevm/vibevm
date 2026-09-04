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
            let bundle = bundle.ok_or_else(|| {
                HealthError::Execution("bundle argv has no sealed custom bundle".to_owned())
            })?;
            let entry = bundle
                .entries
                .iter()
                .find(|entry| entry.path == *path)
                .ok_or_else(|| {
                    HealthError::Execution(format!("bundle member `{path}` is absent"))
                })?;
            if entry.kind != BundleEntryKind::File {
                return Err(HealthError::Execution(format!(
                    "bundle argv member `{path}` is not a regular file"
                )));
            }
            Ok(bundle_target(scratch, path).display().to_string())
        }
    }
}

fn materialize_bundle(bundle: &CustomBundle, scratch: &str) -> Result<(), HealthError> {
    let root = Path::new(scratch).join("verifier-bundle");
    std::fs::create_dir_all(&root).map_err(|error| {
        HealthError::Execution(format!("creating verifier bundle root: {error}"))
    })?;
    for entry in &bundle.entries {
        let target = bundle_target(scratch, &entry.path);
        match entry.kind {
            BundleEntryKind::Directory => {
                std::fs::create_dir_all(&target).map_err(|error| {
                    HealthError::Execution(format!(
                        "materializing verifier bundle directory `{}`: {error}",
                        entry.path
                    ))
                })?;
            }
            BundleEntryKind::File => {
                let bytes = entry.content.as_ref().ok_or_else(|| {
                    HealthError::Execution(format!(
                        "bundle file `{}` has no sealed bytes",
                        entry.path
                    ))
                })?;
                if entry.bytes != Some(bytes.len() as u64)
                    || entry.sha256.as_deref()
                        != Some(format!("sha256:{:x}", Sha256::digest(bytes)).as_str())
                {
                    return Err(HealthError::Execution(format!(
                        "bundle file `{}` differs from its sealed size/digest",
                        entry.path
                    )));
                }
                let parent = target.parent().ok_or_else(|| {
                    HealthError::Execution("bundle target has no parent".to_owned())
                })?;
                std::fs::create_dir_all(parent).map_err(|error| {
                    HealthError::Execution(format!(
                        "creating verifier bundle parent for `{}`: {error}",
                        entry.path
                    ))
                })?;
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)
                {
                    Ok(mut file) => {
                        use std::io::Write as _;
                        file.write_all(bytes)
                            .and_then(|()| file.sync_all())
                            .map_err(|error| {
                                HealthError::Execution(format!(
                                    "materializing verifier bundle `{}`: {error}",
                                    entry.path
                                ))
                            })?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let existing = std::fs::read(&target).map_err(|read| {
                            HealthError::Execution(format!(
                                "re-reading verifier bundle `{}`: {read}",
                                entry.path
                            ))
                        })?;
                        if existing != *bytes {
                            return Err(HealthError::Execution(format!(
                                "verifier bundle `{}` already exists with different bytes",
                                entry.path
                            )));
                        }
                    }
                    Err(error) => {
                        return Err(HealthError::Execution(format!(
                            "creating verifier bundle `{}`: {error}",
                            entry.path
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

fn bundle_target(scratch: &str, path: &str) -> PathBuf {
    Path::new(scratch)
        .join("verifier-bundle")
        .join(path.replace('/', std::path::MAIN_SEPARATOR_STR))
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

fn validate_command_cwd(phase_root: &str, cwd: &str) -> Result<(), HealthError> {
    let phase_root = Path::new(phase_root);
    let cwd = Path::new(cwd);
    if !phase_root.is_absolute() || !cwd.is_absolute() || !cwd.starts_with(phase_root) {
        return Err(HealthError::Preparation(
            "health command cwd is not contained by the exact phase root".to_owned(),
        ));
    }
    let relative = cwd.strip_prefix(phase_root).map_err(|error| {
        HealthError::Preparation(format!("deriving health cwd from phase root: {error}"))
    })?;
    if relative.components().any(|component| {
        !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return Err(HealthError::Preparation(
            "health command cwd contains a non-portable component".to_owned(),
        ));
    }
    let project = Project::open(phase_root).map_err(|error| {
        HealthError::Preparation(format!("opening exact phase root for cwd proof: {error:#}"))
    })?;
    let portable = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !portable.is_empty() {
        project.dir(&portable, false).map_err(|error| {
            HealthError::Preparation(format!("proving nested health cwd no-follow: {error:#}"))
        })?;
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

fn prove_phase_trees(root: &str, protected: &str, expected: &TreeSeal) -> Result<(), HealthError> {
    prove_tree(protected, expected)?;
    if root != protected {
        prove_tree(root, expected)?;
    }
    Ok(())
}

fn observe_tree(root: &str) -> Result<TreeSeal, HealthError> {
    let project = Project::open(Path::new(root)).map_err(|error| {
        HealthError::Tree(format!("opening protected tree `{root}`: {error:#}"))
    })?;
    let inventory = crate::inventory::collect(&project)
        .map_err(|error| HealthError::Tree(error.to_string()))?;
    Ok(TreeSeal::from_inventory(&inventory))
}
