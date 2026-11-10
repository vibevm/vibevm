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

use vibe_core::manifest::{InstallerMethod, InstallerOptionDecl, LauncherDecl};

use crate::error::AppError;
use crate::index::{
    AcceptedLicense, ApplicationMenu, AuthoredCommands, Installation, InstalledTemplate, Lifecycle,
    Location, Method, MigrationEntryRecord, PackageCoordinate,
};
use crate::installer;
use crate::resolve::{ResolvedPackage, strip_verbatim};
use crate::steps::{Plan, Run, RunFlags, Start, StepPaths};
use crate::storage::Storage;

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
    /// The resolved package tree its package-relative icons are copied
    /// from (`PROP-059#menu`).
    pub package_root: PathBuf,
    pub retired: Vec<PathBuf>,
    pub finish_failure: Option<AppError>,
    /// The work directory `--keep-work` kept (`PROP-065#work`).
    pub work: Option<PathBuf>,
}

pub(super) struct Selected<'a> {
    pub operation: Option<&'a vibe_wire::generated::shared::ApplicationPendingTarget>,
    pub resolved: &'a ResolvedPackage,
    pub option: &'a InstallerOptionDecl,
    pub plan: &'a Plan,
    pub prior: Option<&'a Installation>,
    pub library: Option<&'a str>,
    pub force: bool,
    pub mode: Mode,
    pub start: &'a Start,
    pub command: Command,
    /// The key an update replaces, named in the run record.
    pub replaces: Option<String>,
    /// `--keep-work`: the work directory is not retired (`PROP-065#work`).
    pub keep_work: bool,
    /// The requirements' PATH entries per step.
    pub paths: StepPaths,
}

/// Existing agent consent, before pending publication/dependency/removal effects.
/// The temporary runner is read-only; execution constructs its runner once more
/// with actual dependency paths, without repeating the consent prompt.
pub(super) fn admit_agents(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    plan: &Plan,
    request: &super::InstallRequest,
    saved: Option<&crate::steps::StepsFile>,
) -> Result<(), AppError> {
    let run = Run::new(
        ctx,
        resolved,
        plan,
        RunFlags {
            operation: saved
                .and_then(|s| s.operation.as_ref())
                .map(|o| o.target.clone()),
            command: Command::Install,
            start: request.start.clone(),
            force: request.force,
            update_in_place: false,
            replaces: None,
            paths: StepPaths::default(),
        },
    )?;
    run.admit_agents()
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
            operation: selected.operation.cloned(),
            command: selected.command,
            start: selected.start.clone(),
            force: selected.force,
            update_in_place,
            replaces: selected.replaces.clone(),
            paths: selected.paths.clone(),
        },
    )?;
    let derive = |root: &Path| installed_launchers(ctx, selected, root);
    let placed = match plan.result.external_root() {
        None => place_owned(storage, selected, &mut run, &derive),
        Some(template) => run_external(ctx, selected, template, &mut run, &derive).map(
            |(root, failure, launchers)| {
                (
                    Location::External { app_root: root },
                    retired_owned(storage, selected.prior),
                    failure,
                    launchers,
                )
            },
        ),
    };
    run.close(match &placed {
        Ok((_, _, None, _)) => None,
        Ok((_, _, Some(failure), _)) | Err(failure) => Some(failure),
    });
    let (location, mut retired, finish_failure, launchers) = placed?;
    // A parked finish step keeps the work directory for `--resume`, and
    // `--keep-work` keeps it for the next `--from-step` (`PROP-065#work`).
    let kept_work = selected.keep_work.then(|| run.work().to_path_buf());
    if kept_work.is_none() && finish_failure.is_none() && selected.operation.is_none() {
        retired.push(run.work().to_path_buf());
    }
    let mut steps = run.records();
    let env = run.env_entries();
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
            source_evidence: selected.resolved.source_evidence.clone(),
            secret_origins: run.secret_origins(),
            configuration: plan.configuration.clone(),
            pending: None,
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
            launchers,
            public_commands: option.public_commands.clone(),
            path_dirs: option.path_dirs.clone(),
            version_file: selected.resolved.application.version_file.clone(),
            // What this invocation's template layer noted about the exact
            // version it resolved (`PROP-075#TEMPLATE-LOCK-RECORD`): the
            // installed record is where a globally installed application's
            // provenance lives, since a global install writes no lock.
            template: installed_template(&selected.resolved.package),
            // The authored entries and the ones vibe synthesises from the
            // declared ecosystem adapters, so an offer after `install -g`
            // and a later `--only ecosystem:<id>` need no package tree
            // (PROP-075 `#MIGRATION-DECLARATION`, `#ECOSYSTEM-MIGRATION`).
            migrations: selected
                .resolved
                .application
                .migration_entries()
                .iter()
                .map(|entry| MigrationEntryRecord {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    description: entry.description.clone(),
                    default: entry.default,
                })
                .collect(),
            // The outcomes belong to the key, not to one run: a modify of
            // the same version keeps what has already been carried into it
            // (`PROP-075#MIGRATION-FAILURE`).
            migrated: selected
                .prior
                .map(|prior| prior.migrated.clone())
                .unwrap_or_default(),
            // So does what an ecosystem adapter last applied and last
            // observed: it is a fact about this installed version, and a
            // reinstall of a version already installed applies nothing new
            // (`PROP-075#SLOT-EXTRAS`).
            extras: selected
                .prior
                .map(|prior| prior.extras.clone())
                .unwrap_or_default(),
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
            icon: selected.resolved.application.icon.clone(),
            menu: ApplicationMenu {
                menu_group: selected.resolved.application.menu_group.clone(),
                categories: selected.resolved.application.categories.clone(),
                keywords: selected.resolved.application.keywords.clone(),
            },
            steps,
            lifecycle: Lifecycle {
                env,
                ..Lifecycle::default()
            },
        },
        package_root: selected.resolved.root.clone(),
        retired,
        finish_failure,
        work: kept_work,
    })
}

