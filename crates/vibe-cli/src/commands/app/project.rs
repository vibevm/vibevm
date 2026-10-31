//! Project-local applications (PROP-059 `#project-scope`). The ordinary
//! project install, update and uninstall record app packages in `vibe.toml`
//! and `vibe.lock` like any other package; this adapter then brings the
//! project home `vibevm/vibeopt` to the lockfile's app dependencies through
//! the same `vibe-app` operations `-g` uses. The lockfile is the record; the
//! project home is a materialisation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#project-scope");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_app::{
    FeatureRequest, Index, InstallChoice, InstallRequest, OperationContext, PackageCoordinate,
    ResolvedPackage, SettingsRoot, Start,
};
use vibe_core::PackageKind;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest};
use vibe_workspace::vibedeps;

use super::{global, home};
use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::commands::{install, uninstall as project_uninstall, update as project_update};
use crate::output;

/// The group whose installers run without explicit consent, as for
/// `vibe bin build` (PROP-025 `#security`).
const ALLOW_LISTED_GROUP: &str = "org.vibevm";

/// The selection and consent flags one materialisation applies to every app
/// dependency it installs.
struct Materialise<'a> {
    verb: &'static str,
    choice: InstallChoice,
    features: FeatureRequest,
    start: Start,
    force: bool,
    consent: bool,
    offline: bool,
    root: &'a Path,
}

/// `vibe install [PACKAGE…]` inside a project: the ordinary project install,
/// then every app dependency of the lockfile that the project home lacks.
pub(crate) fn install(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    home::refuse_library(args.library.as_deref())?;
    let choice = InstallChoice {
        platform: args.platform.clone(),
        arch: args.arch.clone(),
        installer: args.installer.clone(),
    };
    let (force, consent) = (args.force, args.assume_yes || ctx.is_unattended());
    let offline = root_offline || args.offline;
    let path = args.path.clone();
    let (features, start) = global::step_flags(&args.steps);
    install::run_direct(ctx, args, embedded_root, root_offline)?;
    let Some(root) = home::project_at(&path)? else {
        return Ok(());
    };
    materialise(
        ctx,
        &Materialise {
            verb: "install",
            choice,
            features,
            start,
            force,
            consent,
            offline,
            root: &root,
        },
    )
}

/// `vibe update` inside a project: the ordinary project update, then the
/// project home follows the lockfile (a new locked version of an installed
/// app is applied as an application update).
pub(crate) fn update(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
    root: &Path,
) -> Result<()> {
    home::refuse_library(args.library.as_deref())?;
    let (features, start) = global::step_flags(&args.steps);
    let flags = Materialise {
        verb: "update",
        choice: InstallChoice {
            platform: args.platform.clone(),
            arch: args.arch.clone(),
            installer: args.installer.clone(),
        },
        features,
        start,
        force: args.force,
        consent: args.assume_yes || ctx.is_unattended(),
        offline: root_offline,
        root,
    };
    project_update::run(ctx, args, embedded_root, root_offline)?;
    materialise(ctx, &flags)
}

/// `vibe uninstall PACKAGE` inside a project: every project-home
/// installation of an app dependency is uninstalled there first, then the
/// ordinary project uninstall removes its dependency row.
pub(crate) fn uninstall(
    ctx: &output::Context,
    args: UninstallArgs,
    root_offline: bool,
    root: &Path,
) -> Result<()> {
    let settings = SettingsRoot::project(root, &global::settings_root()?);
    let installed = installed_app_dependency(root, &settings, &args.package)?;
    let mut outcomes = Vec::new();
    if !installed.is_empty() {
        let proxy_image = global::proxy_image()?;
        let operation = OperationContext {
            chooser: None,
            feature_chooser: None,
            ..global::operation(ctx, &settings, root_offline, &proxy_image, None)
        };
        for exact in &installed {
            outcomes
                .push(ctx.suspend_progress(|| vibe_app::uninstall(&operation, exact, args.force))?);
        }
    }
    project_uninstall::run(ctx, args)?;
    for outcome in outcomes {
        ctx.summary(&format!(
            "vibe uninstall: application `{}` — {} in the project home `{}`",
            outcome.slot,
            outcome.message,
            settings.path().display()
        ));
    }
    Ok(())
}

/// The project-home installations of `spelling` when it names an app
/// dependency of the lockfile, each as an exact `group/name@=version`.
fn installed_app_dependency(
    root: &Path,
    settings: &SettingsRoot,
    spelling: &str,
) -> Result<Vec<String>> {
    let Ok(requested) = vibe_app::qualified_ref(spelling) else {
        return Ok(Vec::new());
    };
    let Some(group) = requested.group.as_ref() else {
        return Ok(Vec::new());
    };
    let locked = app_dependencies(root)?
        .into_iter()
        .any(|package| &package.group == group && package.name == requested.name.as_str());
    if !locked {
        return Ok(Vec::new());
    }
    let group = group.to_string();
    Ok(Index::load(settings)?
        .installations
        .values()
        .filter(|record| {
            record.package.group == group && record.package.name == requested.name.as_str()
        })
        .map(|record| {
            let package = &record.package;
            format!("{}/{}@={}", package.group, package.name, package.version)
        })
        .collect())
}

