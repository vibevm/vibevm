//! The detached start (`PROP-059#launcher-catalog`): `vibe app start … --json`
//! starts a launcher and answers at once instead of waiting for it, so a
//! graphical client that draws the start menu is not blocked by the program
//! it launched. The child gets no terminal of the caller's: its stdin is
//! empty and its stdout and stderr go to one log file under
//! `<home>/cache/app-runs/`.
//!
//! The usage facts are recorded *before* the spawn, under the home's
//! mutation lock, and the lock is released again before the child exists —
//! a child must never inherit the lock of the home it was started from.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use specmark::spec;
use vibe_core::manifest::LauncherKind;
use vibe_wire::generated::application_start as wire;

use super::usage::Usage;
use crate::error::{AppError, LaunchError};
use crate::index::Installation;
use crate::launch::{TemplateVars, app_root, launch_arguments, resolve};
use crate::settings::{SettingsRoot, io};
use crate::storage::Storage;

/// Everything one detached start needs besides the installation.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub struct StartRequest<'a> {
    /// The home the launcher belongs to: `$vibe_home` of the templates, the
    /// owner of the usage facts and the home of the log.
    pub settings: &'a SettingsRoot,
    pub launcher_id: &'a str,
    pub extra_args: &'a [OsString],
    pub project_root: Option<&'a Path>,
}

/// Start one launch point detached and answer `vibe-app-start/1`: the
/// launcher key, the child's process id, the command and argument array it
/// was started with, its working directory and its log.
///
/// A `tui` launcher is startable here although it has no terminal of the
/// caller's: a graphical client opens a terminal of its own for one
/// (`PROP-059#launch-points`), and its output stays in the log meanwhile.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog")]
pub fn start(
    installation: &Installation,
    storage: &Storage,
    request: &StartRequest<'_>,
) -> Result<wire::ApplicationStart, AppError> {
    let settings = request.settings;
    let launcher = installation
        .launchers
        .iter()
        .find(|launcher| launcher.id == request.launcher_id)
        .ok_or_else(|| LaunchError::UnknownLauncher {
            id: request.launcher_id.into(),
        })?;
    let unavailable = |reason: String| LaunchError::Unavailable { reason };
    let root = app_root(installation, storage).map_err(unavailable)?;
    let resolved = resolve(
        launcher,
        &TemplateVars {
            app_root: root,
            vibe_home: settings.path().to_path_buf(),
            project_root: request.project_root.map(Path::to_path_buf),
        },
    )
    .map_err(unavailable)?;

    let key = super::launcher_key(&installation.package.key(), &launcher.id);
    // Before the spawn: a start that is recorded and then fails to start is
    // a miscount of one, while a start that runs and is never recorded
    // loses the fact a client's "recent" list is drawn from.
    Usage::record_start(settings, &key)?;

    let log = log_path(settings, installation, &launcher.id);
    let file = open_log(&log)?;
    let errors = file
        .try_clone()
        .map_err(|source| io("writing", &log, source))?;
    let args = launch_arguments(&resolved.args, request.extra_args);
    let mut command = Command::new(&resolved.executable);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(errors));
    if let Some(directory) = &resolved.working_directory {
        command.current_dir(directory);
    }
    detach(&mut command, launcher.effective_kind());
    let child = command.spawn().map_err(|error| LaunchError::Spawn {
        id: launcher.id.clone(),
        reason: error.to_string(),
    })?;
    Ok(wire::ApplicationStart {
        protocol: "vibe-app-start/1".into(),
        key,
        pid: u64::from(child.id()),
        command: resolved.executable.display().to_string(),
        args: args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
        cwd: resolved
            .working_directory
            .as_ref()
            .map(|path| path.display().to_string()),
        log: log.display().to_string(),
    })
}

/// `<home>/cache/app-runs/<group>.<name>@<version>/start-<launcher>-<ts>.log`:
/// the run records' own home (`PROP-065#runs`), one file per start. The
/// directory is named like the installation rather than like the launcher
/// key, because a key holds `/` and `#` and is therefore not a path
/// component.
fn log_path(settings: &SettingsRoot, installation: &Installation, launcher: &str) -> PathBuf {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    settings
        .app_runs_dir()
        .join(installation.package.directory_name())
        .join(format!("start-{launcher}-{stamp}.log"))
}

fn open_log(path: &Path) -> Result<File, AppError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|source| io("creating", parent, source))?;
    Ok(OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| io("writing", path, source))?)
}

/// Put the child out of reach of the caller's terminal, with the platform's
/// own means and no `unsafe`.
#[cfg(unix)]
fn detach(command: &mut Command, _kind: LauncherKind) {
    use std::os::unix::process::CommandExt;
    // A process group of its own: the terminal's ^C reaches the group it is
    // in the foreground of, which is no longer this child's.
    command.process_group(0);
}

#[cfg(windows)]
fn detach(command: &mut Command, kind: LauncherKind) {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    // A window of its own needs no console at all; a terminal program keeps
    // the console it would draw in (`PROP-059#launch-points`).
    command.creation_flags(match kind {
        LauncherKind::Gui => CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS,
        LauncherKind::Tui | LauncherKind::Command => CREATE_NEW_PROCESS_GROUP,
    });
}

#[cfg(not(any(unix, windows)))]
fn detach(_command: &mut Command, _kind: LauncherKind) {}
