//! `vibe app env`: the lines that switch the current shell to a selection
//! (PROP-075 `#ACTIVATION-ENV-OUTPUT`).
//!
//! Stdout carries the lines and nothing else, so `eval "$(vibe app env …)"`
//! is safe; everything the resolution reported goes to stderr, because a
//! prompt hook evaluates this output and must not choke on a notice.
//!
//! It sets exactly two things — `PATH`, composed by the same function
//! `vibe shell` uses, and `VIBE_APP_USE` — so
//! `eval "$(vibe app env --use node=22.23.4)"` and
//! `vibe shell --use node=22.23.4` put a shell in the identical state.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT");

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_app::{
    Activation, ActivationRequest, ActivationSource, ActivationState, Index, ProjectRequirement,
    SESSION_VARIABLE, SettingsRoot, SlotSelection,
};
use vibe_core::manifest::Manifest;
use vibe_wire::generated::application_env as wire;

use super::{global, session};
use crate::cli::AppEnvArgs;
use crate::output;
use crate::shell_syntax::Shell;

/// The variables `vibe app env` owns. Nothing else is touched.
const PATH_VARIABLE: &str = "PATH";

/// What one resolution produced, before it is rendered for a shell.
pub(crate) struct Resolved {
    pub shell: Shell,
    pub unset: bool,
    pub activation: Activation,
    pub path: OsString,
    pub choices: String,
}

pub(super) fn run(ctx: &output::Context, args: AppEnvArgs) -> Result<()> {
    let shell = match &args.shell {
        Some(name) => Shell::parse(name)?,
        None => Shell::detect(parent_shell().as_deref()),
    };
    let resolved = resolve(shell, &args)?;
    if ctx.is_json() {
        return ctx.emit_json(&document(&resolved));
    }
    for line in lines(&resolved) {
        println!("{line}");
    }
    for notice in notices(&resolved.activation) {
        eprintln!("vibe app env: {notice}");
    }
    Ok(())
}

/// The caller's `$SHELL`, read once at this process boundary.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "`vibe app env` prints lines for the shell that invoked it; the detector of PROP-019 §2.6 takes $SHELL as a value, and this boundary reads it once and hands it down (spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT)."
)]
fn parent_shell() -> Option<String> {
    std::env::var("SHELL").ok()
}

/// The caller's `VIBE_APP_USE` and `PATH`, read once at this process
/// boundary and handed down as data — the same boundary `vibe shell` has,
/// because the two surfaces compose the identical environment.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "`vibe app env` composes the lines that change the CALLING shell, so the caller's PATH and VIBE_APP_USE are its input; this boundary reads both once and passes them as values (spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT)."
)]
fn inherited() -> (Option<String>, Option<OsString>) {
    (
        std::env::var(SESSION_VARIABLE).ok(),
        std::env::var_os(PATH_VARIABLE),
    )
}

