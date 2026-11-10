//! `vibe agent <launcher> [args…]` — start one launcher with this project's
//! composed boot lane already in the harness's own instruction slot.
//!
//! **A thin layer over `vibe prompt flags`, deliberately.** There is one
//! code path that decides which flags a kind takes, and this cell adds only
//! what printing cannot do: it recomposes the lane first, writes Codex's
//! launch-time profile file, refuses a user argument that would conflict
//! with the slot flag, and then becomes the harness (decision D10).
//!
//! **The lane is recomposed at every launch**, which is how band U keeps the
//! "current bytes" meaning `#SYS-NO-FROZEN-INDEX` gives it: a mid-session
//! edit of a user boot snippet is seen at the next launch, exactly as it is
//! today once the agent has read it (`#SYS-AGENT-LANE`).
//!
//! **No lane bytes go on a command line.** A single argument is capped at
//! 128 KiB on Linux and the whole command line at about 32,767 characters on
//! Windows, against a lane of roughly 310 KB, so every mechanism here is
//! file-based (`#SYS-HARNESS-DELIVERY`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use vibe_app::{AgentsConfig, AppError};
use vibe_workspace::boot_artifacts::build_node_prompt;
use vibe_workspace::system_prompt::{CodexProfile, HarnessKind};

use crate::commands::app::global;
use crate::commands::prompt;
use crate::output;

use super::launchers;

pub(crate) fn run(ctx: &output::Context, argv: Vec<String>) -> Result<()> {
    let mut argv = argv.into_iter();
    let Some(name) = argv.next() else {
        bail!("`vibe agent` needs a launcher name; `vibe agent list` names the ones you have");
    };
    let rest: Vec<String> = argv.collect();

    // A repository must never choose which program runs here, and this is
    // asked **before** the workspace is opened: the manifest parser would
    // reject the table as an unknown field, and "unknown field `agent`" is
    // not the sentence a contributor needs to read.
    launchers::refuse_project_launchers(Path::new("."))?;
    let selected = prompt::select(&PathBuf::from("."), None)?;
    let settings = global::settings_root()?;
    let config = AgentsConfig::load(&settings).map_err(AppError::from)?;
    let launcher = launchers::resolve(&settings, &config, &name)?;
    refuse_conflicting(launcher.kind, &rest)?;

    // Recompose first, so the slot carries the files as they are now.
    let built = build_node_prompt(&selected.workspace, &selected.node_rel);
    if !ctx.is_quiet() {
        eprintln!("{}", prompt::summary_line(&built));
    }
    built?;

    let composed = prompt::compose_flags(&selected, launcher.kind)?;
    if let Some(profile) = &composed.profile {
        write_profile(profile)?;
    }
    let mut command: Vec<String> = launcher.command.clone();
    command.extend(composed.flags.iter().cloned());
    command.extend(rest);
    exec(&command)
}

/// Refuse a user argument that would land a second system prompt in the
/// slot. Passing two is not an error the harness reports — it is a session
/// that silently takes a prompt nobody chose.
fn refuse_conflicting(kind: HarnessKind, rest: &[String]) -> Result<()> {
    for flag in kind.conflicting_flags() {
        if rest
            .iter()
            .any(|argument| argument == flag || argument.starts_with(&format!("{flag}=")))
        {
            bail!(
                "`{flag}` is already in your own arguments, and `vibe agent` passes the slot \
                 flag for kind `{}` itself — drop one of the two, or use `vibe prompt flags \
                 --harness {}` and compose the command line yourself",
                kind.as_str(),
                kind.as_str()
            );
        }
    }
    Ok(())
}

/// Write Codex's launch-time profile file into `$CODEX_HOME`.
///
/// Written by `vibe agent` and by nothing else — never by an install, which
/// has no business writing outside the project, and never by `vibe prompt
/// flags`, which only prints. `$CODEX_HOME` defaults to `~/.codex`, as Codex
/// itself resolves it (decision D4).
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: `$CODEX_HOME` is the harness's OWN variable, and \
              the file must land where the Codex this launch is about to \
              exec will look for it. Resolving it at the composition root \
              and threading it in would hand the write a value captured \
              before the launch was decided — a different fact from \
              `where will this child read its profile`, and the one place \
              where reading it later than the root is the correctness \
              requirement rather than a convenience"
)]
fn write_profile(profile: &CodexProfile) -> Result<()> {
    let home = match std::env::var_os("CODEX_HOME") {
        Some(home) => PathBuf::from(home),
        None => dirs::home_dir()
            .context("resolving the home directory for `$CODEX_HOME`")?
            .join(".codex"),
    };
    std::fs::create_dir_all(&home).with_context(|| format!("creating `{}`", home.display()))?;
    let path = home.join(&profile.file_name);
    std::fs::write(&path, &profile.body).with_context(|| format!("writing `{}`", path.display()))
}

/// Become the harness.
///
/// On a POSIX host this process is **replaced**, so the harness inherits the
/// terminal, the signals and the exit status with nothing of vibe's left in
/// the way. Windows has no `exec`: spawn, wait, and exit with the child's
/// own code, which is the same observable contract by the only means the
/// platform offers.
#[cfg(unix)]
fn exec(command: &[String]) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let (program, arguments) = command
        .split_first()
        .context("a launcher's `command` is an empty argv prefix")?;
    let error = Command::new(program).args(arguments).exec();
    Err(anyhow::Error::new(error).context(format!("starting `{program}`")))
}

#[cfg(not(unix))]
fn exec(command: &[String]) -> Result<()> {
    let (program, arguments) = command
        .split_first()
        .context("a launcher's `command` is an empty argv prefix")?;
    let status = Command::new(program)
        .args(arguments)
        .status()
        .with_context(|| format!("starting `{program}`"))?;
    std::process::exit(status.code().unwrap_or(1));
}
