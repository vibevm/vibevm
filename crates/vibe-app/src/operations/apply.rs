//! Running one selected installer option: hooks, the installer child or the
//! archive placement, and the resulting installation record. Nothing here
//! writes the index; a failure leaves at most an orphan directory.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#force-and-hooks");

use std::fs;
use std::path::{Component, Path, PathBuf};

use vibe_core::manifest::{InstallerMethod, InstallerOptionDecl, ShellInterpreter};

use crate::error::AppError;
use crate::index::{AuthoredCommands, Installation, Location, Method};
use crate::installer::archive::{self, ArchiveFormat};
use crate::installer::hooks::{self, Hook};
use crate::installer::native::{self, NativeKind};
use crate::installer::{self, shell};
use crate::resolve::{ResolvedPackage, strip_verbatim};
use crate::storage::{DEFAULT_LIBRARY, Storage};

use super::OperationContext;

/// Which authored command an external shell method runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Install,
    Update,
}

/// The new record plus directories to delete once the index is written.
pub(super) struct Applied {
    pub installation: Installation,
    pub retired: Vec<PathBuf>,
}

pub(super) struct Selected<'a> {
    pub resolved: &'a ResolvedPackage,
    pub option: &'a InstallerOptionDecl,
    pub prior: Option<&'a Installation>,
    pub library: Option<&'a str>,
    pub force: bool,
    pub mode: Mode,
}

pub(super) fn apply(
    ctx: &OperationContext<'_>,
    storage: &Storage,
    selected: &Selected<'_>,
) -> Result<Applied, AppError> {
    let option = selected.option;
    let method = Method::of(&option.installation);
    if selected.library.is_some() && !method.is_owned() {
        return Err(AppError::LibraryForExternal {
            key: selected.resolved.package.key(),
            method: method.as_str(),
        });
    }
    let record = |location: Location, commands: AuthoredCommands| Installation {
        slot: selected.resolved.application.id.clone(),
        package: selected.resolved.package.clone(),
        option: option.id.clone(),
        platform: option.platform.clone(),
        arch: option.arch.clone(),
        method,
        portable: method.is_owned()
            && selected.resolved.application.portable
            && option.portable.unwrap_or(true),
        location,
        launchers: option.launchers.clone(),
        public_commands: option.public_commands.clone(),
        commands,
    };
    match &option.installation {
        InstallerMethod::SourceBuild { .. } => Err(AppError::NotImplemented {
            method: "source_build",
        }),
        InstallerMethod::Zip {
            source,
            sha256,
            payload_root,
        }
        | InstallerMethod::TarGz {
            source,
            sha256,
            payload_root,
        } => {
            let format = if method == Method::Zip {
                ArchiveFormat::Zip
            } else {
                ArchiveFormat::TarGz
            };
            let archive = Archive {
                format,
                source,
                sha256,
                payload_root: payload_root.as_deref(),
            };
            let (location, retired) = place_owned(ctx, storage, selected, &archive)?;
            Ok(Applied {
                installation: record(location, AuthoredCommands::default()),
                retired,
            })
        }
        InstallerMethod::Shell {
            app_root,
            install,
            force_install,
            update,
            uninstall,
            force_uninstall,
            interpreter,
        } => {
            let script = if selected.force {
                force_install.as_str()
            } else if selected.mode == Mode::Update
                && selected
                    .prior
                    .is_some_and(|prior| prior.method == Method::Shell)
            {
                update.as_deref().ok_or_else(|| AppError::MissingUpdate {
                    key: selected.resolved.package.key(),
                })?
            } else {
                install.as_str()
            };
            let installer = External::Shell {
                script,
                interpreter: *interpreter,
            };
            let root = run_external(ctx, selected, app_root, *interpreter, &installer)?;
            Ok(Applied {
                installation: record(
                    Location::External { app_root: root },
                    AuthoredCommands {
                        uninstall: Some(uninstall.clone()),
                        force_uninstall: Some(force_uninstall.clone()),
                        update: update.clone(),
                        interpreter: *interpreter,
                    },
                ),
                retired: retired_owned(storage, selected.prior),
            })
        }
        InstallerMethod::Exe {
            source,
            sha256,
            app_root,
            arguments,
            force_arguments,
            uninstall,
            force_uninstall,
        }
        | InstallerMethod::Msi {
            source,
            sha256,
            app_root,
            arguments,
            force_arguments,
            uninstall,
            force_uninstall,
        } => {
            let kind = if method == Method::Exe {
                NativeKind::Exe
            } else {
                NativeKind::Msi
            };
            let arguments = match (selected.force, force_arguments) {
                (false, _) => arguments.as_slice(),
                (true, Some(forced)) => forced.as_slice(),
                (true, None) => {
                    return Err(AppError::ForceUnsupported {
                        key: selected.resolved.package.key(),
                        reason: "the native installer declares no force_arguments",
                    });
                }
            };
            let installer = External::Native {
                kind,
                source,
                sha256,
                arguments,
            };
            let root = run_external(ctx, selected, app_root, None, &installer)?;
            Ok(Applied {
                installation: record(
                    Location::External { app_root: root },
                    AuthoredCommands {
                        uninstall: uninstall.clone(),
                        force_uninstall: force_uninstall.clone(),
                        update: None,
                        interpreter: None,
                    },
                ),
                retired: retired_owned(storage, selected.prior),
            })
        }
    }
}