/// Resolve a selection and compose the child environment for it.
pub(crate) fn resolve(shell: Shell, args: &AppEnvArgs) -> Result<Resolved> {
    let cwd = std::env::current_dir()?;
    let project = enclosing_project(&cwd);
    let user = global::settings_root()?;
    let homes = session_homes(project.as_deref(), &user);
    let (inherited_use, inherited_path) = inherited();

    if args.unset {
        // nvm's `deactivate`: take out exactly the directories the inherited
        // selection put in and clear the variable.
        let entries = inherited_choices(inherited_use.as_deref());
        let applied = session::selection(&homes, &entries)?;
        let mut withdrawn = applied.bins;
        withdrawn.extend(applied.withdrawn);
        let path = retract(inherited_path.as_deref(), &withdrawn)?;
        return Ok(Resolved {
            shell,
            unset: true,
            activation: Activation {
                project: project.clone(),
                ..Activation::default()
            },
            path,
            choices: String::new(),
        });
    }

    let requirements = match (args.project, project.as_deref()) {
        (true, Some(root)) => project_requirements(root)?,
        _ => Vec::new(),
    };
    // One activation per home, the nearer home winning per slot: the project
    // home's installations answer before the user home's, exactly as
    // `vibe shell` orders their PATH.
    let mut activation = Activation {
        project: project.clone(),
        ..Activation::default()
    };
    // The channel table, read from the template cache alone — no network,
    // no registry call, no index read (PROP-075 `#ACTIVATION-HOOK`).
    let channels = super::channels::CachedChannels::new();
    for settings in &homes {
        let index = Index::load(settings)?;
        let request = ActivationRequest {
            uses: &args.uses,
            directory: &cwd,
            project: project.as_deref(),
            requirements: &requirements,
            consult_project: args.project,
            channels: Some(&channels),
        };
        let resolved = vibe_app::resolve_activation(settings, &index, &request)?;
        for notice in resolved.notices {
            if !activation.notices.contains(&notice) {
                activation.notices.push(notice);
            }
        }
        for selection in resolved.selections {
            match activation.of_slot(&selection.slot) {
                // A slot the nearer home could not satisfy is answered by
                // the next one.
                Some(seen) if seen.state == ActivationState::Selected => {}
                Some(_) => {
                    activation
                        .selections
                        .retain(|row| row.slot != selection.slot);
                    activation.selections.push(selection);
                }
                None => activation.selections.push(selection),
            }
        }
    }
    activation.selections.sort_by(|a, b| a.slot.cmp(&b.slot));

    let entries: Vec<String> = activation
        .selections
        .iter()
        .filter(|selection| {
            matches!(
                selection.state,
                ActivationState::Selected | ActivationState::Off
            )
        })
        .filter_map(SlotSelection::choice)
        .collect();
    let applied = session::selection(&homes, &entries)?;
    // `--project` makes PATH match this directory, which means retracting
    // what an earlier directory's selection put in front and this one does
    // not re-apply. Without that a `cd` out of a project would leave its
    // version on PATH and the hook could never undo anything
    // (PROP-075 `#ACTIVATION-HOOK`).
    let superseded: Vec<String> = inherited_choices(inherited_use.as_deref())
        .into_iter()
        .filter(|entry| !entries.contains(entry))
        .collect();
    let stale = session::selection(&homes, &superseded)?;
    let mut withdrawn = applied.withdrawn;
    withdrawn.extend(stale.bins);
    withdrawn.extend(stale.withdrawn);
    let choices = session::choices(None, &entries)?;
    let path = session::compose_path(&applied.bins, inherited_path.as_deref(), &withdrawn)?;
    Ok(Resolved {
        shell,
        unset: false,
        activation,
        path,
        choices: choices.render(),
    })
}