/// The template provenance of one resolved version, read off this
/// invocation's installed template layer. `None` whenever the layer noted
/// nothing — an ordinary published package, or no layer at all — which is
/// what keeps a home without a rendered installation at
/// `vibe-user-applications/6` (`PROP-059#proxy-index-upgrade`).
fn installed_template(package: &PackageCoordinate) -> Option<InstalledTemplate> {
    let layer = vibe_registry::templates::active()?;
    let group = vibe_core::Group::parse(&package.group).ok()?;
    let version = semver::Version::parse(&package.version).ok()?;
    let noted = layer.noted(&group, &package.name, &version).template?;
    Some(InstalledTemplate {
        revision: noted.revision.to_string(),
        produced: package.version.clone(),
        rendered_sha256: noted.rendered_sha256,
        digest_source_sha256: Some(noted.digest_source_sha256),
        source_url: noted.source_url,
        digest_url: noted.digest_url,
    })
}

type Placed = (Location, Vec<PathBuf>, Option<AppError>, Vec<LauncherDecl>);

/// Derives the installed launchers once the payload is at its root.
type Derive<'a> = dyn Fn(&Path) -> Result<Vec<LauncherDecl>, AppError> + 'a;

/// The launchers an installation records: the option's rows merged with
/// those its `launchers_from` desktop entries derive under `root`, every
/// public command naming one of them (`PROP-059#desktop-entries`).
fn installed_launchers(
    ctx: &OperationContext<'_>,
    selected: &Selected<'_>,
    root: &Path,
) -> Result<Vec<LauncherDecl>, AppError> {
    let option = selected.option;
    let key = selected.resolved.package.key();
    let derived = if option.launchers_from.is_empty() {
        Vec::new()
    } else {
        crate::derive::derive_launchers(root, &option.launchers_from, ctx.search_path)
    };
    let launchers =
        crate::derive::merge(&option.launchers, derived).map_err(|reason| AppError::Launchers {
            key: key.clone(),
            reason,
        })?;
    if let Some(command) = option.public_commands.iter().find(|command| {
        !launchers
            .iter()
            .any(|launcher| launcher.id == command.launcher)
    }) {
        return Err(AppError::Launchers {
            key,
            reason: format!(
                "public command `{}` names launcher `{}`, which no row declares and no desktop entry derived",
                command.name, command.launcher
            ),
        });
    }
    Ok(launchers)
}

