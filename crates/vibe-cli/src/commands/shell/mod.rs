//! `vibe shell`: an interactive shell whose PATH begins with the `opt/bin`
//! directory of each application home and whose `VIBE_APP_USE` carries the
//! session's provider choices. Nothing is written to disk.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#shell-command");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#session-selection");

mod parent;
#[cfg(windows)]
mod windows;

use std::ffi::{OsStr, OsString};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use vibe_app::{EnvOp, SESSION_OFF, SESSION_VARIABLE, SessionChoices, SettingsRoot};

use crate::cli::ShellArgs;
use crate::commands::app::{global, home};
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
    let mut withdrawn: Vec<PathBuf> = Vec::new();
    for entry in &args.uses {
        let (slot, key) = entry
            .split_once('=')
            .map(|(slot, key)| (slot.trim(), key.trim()))
            .unwrap_or((entry.trim(), ""));
        for settings in &homes {
            let index = vibe_app::Index::load(settings)?;
            if key == SESSION_OFF {
                withdrawn.extend(vibe_app::slot_link_dirs(settings, &index, slot));
                continue;
            }
            let storage = vibe_app::Storage::load(settings)?;
            bins.extend(vibe_app::slot_session_dirs(
                settings, &index, &storage, key,
            )?);
        }
    }
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
    let mut choices = match inherited_use {
        Some(value) => SessionChoices::parse(
            value
                .to_str()
                .with_context(|| format!("inherited {SESSION_VARIABLE} is not valid UTF-8"))?,
        )?,
        None => SessionChoices::default(),
    };
    for entry in with_choices.iter().chain(&args.uses) {
        choices.apply(entry)?;
    }
    let path = compose_path(&bins, inherited_path.as_deref(), &withdrawn)?;
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

/// Put each home's `opt/bin` at the front of `inherited`, in priority order,
/// unless the directory is already listed (compared canonically), and leave
/// out every directory a `--use SLOT=none` withdrew.
fn compose_path(
    homes: &[PathBuf],
    inherited: Option<&OsStr>,
    withdrawn: &[PathBuf],
) -> Result<OsString> {
    let same = |left: &Path, right: &Path| {
        left == right
            || matches!(
                (std::fs::canonicalize(left), std::fs::canonicalize(right)),
                (Ok(left), Ok(right)) if left == right
            )
    };
    let kept = |entry: &PathBuf| !withdrawn.iter().any(|gone| same(gone, entry));
    let existing: Vec<PathBuf> = inherited
        .map(|value| std::env::split_paths(value).filter(kept).collect())
        .unwrap_or_default();
    let front: Vec<PathBuf> = homes
        .iter()
        .filter(|home| kept(home))
        .filter(|home| !existing.iter().any(|entry| same(home, entry)))
        .cloned()
        .collect();
    std::env::join_paths(front.into_iter().chain(existing))
        .context("composing the shell PATH: a directory contains the PATH separator")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_bin_goes_first_unless_already_listed() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("opt/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let inherited = std::env::join_paths(["/usr/bin", "/bin"]).unwrap();
        let composed = compose_path(std::slice::from_ref(&bin), Some(&inherited), &[]).unwrap();
        let entries: Vec<PathBuf> = std::env::split_paths(&composed).collect();
        assert_eq!(entries[0], bin);
        assert_eq!(entries.len(), 3);

        let spelled = dir.path().join("opt/./bin");
        let listed = std::env::join_paths([Path::new("/usr/bin"), spelled.as_path()]).unwrap();
        let composed = compose_path(std::slice::from_ref(&bin), Some(&listed), &[]).unwrap();
        assert_eq!(composed, listed, "an already listed home is not repeated");

        let alone = compose_path(std::slice::from_ref(&bin), None, &[]).unwrap();
        assert_eq!(PathBuf::from(alone), bin);
    }

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#slot-off")]
    fn a_withdrawn_slot_link_directory_is_left_out_of_the_child_path() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("opt/paths/node/0");
        let bin = dir.path().join("opt/bin");
        std::fs::create_dir_all(&link).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let inherited = std::env::join_paths([link.as_path(), Path::new("/usr/bin")]).unwrap();
        // `--use node=none`: the slot's link directory goes, everything else
        // stays in order, and `opt/bin` still leads.
        let composed = compose_path(
            std::slice::from_ref(&bin),
            Some(&inherited),
            std::slice::from_ref(&link),
        )
        .unwrap();
        let entries: Vec<PathBuf> = std::env::split_paths(&composed).collect();
        assert_eq!(entries, vec![bin.clone(), PathBuf::from("/usr/bin")]);
        // A withdrawn directory is not put back at the front either.
        let composed = compose_path(
            &[link.clone(), bin.clone()],
            Some(&inherited),
            std::slice::from_ref(&link),
        )
        .unwrap();
        let entries: Vec<PathBuf> = std::env::split_paths(&composed).collect();
        assert_eq!(entries, vec![bin, PathBuf::from("/usr/bin")]);
    }
}