/// The `SLOT=KEY` entries an inherited `VIBE_APP_USE` carries.
fn inherited_choices(value: Option<&str>) -> Vec<String> {
    value
        .map(|value| {
            value
                .split(';')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The nearest ancestor of `from` that holds a `vibe.toml`. This is the
/// root the upward version-file search stops at
/// (PROP-075 `#VERSION-FILE-FORMATS`), so it is found from anywhere inside
/// the project rather than only from its own directory, which is what
/// `commands::app::home::project_at` answers for the project *home* scope.
pub(crate) fn enclosing_project(from: &Path) -> Option<PathBuf> {
    let mut current = Some(from);
    while let Some(dir) = current {
        if dir.join(Manifest::FILENAME).is_file() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

/// The homes of one session in priority order: the project home when it
/// exists, then the user home.
fn session_homes(project: Option<&Path>, user: &SettingsRoot) -> Vec<SettingsRoot> {
    let mut homes = Vec::new();
    if let Some(root) = project {
        let settings = SettingsRoot::project(root, user);
        if settings.path().is_dir() {
            homes.push(settings);
        }
    }
    homes.push(user.clone());
    homes
}

/// The project's requirements on packages, as data for the activation.
fn project_requirements(root: &Path) -> Result<Vec<ProjectRequirement>> {
    let path = root.join(Manifest::FILENAME);
    let manifest = Manifest::read(&path)
        .with_context(|| format!("reading the project manifest `{}`", path.display()))?;
    Ok(manifest
        .requires
        .packages
        .iter()
        .filter_map(|reference| {
            Some(ProjectRequirement {
                group: reference.group.as_ref()?.to_string(),
                name: reference.name.to_string(),
                version: reference.version.clone(),
                authored: match reference.version {
                    vibe_core::VersionSpec::Latest => "latest".to_owned(),
                    ref spec => spec.to_string(),
                },
            })
        })
        .collect())
}

/// `PATH` without `withdrawn`, keeping every other entry in order.
fn retract(inherited: Option<&std::ffi::OsStr>, withdrawn: &[PathBuf]) -> Result<OsString> {
    session::compose_path(&[], inherited, withdrawn)
}

/// The lines this resolution prints, in order.
pub(crate) fn lines(resolved: &Resolved) -> Vec<String> {
    let shell = resolved.shell;
    let mut lines = vec![shell.set_line(PATH_VARIABLE, &resolved.path.to_string_lossy())];
    if resolved.unset || resolved.choices.is_empty() {
        lines.push(shell.unset_line(SESSION_VARIABLE));
    } else {
        lines.push(shell.set_line(SESSION_VARIABLE, &resolved.choices));
    }
    lines
}

/// What the resolution reported and did not fail on, one human line each.
pub(crate) fn notices(activation: &Activation) -> Vec<String> {
    let mut notices = activation.notices.clone();
    for selection in &activation.selections {
        match selection.state {
            ActivationState::NotInstalled => notices.push(format!(
                "slot `{}` wants `{}`{}, which is not installed; run `{}`",
                selection.slot,
                selection.requested.as_deref().unwrap_or(""),
                selection
                    .file
                    .as_deref()
                    .map(|file| format!(" ({})", file.display()))
                    .unwrap_or_default(),
                selection
                    .install
                    .as_deref()
                    .unwrap_or("vibe install -g <package>")
            )),
            ActivationState::Ambiguous => notices.push(format!(
                "slot `{}` wants `{}`, which several installations match ({}); name one exactly",
                selection.slot,
                selection.requested.as_deref().unwrap_or(""),
                selection.candidates.join(", ")
            )),
            ActivationState::UnresolvedChannel => notices.push(format!(
                "slot `{}` wants channel `{}`, which this vibe cannot lower yet; `latest` and \
                 `stable` resolve over the installed versions",
                selection.slot,
                selection.requested.as_deref().unwrap_or("")
            )),
            ActivationState::Selected | ActivationState::Off => {}
        }
    }
    notices
}

/// The `vibe-app-env/1` answer.
pub(crate) fn document(resolved: &Resolved) -> wire::ApplicationEnv {
    wire::ApplicationEnv {
        protocol: "vibe-app-env/1".to_owned(),
        shell: match resolved.shell {
            Shell::Bash => wire::Shell::Bash,
            Shell::Zsh => wire::Shell::Zsh,
            Shell::Fish => wire::Shell::Fish,
            Shell::Pwsh => wire::Shell::Pwsh,
            Shell::Cmd => wire::Shell::Cmd,
            Shell::Posix => wire::Shell::Posix,
        },
        unset: resolved.unset,
        project: resolved
            .activation
            .project
            .as_deref()
            .map(|root| root.to_string_lossy().into_owned()),
        selections: resolved
            .activation
            .selections
            .iter()
            .map(selection_wire)
            .collect(),
        path: std::env::split_paths(&resolved.path)
            .map(|entry| entry.to_string_lossy().into_owned())
            .collect(),
        variables: if resolved.unset || resolved.choices.is_empty() {
            [(
                PATH_VARIABLE.to_owned(),
                resolved.path.to_string_lossy().into_owned(),
            )]
            .into()
        } else {
            [
                (
                    PATH_VARIABLE.to_owned(),
                    resolved.path.to_string_lossy().into_owned(),
                ),
                (SESSION_VARIABLE.to_owned(), resolved.choices.clone()),
            ]
            .into()
        },
        cleared: if resolved.unset || resolved.choices.is_empty() {
            vec![SESSION_VARIABLE.to_owned()]
        } else {
            Vec::new()
        },
        notices: notices(&resolved.activation),
    }
}

fn selection_wire(selection: &SlotSelection) -> wire::Selection {
    wire::Selection {
        slot: selection.slot.clone(),
        source: match selection.source {
            ActivationSource::Flag => wire::Source::Flag,
            ActivationSource::ProjectRequirement => wire::Source::ProjectRequirement,
            ActivationSource::VersionFile => wire::Source::VersionFile,
            ActivationSource::SlotDefault => wire::Source::SlotDefault,
            ActivationSource::SlotOff => wire::Source::SlotOff,
            ActivationSource::None => wire::Source::None,
        },
        state: match selection.state {
            ActivationState::Selected => wire::State::Selected,
            ActivationState::NotInstalled => wire::State::NotInstalled,
            ActivationState::Off => wire::State::Off,
            ActivationState::UnresolvedChannel => wire::State::UnresolvedChannel,
            ActivationState::Ambiguous => wire::State::Ambiguous,
        },
        requested: selection.requested.clone(),
        key: selection.key.clone(),
        version: selection.version.clone(),
        file: selection
            .file
            .as_deref()
            .map(|file| file.to_string_lossy().into_owned()),
        unused: selection.unused.clone(),
        candidates: selection.candidates.clone(),
        install: selection.install.clone(),
    }
}
