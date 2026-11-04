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
    Index, InstallChoice, InstallRequest, OperationContext, PackageCoordinate, ResolvedPackage,
    SettingsRoot,
};
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest};
use vibe_core::{PackageKind, PackageRef};
use vibe_workspace::vibedeps;

use super::agents::{AgentSetup, report_parked};
use super::display::StepDisplay;
use super::global::{self, Observation};
use super::home;
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
    request: InstallRequest,
    /// What the materialisation shows of its children (PROP-065
    /// `#display`); the flags of the project command resolved it.
    display: &'a StepDisplay,
    consent: bool,
    offline: bool,
    root: &'a Path,
    /// `group/name` of the packages `--hidden` names (PROP-059
    /// `#hidden-installations`).
    hidden: Vec<(String, String)>,
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
    let consent = args.assume_yes || ctx.is_unattended();
    let offline = root_offline || args.offline;
    let path = args.path.clone();
    let request = global::request(choice, None, args.force, &args.steps)?;
    let display = StepDisplay::open(ctx, &args.steps);
    let hidden = if args.hidden {
        args.packages
            .iter()
            .filter_map(|spelling| vibe_app::qualified_ref(spelling).ok())
            .filter_map(|reference| {
                Some((reference.group?.to_string(), reference.name.to_string()))
            })
            .collect()
    } else {
        Vec::new()
    };
    install::run_direct(ctx, args, embedded_root, root_offline)?;
    let Some(root) = home::project_at(&path)? else {
        return Ok(());
    };
    materialise(
        ctx,
        &Materialise {
            verb: "install",
            request,
            display: &display,
            consent,
            offline,
            root: &root,
            hidden,
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
    let choice = InstallChoice {
        platform: args.platform.clone(),
        arch: args.arch.clone(),
        installer: args.installer.clone(),
    };
    let display = StepDisplay::open(ctx, &args.steps);
    let flags = Materialise {
        verb: "update",
        request: InstallRequest {
            purge_previous: args.purge_previous,
            ..global::request(choice, None, args.force, &args.steps)?
        },
        display: &display,
        consent: args.assume_yes || ctx.is_unattended(),
        offline: root_offline,
        root,
        hidden: Vec::new(),
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
    let flat = StepDisplay::flat();
    let observe = Observation::new(ctx, true, &flat)?;
    if !installed.is_empty() {
        let proxy_image = global::proxy_image()?;
        let operation = OperationContext {
            chooser: None,
            feature_chooser: None,
            ..global::operation(ctx, &settings, root_offline, &proxy_image, None, &observe)
        };
        let request = global::remove_request(&args);
        for exact in &installed {
            let removal = flat.around(ctx, || vibe_app::uninstall(&operation, exact, &request))?;
            match removal.outcome {
                Some(outcome) => outcomes.push(outcome),
                None => return super::report::render_removal(ctx, &removal),
            }
        }
    }
    project_uninstall::run(ctx, args)?;
    outcomes.extend(remove_unrecorded(
        ctx,
        root,
        &settings,
        root_offline,
        &observe,
    )?);
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

/// Bring the project home to the lockfile: install every app dependency
/// it records that the home lacks (a key already installed is skipped),
/// then remove what it no longer records. A row that only another locked
/// application requires is installed on that requirer's behalf, from its
/// lock row (PROP-065 `#app-dependencies`).
fn materialise(ctx: &output::Context, flags: &Materialise<'_>) -> Result<()> {
    let settings = SettingsRoot::project(flags.root, &global::settings_root()?);
    let locked = app_dependencies(flags.root)?;
    if locked.is_empty() && !settings.path().join("applications").exists() {
        return Ok(());
    }
    let observe = Observation::new(ctx, true, flags.display)?;
    install_pending(ctx, flags, &settings, &locked, &observe)?;
    for outcome in remove_unrecorded(ctx, flags.root, &settings, flags.offline, &observe)? {
        ctx.summary(&format!(
            "vibe {}: application `{}` — {} in the project home `{}` (vibe.lock no longer records it)",
            flags.verb,
            outcome.slot,
            outcome.message,
            settings.path().display()
        ));
    }
    Ok(())
}

/// Install the lockfile's app dependencies the project home lacks.
fn install_pending(
    ctx: &output::Context,
    flags: &Materialise<'_>,
    settings: &SettingsRoot,
    locked: &[LockedPackage],
    observe: &Observation<'_>,
) -> Result<()> {
    let index = Index::load(settings)?;
    let roots = root_dependencies(flags.root)?;
    let missing =
        |package: &&LockedPackage| !index.installations.contains_key(&coordinate(package).key());
    let pending: Vec<&LockedPackage> = locked
        .iter()
        .filter(|package| !requirement_only(package, &roots))
        .filter(missing)
        .collect();
    if pending.is_empty() {
        return Ok(());
    }
    let requirements: Vec<&LockedPackage> = locked
        .iter()
        .filter(|package| requirement_only(package, &roots))
        .filter(missing)
        .collect();
    consent(
        pending.iter().chain(&requirements).copied(),
        settings,
        flags,
    )?;
    let candidates = pending
        .iter()
        .map(|package| from_slot(flags.root, package))
        .collect::<Result<Vec<_>>>()?;
    let selected = super::configuration::select_batch(
        ctx,
        candidates,
        &flags.request.choice,
        &flags.request.parameters,
        flags.request.configuration.as_ref(),
    )?;
    if !settings.path().exists() {
        let entry = vibe_app::PROJECT_HOME.join("/");
        vibedeps::ensure_gitignored(flags.root, &entry)
            .with_context(|| format!("ignoring `{entry}` in the project's .gitignore"))?;
    }
    let proxy_image = global::proxy_image()?;
    let show = global::plan_printer(ctx);
    let agents = AgentSetup::new(ctx, None, flags.consent)?;
    // A requirement materialises from its lock row, offline-capable, never
    // from a registry (PROP-065 `#app-dependencies`).
    let resolver = |spelling: &str| -> std::result::Result<ResolvedPackage, vibe_app::AppError> {
        let requested = vibe_app::qualified_ref(spelling)?;
        let unrecorded = |reason: String| -> vibe_app::AppError {
            vibe_app::ResolveError::Registry {
                spelling: spelling.into(),
                reason,
            }
            .into()
        };
        let row = locked
            .iter()
            .find(|package| {
                requested.group.as_ref() == Some(&package.group)
                    && requested.name == package.name
                    && requested.version.matches(&package.version)
            })
            .ok_or_else(|| {
                unrecorded(
                    "vibe.lock records no row satisfying it; run `vibe install` to resolve the project again"
                        .into(),
                )
            })?;
        from_slot(flags.root, row).map_err(|error| unrecorded(format!("{error:#}")))
    };
    for (resolved, choice) in selected {
        let request = &InstallRequest {
            choice,
            hidden: flags.hidden.iter().any(|(group, name)| {
                *group == resolved.package.group && *name == resolved.package.name
            }),
            ..flags.request.clone()
        };
        let previous = Index::load(settings)?
            .installations
            .values()
            .any(|record| record.package.same_package(&resolved.package));
        let outcome = flags.display.around(ctx, || {
            let operation = OperationContext {
                agent: agents.hooks(),
                resolver: Some(&resolver),
                resolver_scope: vibe_app::ResolverScope::Project(flags.root),
                ..global::operation(
                    ctx,
                    settings,
                    flags.offline,
                    &proxy_image,
                    Some(&show),
                    observe,
                )
            };
            report_parked(
                ctx,
                if previous {
                    vibe_app::update(&operation, &resolved, request)
                } else {
                    vibe_app::install(&operation, &resolved, request)
                },
            )
        });
        let outcome = flags.display.report(ctx, outcome)?;
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

/// Remove every project-home installation the lockfile no longer records
/// (PROP-065 `#uninstall-tree`: deleting the dependency row leads to the
/// same removal in the project home, orphans included).
fn remove_unrecorded(
    ctx: &output::Context,
    root: &Path,
    settings: &SettingsRoot,
    offline: bool,
    observe: &Observation<'_>,
) -> Result<Vec<vibe_app::Outcome>> {
    if !settings.path().join("applications").exists() {
        return Ok(Vec::new());
    }
    let locked = app_dependencies(root)?;
    let proxy_image = global::proxy_image()?;
    let recorded = |package: &PackageCoordinate| {
        locked
            .iter()
            .any(|row| coordinate(row).key() == package.key())
    };
    let outcomes = {
        let _held = ctx.hold_progress();
        let operation = OperationContext {
            chooser: None,
            feature_chooser: None,
            ..global::operation(ctx, settings, offline, &proxy_image, None, observe)
        };
        vibe_app::remove_unrecorded(&operation, &recorded)?
    };
    Ok(outcomes)
}

/// A lock row that another locked application requires and the project
/// does not name itself: it is installed only through its requirer.
fn requirement_only(package: &LockedPackage, roots: &[PackageRef]) -> bool {
    !package.required_by.is_empty()
        && !roots
            .iter()
            .any(|root| root.group.as_ref() == Some(&package.group) && root.name == package.name)
}

/// Installers run package-authored code: like `vibe bin build`, only the
/// allow-listed group proceeds without `--assume-yes` (or `--unattended`).
fn consent<'p>(
    pending: impl Iterator<Item = &'p LockedPackage>,
    settings: &SettingsRoot,
    flags: &Materialise<'_>,
) -> Result<()> {
    if flags.consent {
        return Ok(());
    }
    let refused: Vec<String> = pending
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

/// The project's direct dependencies as the lockfile records them.
fn root_dependencies(root: &Path) -> Result<Vec<PackageRef>> {
    let path = root.join(Lockfile::FILENAME);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let lock = Lockfile::read(&path).with_context(|| format!("reading `{}`", path.display()))?;
    Ok(lock.meta.root_dependencies)
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
    let mut resolved = vibe_app::from_manifest(manifest, slot, coordinate(package))?;
    resolved.source.kind = vibe_wire::generated::shared::ApplicationPendingSourceKind::ProjectSlot;
    resolved.source.project = Some(
        root.to_str()
            .context("project source path is not lossless UTF-8")?
            .into(),
    );
    Ok(resolved)
}

/// Re-admit an exact saved slot through the current project lock and existing slot owner.
pub(super) fn read_saved_source(
    route: &vibe_wire::generated::shared::ApplicationPendingSource,
    coordinate: &PackageCoordinate,
) -> std::result::Result<ResolvedPackage, vibe_app::AppError> {
    let failed = |reason: String| vibe_app::ResolveError::Registry {
        spelling: coordinate.key(),
        reason,
    };
    let project = route
        .project
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| failed("retained project source has no absolute project root".into()))?;
    let root = crate::commands::resolve_project_root(&project)
        .map_err(|e| failed(format!("saved project is unavailable: {e:#}")))?;
    let rows = app_dependencies(&root)
        .map_err(|e| failed(format!("saved project lock is unavailable: {e:#}")))?;
    let row = rows
        .iter()
        .find(|row| self::coordinate(row) == *coordinate)
        .ok_or_else(|| {
            failed(
                "the current project lock no longer binds this exact application coordinate".into(),
            )
        })?;
    from_slot(&root, row)
        .map_err(|e| failed(format!("saved slot cannot be admitted: {e:#}")).into())
}

pub(super) fn resolve_saved_requirement(
    root: &Path,
    spelling: &str,
) -> std::result::Result<ResolvedPackage, vibe_app::AppError> {
    let requested = vibe_app::qualified_ref(spelling)?;
    let fail = |reason| vibe_app::ResolveError::Registry {
        spelling: spelling.into(),
        reason,
    };
    let rows =
        app_dependencies(root).map_err(|e| fail(format!("project lock cannot be read: {e:#}")))?;
    let row = rows
        .iter()
        .find(|p| {
            requested.group.as_ref() == Some(&p.group)
                && requested.name == p.name
                && requested.version.matches(&p.version)
        })
        .ok_or_else(|| fail("project lock has no matching application dependency".into()))?;
    from_slot(root, row).map_err(|e| fail(format!("project slot cannot be read: {e:#}")).into())
}
