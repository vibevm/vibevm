//! `vibe app exec SLOT[@VER] -- COMMAND …`: run one command with that
//! version first on PATH, without changing any default, state or file
//! (PROP-075 `#VERSION-EXEC-SPELLING`).
//!
//! The selection, the PATH and `VIBE_APP_USE` come from the same
//! `commands::app::session` library `vibe shell --use` and `vibe app env`
//! call, so the three surfaces cannot drift. On a POSIX host the child
//! *replaces* this process, which is the strongest form of "the standard
//! streams are inherited, the exit status passes through unchanged and
//! POSIX signals are forwarded": there is no parent left to lose a signal.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING");

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};
use vibe_app::{
    ActivationRequest, ActivationState, Index, SESSION_VARIABLE, SettingsRoot, SlotSelection,
};

use super::{global, home, session};
use crate::cli::AppExecArgs;
use crate::output;

pub(super) fn run(ctx: &output::Context, args: AppExecArgs) -> Result<Option<i32>> {
    if args.command.is_empty() {
        bail!(
            "vibe app exec needs a command after `--`, as in `vibe app exec node@22 -- node -v` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING)"
        );
    }
    let (slot, requested) = match args.target.split_once('@') {
        Some((slot, version)) => (slot.trim(), Some(version.trim().to_owned())),
        None => (args.target.trim(), None),
    };
    if slot.is_empty() {
        bail!("vibe app exec needs a slot, as in `vibe app exec node@22 -- node -v`");
    }
    let cwd = std::env::current_dir()?;
    let project = home::project_at(&cwd)?;
    let user = global::settings_root()?;
    let mut homes = Vec::new();
    if let Some(root) = project.as_deref() {
        let settings = SettingsRoot::project(root, &user);
        if settings.path().is_dir() {
            homes.push(settings);
        }
    }
    homes.push(user.clone());

    let selection = select(&homes, slot, requested.as_deref(), project.as_deref(), &cwd)?;
    let Some(key) = selection
        .key
        .clone()
        .filter(|key| key != vibe_app::SESSION_OFF)
    else {
        return Err(refusal(slot, &selection).into());
    };
    let entries = vec![format!("{slot}={key}")];
    let applied = session::selection(&homes, &entries)?;
    let (inherited_use, inherited_path) = inherited();
    let choices = session::choices(inherited_use.as_deref(), &entries)?;
    let path = session::compose_path(&applied.bins, inherited_path.as_deref(), &applied.withdrawn)?;

    let mut command = Command::new(&args.command[0]);
    command.args(&args.command[1..]);
    command.env("PATH", &path);
    command.env(SESSION_VARIABLE, choices.render());
    ctx.suspend_progress(|| ());
    launch(command, Path::new(&args.command[0]))
}

/// The caller's `VIBE_APP_USE` and `PATH`, read once at this process
/// boundary: the command this exec runs inherits them with the selection in
/// front, exactly as `vibe shell` composes a child's.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "`vibe app exec` runs a command with the caller's environment plus one selection, so the caller's PATH and VIBE_APP_USE are its input; this boundary reads both once and passes them as values (spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING)."
)]
fn inherited() -> (Option<String>, Option<std::ffi::OsString>) {
    (
        std::env::var(SESSION_VARIABLE).ok(),
        std::env::var_os("PATH"),
    )
}

/// Resolve the slot's version by the ordinary precedence, or by the explicit
/// `@VER` when one is given.
fn select(
    homes: &[SettingsRoot],
    slot: &str,
    requested: Option<&str>,
    project: Option<&Path>,
    cwd: &Path,
) -> Result<SlotSelection> {
    let uses: Vec<String> = requested
        .map(|version| vec![format!("{slot}={version}")])
        .unwrap_or_default();
    let mut known = false;
    let mut last: Option<SlotSelection> = None;
    let channels = super::channels::CachedChannels::new();
    for settings in homes {
        let index = Index::load(settings)?;
        known = known || index.installations.values().any(|row| row.slot == slot);
        let request = ActivationRequest {
            uses: &uses,
            directory: cwd,
            project,
            requirements: &[],
            channels: Some(&channels),
            // With no explicit version the version files decide, which is
            // what `#VERSION-EXEC-SPELLING` means by "resolved exactly as
            // `#VERSION-FILE-FORMATS` prescribes".
            consult_project: requested.is_none(),
        };
        let activation = vibe_app::resolve_activation(settings, &index, &request)?;
        let found = activation
            .of_slot(slot)
            .cloned()
            // Nothing named this slot: its global default answers.
            .unwrap_or_else(|| vibe_app::slot_default(&index, slot));
        if found.state == ActivationState::Selected {
            return Ok(found);
        }
        last = Some(found);
    }
    if !known {
        return Err(vibe_app::AppError::UnknownSlot {
            slot: slot.to_owned(),
        }
        .into());
    }
    Ok(last.unwrap_or_else(|| SlotSelection {
        slot: slot.to_owned(),
        source: vibe_app::ActivationSource::None,
        state: ActivationState::NotInstalled,
        requested: requested.map(str::to_owned),
        key: None,
        version: None,
        file: None,
        unused: Vec::new(),
        candidates: Vec::new(),
        install: None,
    }))
}

/// The refusal a selection that chose nothing deserves, before any effect.
fn refusal(slot: &str, selection: &SlotSelection) -> vibe_app::AppError {
    match selection.state {
        ActivationState::Ambiguous => vibe_app::AppError::AmbiguousInstallation {
            package: format!(
                "{slot}@{} ({})",
                selection.requested.as_deref().unwrap_or(""),
                selection.candidates.join(", ")
            ),
        },
        ActivationState::Off => vibe_app::AppError::NotInstalled {
            package: slot.to_owned(),
            home: "the selected application home".to_owned(),
            fix: format!("the slot is off; `vibe app use {slot} --default` turns it back on"),
        },
        _ => vibe_app::AppError::NotInstalled {
            package: match &selection.requested {
                Some(requested) => format!("{slot}@{requested}"),
                None => slot.to_owned(),
            },
            home: "the selected application home".to_owned(),
            fix: selection.install.clone().unwrap_or_else(|| {
                format!("install a provider of `{slot}` with `vibe install -g <package>`")
            }),
        },
    }
}

#[cfg(unix)]
fn launch(mut command: Command, program: &Path) -> Result<Option<i32>> {
    use std::os::unix::process::CommandExt;
    // `exec` replaces this process: the streams, the exit status and every
    // signal belong to the command from here on.
    let error = command.exec();
    Err(anyhow::Error::from(error).context(format!("running `{}`", program.display())))
}

/// Windows has no `exec`: the command is spawned and waited on, and its
/// exit status is this process's. The console control handler the OS gives
/// a console process group already delivers Ctrl-C and Ctrl-Break to the
/// child, which is what signal forwarding means there.
#[cfg(not(unix))]
fn launch(mut command: Command, program: &Path) -> Result<Option<i32>> {
    use anyhow::Context;
    let status = command
        .status()
        .with_context(|| format!("running `{}`", program.display()))?;
    Ok(Some(status.code().unwrap_or(1)))
}
