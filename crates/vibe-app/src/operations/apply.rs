//! Running one selected installer option through the step engine: the
//! prepare steps, the owned placement of `$VIBE_OUT` or the registration of
//! an external root, the finish steps, and the resulting installation
//! record. Nothing here writes the index; a failure leaves at most an orphan
//! directory and the kept work directory.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#force-and-hooks");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#result");

use std::fs;
use std::path::{Component, Path, PathBuf};

use vibe_core::manifest::{InstallerMethod, InstallerOptionDecl};

use crate::error::AppError;
use crate::index::{AcceptedLicense, AuthoredCommands, Installation, Location, Method};
use crate::installer;
use crate::resolve::{ResolvedPackage, strip_verbatim};
use crate::steps::{Plan, Run, RunFlags, Start};
use crate::storage::{DEFAULT_LIBRARY, Storage};

use super::{Command, OperationContext};

/// Which authored command an external shell step runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Install,
    Update,
}

/// The new record, directories to delete once the index is written, and a
/// failed authored finish step to report after that.
pub(super) struct Applied {
    pub installation: Installation,
    pub retired: Vec<PathBuf>,
    pub finish_failure: Option<AppError>,
}

pub(super) struct Selected<'a> {
    pub resolved: &'a ResolvedPackage,
    pub option: &'a InstallerOptionDecl,
    pub plan: &'a Plan,
    pub prior: Option<&'a Installation>,
    pub library: Option<&'a str>,
    pub force: bool,
    pub mode: Mode,
    pub start: &'a Start,
    pub command: Command,
}

pub(super) fn apply(
    ctx: &OperationContext<'_>,
    storage: &Storage,
    selected: &Selected<'_>,
) -> Result<Applied, AppError> {
    let option = selected.option;
    let plan = selected.plan;
    let method = Method::of(option);
    let owned = plan.result.is_owned();
    if selected.library.is_some() && !owned {
        return Err(AppError::LibraryForExternal {
            key: selected.resolved.package.key(),
            method: method.as_str(),
        });
    }
    let update_in_place = selected.mode == Mode::Update
        && !owned
        && selected
            .prior
            .is_some_and(|prior| matches!(prior.location, Location::External { .. }));
    let mut run = Run::new(
        ctx,
        selected.resolved,
        plan,
        RunFlags {
            command: selected.command,
            start: selected.start.clone(),
            force: selected.force,
            update_in_place,
        },
    )?;
    let placed = match plan.result.external_root() {
        None => place_owned(storage, selected, &mut run),
        Some(template) => run_external(ctx, selected, template, &mut run).map(|(root, failure)| {
            (
                Location::External { app_root: root },
                retired_owned(storage, selected.prior),
                failure,
            )
        }),
    };
    run.close(match &placed {
        Ok((_, _, None)) => None,
        Ok((_, _, Some(failure))) | Err(failure) => Some(failure),
    });
    let (location, mut retired, finish_failure) = placed?;
    retired.push(run.work().to_path_buf());
    let mut steps = run.records();
    let commands = match &option.installation {
        Some(InstallerMethod::Shell {
            uninstall,
            force_uninstall,
            update,
            interpreter,
            ..
        }) => AuthoredCommands {
            uninstall: Some(uninstall.clone()),
            force_uninstall: Some(force_uninstall.clone()),
            update: update.clone(),
            interpreter: *interpreter,
        },
        Some(
            InstallerMethod::Exe {
                uninstall,
                force_uninstall,
                ..
            }
            | InstallerMethod::Msi {
                uninstall,
                force_uninstall,
                ..
            },
        ) => AuthoredCommands {
            uninstall: uninstall.clone(),
            force_uninstall: force_uninstall.clone(),
            update: None,
            interpreter: None,
        },
        _ => AuthoredCommands::default(),
    };
    if option.installation.is_some() {
        // The method form keeps its uninstall data in `commands`, as before.
        for step in &mut steps {
            step.uninstall = AuthoredCommands::default();
        }
    }
    Ok(Applied {
        installation: Installation {
            slot: selected.resolved.application.id.clone(),
            package: selected.resolved.package.clone(),
            option: option.id.clone(),
            platform: option.platform.clone(),
            arch: option.arch.clone(),
            method,
            portable: owned
                && selected.resolved.application.portable
                && option.portable.unwrap_or(true),
            location,
            launchers: option.launchers.clone(),
            public_commands: option.public_commands.clone(),
            commands,
            features: plan.features.clone(),
            parameters: plan
                .parameters
                .iter()
                .filter(|param| !param.secret)
                .map(|param| (param.id.clone(), param.value.clone()))
                .collect(),
            license: plan.license.as_ref().map(|license| {
                let kept = selected
                    .prior
                    .and_then(|prior| prior.license.clone())
                    .filter(|prior| prior.name == license.name);
                kept.unwrap_or_else(|| AcceptedLicense {
                    name: license.name.clone(),
                    accepted_at: chrono::Utc::now()
                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                })
            }),
            icon: selected
                .resolved
                .application
                .icon
                .as_ref()
                .map(|icon| icon.to_string_lossy().into_owned()),
            steps,
        },
        retired,
        finish_failure,
    })
}

