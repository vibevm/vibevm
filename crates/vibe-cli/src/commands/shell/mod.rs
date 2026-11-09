//! `vibe shell`: an interactive shell whose PATH begins with the `opt/bin`
//! directory of each application home and whose `VIBE_APP_USE` carries the
//! session's provider choices. Nothing is written to disk.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#shell-command");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#session-selection");

mod parent;
#[cfg(windows)]
mod windows;

use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use vibe_app::{EnvOp, SESSION_VARIABLE, SettingsRoot};

use crate::cli::ShellArgs;
use crate::commands::app::{global, home, session};
use crate::output;

pub fn run(ctx: &output::Context, args: ShellArgs) -> Result<()> {
    if ctx.is_json() || ctx.is_unattended() {
        bail!("vibe shell requires an interactive terminal; --json and --unattended are refused");
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("vibe shell requires stdin and stdout attached to a terminal");
    }
    // `Auto` scope puts an existing project home ahead of the user home;
    // `-g` selects the user home alone.
    let project = home::project_at(&std::env::current_dir()?)?;
    let user = global::settings_root()?;
    let homes = session_homes(args.global, project.as_deref(), &user);
    let mut bins = Vec::new();
    let mut with_choices = Vec::new();
    for spelling in &args.with {
        let exposure = expose(&homes, spelling)?;
        bins.push(exposure.proxies.clone());
        with_choices.push(exposure.choice());
    }
    // A session choice for a slot that exposes payload directories is the
    // real payload, put in front of the slot's link: `--use SLOT=KEY`
    // shadows the global default without repointing anything, and
    // `--use SLOT=none` takes the slot's link directories out of this
    // child's PATH so a lookup falls through to the system program
    // (PROP-059 `#session-selection`, `#slot-off`).
    let selection = session::selection(&homes, &args.uses)?;
    let withdrawn: Vec<PathBuf> = selection.withdrawn;
    bins.extend(selection.bins);
    bins.extend(home::shell_bins(args.global, project.as_deref(), &user));
    let mut recorded = Vec::new();
    for settings in &homes {
        recorded.extend(vibe_app::recorded_environment(settings)?);
    }
    bins.extend(
        recorded
            .iter()
            .filter(|entry| entry.op == EnvOp::PathPrepend)
            .filter_map(|entry| entry.value.as_deref().map(PathBuf::from)),
    );
    let (inherited_path, inherited_use) = inherited();
    let inherited_use = match &inherited_use {
        Some(value) => Some(
            value
                .to_str()
                .with_context(|| format!("inherited {SESSION_VARIABLE} is not valid UTF-8"))?,
        ),
        None => None,
    };
    let entries: Vec<String> = with_choices.iter().chain(&args.uses).cloned().collect();
    let choices = session::choices(inherited_use, &entries)?;
    let path = session::compose_path(&bins, inherited_path.as_deref(), &withdrawn)?;
    let shell = parent::shell().context("selecting the interactive shell")?;
    let mut command = Command::new(&shell);
    command
        .env("PATH", path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for entry in &recorded {
        match (entry.op, &entry.value) {
            (EnvOp::Set, Some(value)) => {
                command.env(&entry.name, value);
            }
            (EnvOp::Unset, _) => {
                command.env_remove(&entry.name);
            }
            _ => {}
        }
    }
    if choices.is_empty() {
        command.env_remove(SESSION_VARIABLE);
    } else {
        command.env(SESSION_VARIABLE, choices.render());
    }
    launch(command, &shell)
}

/// The application homes of the session in priority order: the project
/// home when it exists and the scope is not `-g`, then the user home.
fn session_homes(
    global_scope: bool,
    project: Option<&Path>,
    user: &SettingsRoot,
) -> Vec<SettingsRoot> {
    let mut homes = Vec::new();
    if let (false, Some(root)) = (global_scope, project) {
        let settings = SettingsRoot::project(root, user);
        if settings.path().is_dir() {
            homes.push(settings);
        }
    }
    homes.push(user.clone());
    homes
}

/// Expose `spelling` from the first home that has it installed
/// (PROP-059 `#hidden-installations`, `#session-selection`).
fn expose(homes: &[SettingsRoot], spelling: &str) -> Result<vibe_app::Exposure> {
    let image = global::proxy_image()?;
    let mut last = None;
    for settings in homes {
        match vibe_app::expose_for_session(settings, spelling, &image) {
            Ok(exposure) => return Ok(exposure),
            Err(error) if error.code() == "not_installed" => last = Some(error),
            Err(error) => return Err(error.into()),
        }
    }
    Err(last
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow::anyhow!("`{spelling}` is not installed")))
}

/// The caller's `PATH` and `VIBE_APP_USE`, read once at this process
/// boundary and handed down as data.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "vibe shell composes the child shell's environment from the caller's PATH and VIBE_APP_USE; this boundary reads both once and passes them as values (spec://org.vibevm.core/vibevm/common/PROP-059#shell-command)."
)]
fn inherited() -> (Option<OsString>, Option<OsString>) {
    (std::env::var_os("PATH"), std::env::var_os(SESSION_VARIABLE))
}

#[cfg(unix)]
fn launch(mut command: Command, shell: &Path) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let error = command.exec();
    Err(error).with_context(|| format!("starting the shell `{}`", shell.display()))
}

#[cfg(windows)]
fn launch(command: Command, shell: &Path) -> Result<()> {
    windows::run(command, shell)
}

#[cfg(not(any(unix, windows)))]
fn launch(_command: Command, shell: &Path) -> Result<()> {
    bail!(
        "vibe shell cannot start `{}` on this platform",
        shell.display()
    )
}