/// Run the prepare steps, then place `$VIBE_OUT` at
/// `<library>/apps/<slot>/<dir>`, replacing the same key in place without
/// inspecting the old payload; a failing step or hook moves the previous
/// payload back. The finish steps run against the placed payload.
fn place_owned(
    storage: &Storage,
    selected: &Selected<'_>,
    run: &mut Run<'_>,
    derive: &Derive<'_>,
) -> Result<Placed, AppError> {
    let prior_owned = selected.prior.and_then(|prior| match &prior.location {
        Location::Owned { library, directory } => Some((library.as_str(), directory.as_str())),
        Location::External { .. } => None,
    });
    // A new installation follows the install root; an update stays where
    // its record is (`PROP-019#install-root`): nothing moves unasked.
    let library = match selected.library.or(prior_owned.map(|(library, _)| library)) {
        Some(library) => library,
        None => storage.fresh_library()?,
    };
    let library_root = storage.usable(library)?;
    let slot = &selected.resolved.application.id;
    let directory = format!("apps/{slot}/{}", selected.resolved.package.directory_name());
    let target = crate::launch::library_path(&library_root, &directory);
    let owned_by_record = prior_owned == Some((library, directory.as_str()));
    if target.exists() && !owned_by_record && !run.has_placement() && !selected.force {
        return Err(AppError::ForeignDirectory {
            path: target.display().to_string(),
        });
    }
    let staging = crate::launch::library_path(
        &library_root,
        &format!(
            "apps/{slot}/.staging-{}-{}",
            selected.resolved.package.directory_name(),
            std::process::id()
        ),
    );
    fs::create_dir_all(&staging).map_err(|source| installer::io("creating", &staging, source))?;
    let payload = staging.join("payload");
    let previous = staging.join("previous");
    // Resuming a parked finish step: the payload is already in place.
    let placed_before =
        run.resumes_finish() && (owned_by_record || run.has_placement()) && target.is_dir();
    let placed = run.prepare(&target).and_then(|()| {
        if placed_before {
            return Ok(());
        }
        run.operation_phase(
            vibe_wire::generated::shared::ApplicationPendingOperationPhase::Placing,
        )?;
        run.take_out(&payload)?;
        if target.exists() {
            fs::rename(&target, &previous)
                .map_err(|source| installer::io("moving aside", &target, source))?;
        }
        fs::rename(&payload, &target)
            .map_err(|source| installer::io("placing", &target, source).into())
    });
    let placed = placed.and_then(|()| {
        run.operation_phase(vibe_wire::generated::shared::ApplicationPendingOperationPhase::Placed)
    });
    let launchers = placed.and_then(|()| {
        derive(&target).inspect_err(|_| {
            // Keep the payload for `--resume`; the previous one moves back.
            if !placed_before {
                let _ = fs::rename(&target, run.out());
                let _ = fs::remove_dir_all(&target);
            }
        })
    });
    let launchers = match launchers {
        Ok(launchers) => launchers,
        Err(error) => {
            restore(&target, &previous);
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    let finish_failure = match run.finish(&target) {
        Ok(failure) => failure,
        Err(error) => {
            // A resumed placed payload was not replaced by this attempt;
            // preserve its already-published record/root on hook failure.
            if !placed_before {
                let _ = fs::rename(&target, run.out());
                let _ = fs::remove_dir_all(&target);
            }
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
        retired.push(crate::launch::library_path(&root, prior_directory));
    }
    Ok((
        Location::Owned {
            library: library.into(),
            directory,
        },
        retired,
        finish_failure,
        launchers,
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
    derive: &Derive<'_>,
) -> Result<(PathBuf, Option<AppError>, Vec<LauncherDecl>), AppError> {
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
    let launchers = derive(&root)?;
    run.operation_phase(vibe_wire::generated::shared::ApplicationPendingOperationPhase::Placed)?;
    let failure = run.finish(&root)?;
    Ok((root, failure, launchers))
}

/// An owned directory of the same key that an external result supersedes.
fn retired_owned(storage: &Storage, prior: Option<&Installation>) -> Vec<PathBuf> {
    match prior.map(|prior| &prior.location) {
        Some(Location::Owned {
            library, directory, ..
        }) => storage
            .path_of(library)
            .map(|root| vec![crate::launch::library_path(&root, directory)])
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