type Placed = (Location, Vec<PathBuf>, Option<AppError>);

/// Run the prepare steps, then place `$VIBE_OUT` at
/// `<library>/apps/<slot>/<dir>`, replacing the same key in place without
/// inspecting the old payload; a failing step or hook moves the previous
/// payload back. The finish steps run against the placed payload.
fn place_owned(
    storage: &Storage,
    selected: &Selected<'_>,
    run: &mut Run<'_>,
) -> Result<Placed, AppError> {
    let prior_owned = selected.prior.and_then(|prior| match &prior.location {
        Location::Owned { library, directory } => Some((library.as_str(), directory.as_str())),
        Location::External { .. } => None,
    });
    let library = selected
        .library
        .or(prior_owned.map(|(library, _)| library))
        .unwrap_or(DEFAULT_LIBRARY);
    let library_root = storage.usable(library)?;
    let slot = &selected.resolved.application.id;
    let directory = format!("apps/{slot}/{}", selected.resolved.package.directory_name());
    let target = library_root.join(&directory);
    let owned_by_record = prior_owned == Some((library, directory.as_str()));
    if target.exists() && !owned_by_record && !selected.force {
        return Err(AppError::ForeignDirectory {
            path: target.display().to_string(),
        });
    }
    let staging = library_root.join(format!(
        "apps/{slot}/.staging-{}-{}",
        selected.resolved.package.directory_name(),
        std::process::id()
    ));
    fs::create_dir_all(&staging).map_err(|source| installer::io("creating", &staging, source))?;
    let payload = staging.join("payload");
    let previous = staging.join("previous");
    let placed = run
        .prepare(&target)
        .and_then(|()| run.take_out(&payload))
        .and_then(|()| {
            if target.exists() {
                fs::rename(&target, &previous)
                    .map_err(|source| installer::io("moving aside", &target, source))?;
            }
            fs::rename(&payload, &target)
                .map_err(|source| installer::io("placing", &target, source).into())
        });
    if let Err(error) = placed {
        restore(&target, &previous);
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let finish_failure = match run.finish(&target) {
        Ok(failure) => failure,
        Err(error) => {
            // Keep the payload for `--resume` when it can move back.
            let _ = fs::rename(&target, run.out());
            let _ = fs::remove_dir_all(&target);
            restore(&target, &previous);
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    let mut retired = vec![staging];
    if let Some((prior_library, prior_directory)) = prior_owned
        && (prior_library != library || prior_directory != directory)
        && let Some(root) = storage.path_of(prior_library)
    {
        retired.push(root.join(prior_directory));
    }
    Ok((
        Location::Owned {
            library: library.into(),
            directory,
        },
        retired,
        finish_failure,
    ))
}

fn restore(target: &Path, previous: &Path) {
    if previous.exists() && !target.exists() {
        let _ = fs::rename(previous, target);
    }
}

/// Run the prepare steps and register the `app_root` they produced, then
/// the finish steps against it.
fn run_external(
    ctx: &OperationContext<'_>,
    selected: &Selected<'_>,
    app_root_template: &str,
    run: &mut Run<'_>,
) -> Result<(PathBuf, Option<AppError>), AppError> {
    let key = selected.resolved.package.key();
    let expected = expand_app_root(app_root_template, ctx.settings.path()).map_err(|reason| {
        AppError::MissingAppRoot {
            key: key.clone(),
            path: app_root_template.into(),
            reason,
        }
    })?;
    run.prepare(&expected)?;
    let root = fs::symlink_metadata(&expected)
        .ok()
        .filter(|metadata| metadata.is_dir())
        .and_then(|_| fs::canonicalize(&expected).ok())
        .map(strip_verbatim)
        .ok_or_else(|| AppError::MissingAppRoot {
            key: key.clone(),
            path: expected.display().to_string(),
            reason: "it is not a real directory after the installer finished".into(),
        })?;
    let failure = run.finish(&root)?;
    Ok((root, failure))
}

/// An owned directory of the same key that an external result supersedes.
fn retired_owned(storage: &Storage, prior: Option<&Installation>) -> Vec<PathBuf> {
    match prior.map(|prior| &prior.location) {
        Some(Location::Owned {
            library, directory, ..
        }) => storage
            .path_of(library)
            .map(|root| vec![root.join(directory)])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// An authored external `app_root`: absolute, or under `$vibe_home`.
pub(super) fn expand_app_root(template: &str, settings_root: &Path) -> Result<PathBuf, String> {
    let expanded = match template
        .strip_prefix("${vibe_home}")
        .or_else(|| template.strip_prefix("$vibe_home"))
    {
        Some(rest) => format!("{}{rest}", settings_root.display()),
        None => template.to_owned(),
    };
    let path = PathBuf::from(expanded);
    if !path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
        return Err("app_root must resolve to an absolute path without `..`".into());
    }
    Ok(path)
}