struct Archive<'a> {
    format: ArchiveFormat,
    source: &'a str,
    sha256: &'a str,
    payload_root: Option<&'a Path>,
}

enum External<'a> {
    Shell {
        script: &'a str,
        interpreter: Option<ShellInterpreter>,
    },
    Native {
        kind: NativeKind,
        source: &'a str,
        sha256: &'a str,
        arguments: &'a [String],
    },
}

/// Fetch, verify and extract an archive into `<library>/apps/<slot>/<dir>`,
/// replacing the same key in place without inspecting the old payload; a
/// failing extraction or post-hook moves the previous payload back.
fn place_owned(
    ctx: &OperationContext<'_>,
    storage: &Storage,
    selected: &Selected<'_>,
    archive: &Archive<'_>,
) -> Result<(Location, Vec<PathBuf>), AppError> {
    let key = selected.resolved.package.key();
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
    let package_root = &selected.resolved.root;
    let hook_failed = |hook: Hook, error: crate::error::InstallerError| AppError::Hook {
        hook: hook.as_str(),
        key: key.clone(),
        reason: error.to_string(),
    };
    hooks::run(
        Hook::PreInstall,
        selected.option.pre_install.as_deref(),
        None,
        package_root,
        &target,
        ctx.json,
    )
    .map_err(|error| hook_failed(Hook::PreInstall, error))?;
    let staging = library_root.join(format!(
        "apps/{slot}/.staging-{}-{}",
        selected.resolved.package.directory_name(),
        std::process::id()
    ));
    fs::create_dir_all(&staging).map_err(|source| installer::io("creating", &staging, source))?;
    let payload = staging.join("payload");
    let previous = staging.join("previous");
    let placed = (|| {
        let artifact = staging.join("artifact");
        installer::fetch_verified(
            archive.source,
            archive.sha256,
            package_root,
            &artifact,
            ctx.offline,
        )?;
        archive::extract(&artifact, archive.format, archive.payload_root, &payload)?;
        if target.exists() {
            fs::rename(&target, &previous)
                .map_err(|source| installer::io("moving aside", &target, source))?;
        }
        fs::rename(&payload, &target).map_err(|source| installer::io("placing", &target, source))
    })();
    if let Err(error) = placed {
        restore(&target, &previous);
        let _ = fs::remove_dir_all(&staging);
        return Err(error.into());
    }
    if let Err(error) = hooks::run(
        Hook::PostInstall,
        selected.option.post_install.as_deref(),
        None,
        package_root,
        &target,
        ctx.json,
    ) {
        let _ = fs::remove_dir_all(&target);
        restore(&target, &previous);
        let _ = fs::remove_dir_all(&staging);
        return Err(hook_failed(Hook::PostInstall, error));
    }
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
    ))
}

fn restore(target: &Path, previous: &Path) {
    if previous.exists() && !target.exists() {
        let _ = fs::rename(previous, target);
    }
}

/// Run an external installer and register the `app_root` it produced.
fn run_external(
    ctx: &OperationContext<'_>,
    selected: &Selected<'_>,
    app_root_template: &str,
    interpreter: Option<ShellInterpreter>,
    installer: &External<'_>,
) -> Result<PathBuf, AppError> {
    let key = selected.resolved.package.key();
    let expected = expand_app_root(app_root_template, ctx.settings.path()).map_err(|reason| {
        AppError::MissingAppRoot {
            key: key.clone(),
            path: app_root_template.into(),
            reason,
        }
    })?;
    let package_root = &selected.resolved.root;
    let hook_failed = |hook: Hook, error: crate::error::InstallerError| AppError::Hook {
        hook: hook.as_str(),
        key: key.clone(),
        reason: error.to_string(),
    };
    hooks::run(
        Hook::PreInstall,
        selected.option.pre_install.as_deref(),
        interpreter,
        package_root,
        &expected,
        ctx.json,
    )
    .map_err(|error| hook_failed(Hook::PreInstall, error))?;
    match installer {
        External::Shell {
            script,
            interpreter,
        } => shell::run(
            *interpreter,
            script,
            package_root,
            &expected,
            "shell installer",
            ctx.json,
        )?,
        External::Native {
            kind,
            source,
            sha256,
            arguments,
        } => {
            if !cfg!(windows) {
                return Err(crate::error::InstallerError::WindowsOnly {
                    method: kind.as_str(),
                }
                .into());
            }
            let file = ctx.settings.applications_dir().join(format!(
                ".installer-{}.{}",
                std::process::id(),
                kind.as_str()
            ));
            let result =
                installer::fetch_verified(source, sha256, package_root, &file, ctx.offline)
                    .and_then(|()| native::run(*kind, &file, arguments, package_root, ctx.json));
            let _ = fs::remove_file(&file);
            result?;
        }
    }
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
    hooks::run(
        Hook::PostInstall,
        selected.option.post_install.as_deref(),
        interpreter,
        package_root,
        &root,
        ctx.json,
    )
    .map_err(|error| hook_failed(Hook::PostInstall, error))?;
    Ok(root)
}

/// An owned directory of the same key that an external method supersedes.
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