/// Install every app dependency of the lockfile whose key the project home
/// lacks; a key already installed is skipped.
fn materialise(ctx: &output::Context, flags: &Materialise<'_>) -> Result<()> {
    let settings = SettingsRoot::project(flags.root, &global::settings_root()?);
    let index = Index::load(&settings)?;
    let pending: Vec<LockedPackage> = app_dependencies(flags.root)?
        .into_iter()
        .filter(|package| !index.installations.contains_key(&coordinate(package).key()))
        .collect();
    if pending.is_empty() {
        return Ok(());
    }
    consent(&pending, &settings, flags)?;
    if !settings.path().exists() {
        let entry = vibe_app::PROJECT_HOME.join("/");
        vibedeps::ensure_gitignored(flags.root, &entry)
            .with_context(|| format!("ignoring `{entry}` in the project's .gitignore"))?;
    }
    let proxy_image = global::proxy_image()?;
    let request = InstallRequest {
        choice: flags.choice.clone(),
        library: None,
        force: flags.force,
        features: flags.features.clone(),
        start: flags.start.clone(),
    };
    let show = global::plan_printer(ctx);
    for package in &pending {
        let resolved = from_slot(flags.root, package)?;
        let previous = Index::load(&settings)?
            .installations
            .values()
            .any(|record| record.package.same_package(&resolved.package));
        let outcome = ctx.suspend_progress(|| {
            let operation =
                global::operation(ctx, &settings, flags.offline, &proxy_image, Some(&show));
            if previous {
                vibe_app::update(&operation, &resolved, &request)
            } else {
                vibe_app::install(&operation, &resolved, &request)
            }
        })?;
        ctx.summary(&format!(
            "vibe {}: application `{}` — {} in the project home `{}`",
            flags.verb,
            outcome.slot,
            outcome.message,
            settings.path().display()
        ));
    }
    Ok(())
}

/// Installers run package-authored code: like `vibe bin build`, only the
/// allow-listed group proceeds without `--assume-yes` (or `--unattended`).
fn consent(
    pending: &[LockedPackage],
    settings: &SettingsRoot,
    flags: &Materialise<'_>,
) -> Result<()> {
    if flags.consent {
        return Ok(());
    }
    let refused: Vec<String> = pending
        .iter()
        .filter(|package| package.group.to_string() != ALLOW_LISTED_GROUP)
        .map(|package| coordinate(package).key())
        .collect();
    if refused.is_empty() {
        return Ok(());
    }
    bail!(
        "violates spec://org.vibevm.core/vibevm/common/PROP-059#project-scope: installing {} \
         into the project home `{}` runs package-authored installers (arbitrary code) and the \
         group is not allow-listed; the dependency stays recorded in vibe.toml and vibe.lock; \
         fix surface: consent explicitly — `vibe {} --assume-yes` (the PROP-020 posture)",
        refused.join(", "),
        settings.path().display(),
        flags.verb
    )
}

/// The lockfile's `kind = "app"` rows; no lockfile means none.
fn app_dependencies(root: &Path) -> Result<Vec<LockedPackage>> {
    let path = root.join(Lockfile::FILENAME);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let lock = Lockfile::read(&path).with_context(|| format!("reading `{}`", path.display()))?;
    Ok(lock
        .packages
        .into_iter()
        .filter(|package| package.kind == PackageKind::App)
        .collect())
}

fn coordinate(package: &LockedPackage) -> PackageCoordinate {
    PackageCoordinate {
        group: package.group.to_string(),
        name: package.name.to_string(),
        version: package.version.to_string(),
    }
}

/// The app package as the project install materialised it into `vibedeps/`.
fn from_slot(root: &Path, package: &LockedPackage) -> Result<ResolvedPackage> {
    let slot = if package.materialization.is_in_place() {
        vibedeps::in_place_slot_abs_path(root, &package.group, &package.name)
    } else {
        vibedeps::slot_abs_path(root, &package.group, &package.name, &package.version)
    };
    let manifest = Manifest::read(slot.join(Manifest::FILENAME)).with_context(|| {
        format!(
            "reading the materialised app package `{}`; run `vibe install` to restore vibedeps/",
            slot.display()
        )
    })?;
    Ok(vibe_app::from_manifest(
        manifest,
        slot,
        coordinate(package),
    )?)
}
