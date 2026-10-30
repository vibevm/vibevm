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
use vibe_app::{SESSION_VARIABLE, SessionChoices};

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
    let bins = home::shell_bins(args.global, project.as_deref(), &global::settings_root()?);
    let (inherited_path, inherited_use) = inherited();
    let mut choices = match inherited_use {
        Some(value) => SessionChoices::parse(
            value
                .to_str()
                .with_context(|| format!("inherited {SESSION_VARIABLE} is not valid UTF-8"))?,
        )?,
        None => SessionChoices::default(),
    };
    for entry in &args.uses {
        choices.apply(entry)?;
    }
    let path = compose_path(&bins, inherited_path.as_deref())?;
    let shell = parent::shell().context("selecting the interactive shell")?;
    let mut command = Command::new(&shell);
    command
        .env("PATH", path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if choices.is_empty() {
        command.env_remove(SESSION_VARIABLE);
    } else {
        command.env(SESSION_VARIABLE, choices.render());
    }
    launch(command, &shell)
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
/// unless the directory is already listed (compared canonically).
fn compose_path(homes: &[PathBuf], inherited: Option<&OsStr>) -> Result<OsString> {
    let existing: Vec<PathBuf> = inherited
        .map(|value| std::env::split_paths(value).collect())
        .unwrap_or_default();
    let same = |left: &Path, right: &Path| {
        left == right
            || matches!(
                (std::fs::canonicalize(left), std::fs::canonicalize(right)),
                (Ok(left), Ok(right)) if left == right
            )
    };
    let front: Vec<PathBuf> = homes
        .iter()
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
        let composed = compose_path(std::slice::from_ref(&bin), Some(&inherited)).unwrap();
        let entries: Vec<PathBuf> = std::env::split_paths(&composed).collect();
        assert_eq!(entries[0], bin);
        assert_eq!(entries.len(), 3);

        let spelled = dir.path().join("opt/./bin");
        let listed = std::env::join_paths([Path::new("/usr/bin"), spelled.as_path()]).unwrap();
        let composed = compose_path(std::slice::from_ref(&bin), Some(&listed)).unwrap();
        assert_eq!(composed, listed, "an already listed home is not repeated");

        let alone = compose_path(std::slice::from_ref(&bin), None).unwrap();
        assert_eq!(PathBuf::from(alone), bin);
    }
}
