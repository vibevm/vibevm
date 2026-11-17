//! Personal package mutations delegate to ordinary project command owners.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#USER-PROJECT-LIFECYCLE-ADMISSION");
use crate::{
    cli::{AppStepArgs, InstallArgs, UninstallArgs, UpdateArgs},
    output,
};
use anyhow::{Result, bail};
use std::path::PathBuf;

fn refuse_installer_steps(steps: &AppStepArgs) -> Result<()> {
    let choice = &steps.choice;
    if choice.configuration_input.is_some()
        || choice.parsed_configuration.is_some()
        || !choice.feature.is_empty()
        || !choice.no_feature.is_empty()
        || choice.profile.is_some()
        || !choice.set.is_empty()
        || choice.accept_license
        || steps.resume
        || steps.from_step.is_some()
        || steps.keep_work
        || steps.log.is_some()
        || steps.log_tail.is_some()
    {
        bail!("application installer controls do not apply to --user-project");
    }
    Ok(())
}

pub(crate) fn select_installed_destination(
    command: &mut crate::cli::Command,
    explicit_path: bool,
) -> Result<()> {
    if let crate::cli::Command::Update(args) = &*command
        && args.global
        && args
            .packages
            .iter()
            .any(|package| package.starts_with("mcp:"))
        && args
            .packages
            .iter()
            .any(|package| !package.starts_with("mcp:"))
    {
        bail!("request mixes MCP service and other destinations; split the commands");
    }
    match command {
        crate::cli::Command::Update(args)
            if args.global && !args.user_project && !args.packages.is_empty() && !args.all =>
        {
            args.user_project = installed_destination(&args.packages, explicit_path, false)?;
        }
        crate::cli::Command::Uninstall(args) if args.global && !args.user_project => {
            args.user_project =
                installed_destination(std::slice::from_ref(&args.package), explicit_path, true)?;
        }
        _ => {}
    }
    Ok(())
}

/// Installed requests choose one owner for the entire invocation, before mutation.
pub(crate) fn installed_destination(
    packages: &[String],
    explicit_path: bool,
    uninstall: bool,
) -> Result<bool> {
    if packages.is_empty() {
        return Ok(false);
    }
    let mut applications = None;
    let mut remaps = None;
    let mut destinations = Vec::new();
    for spelling in packages {
        if spelling.starts_with("mcp:") {
            destinations.push(false);
            continue;
        }
        if uninstall {
            if remaps.is_none() {
                remaps = Some(super::remap::RemapHome::open()?);
            }
            if remaps.as_ref().is_some_and(|home| {
                home.installed
                    .find(spelling.strip_prefix("remap:").unwrap_or(spelling))
                    .is_some()
            }) {
                destinations.push(false);
                continue;
            }
        }
        if applications.is_none() {
            applications = Some(vibe_app::Index::load(&super::app::global::settings_root()?)?);
        }
        let applications = applications
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("application inventory unavailable"))?;
        let reference = vibe_core::PackageRef::parse(spelling)?;
        let app = applications.installations.values().any(|row| {
            row.package.name == reference.name.as_str()
                && reference
                    .group
                    .as_ref()
                    .is_none_or(|group| group.as_str() == row.package.group)
        });
        if app {
            destinations.push(false);
            continue;
        }
        let selected = vibe_core::user_project::admit_root(vibe_core::settings::settings_dir())
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        match vibe_workspace::user_project::status(&selected) {
            vibe_workspace::user_project::Observation::Absent
            | vibe_workspace::user_project::Observation::Empty => {
                destinations.push(false);
                continue;
            }
            vibe_workspace::user_project::Observation::Unavailable(error) => {
                return Err(anyhow::anyhow!("{error}"));
            }
            vibe_workspace::user_project::Observation::Initialized { .. } => {}
        }
        let manifest = vibe_core::manifest::Manifest::read(selected.join("vibe.toml"))?;
        let declared = manifest.requires.packages.iter().any(|row| {
            row.name == reference.name
                && reference
                    .group
                    .as_ref()
                    .is_none_or(|group| row.group.as_ref() == Some(group))
        }) || manifest.requires.git_packages.iter().any(|row| {
            row.name == reference.name
                && reference
                    .group
                    .as_ref()
                    .is_none_or(|group| &row.group == group)
        });
        destinations.push(declared);
    }
    if destinations.iter().any(|value| *value) && destinations.iter().any(|value| !*value) {
        bail!("request spans personal and application/MCP/remap destinations; split the commands");
    }
    let personal = destinations.iter().all(|value| *value);
    if personal && explicit_path {
        bail!(
            "--path conflicts with the personal destination; omit it or choose an ordinary project operation"
        );
    }
    Ok(personal)
}

pub(crate) fn automatic_install(
    ctx: &output::Context,
    mut args: InstallArgs,
    embedded: Option<PathBuf>,
    offline: bool,
) -> Result<()> {
    use vibe_install::InstallSource;
    use vibe_orchestrator::ports::PackageSource;
    if args.packages.is_empty() {
        bail!("global install without references requires --user-project");
    }
    let source_root = vibe_core::settings::settings_dir()
        .ok_or_else(|| anyhow::anyhow!("settings root unavailable"))?;
    let manifest = vibe_core::manifest::Manifest::default();
    let resolver = super::install::build_install_resolver_deferred(
        &args,
        &manifest,
        embedded.as_deref(),
        &source_root,
        offline || args.offline,
        &[],
    )?;
    let source = vibe_package_source::RegistryPackageSource::new(
        resolver,
        Box::new(super::install::CliQualifier),
    );
    let lock = vibe_core::manifest::Lockfile::empty("routing-preflight", "0");
    let mut selected = Vec::new();
    let mut qualified_requests = Vec::new();
    let mut personal = None;
    for spelling in &args.packages {
        let reference = source.qualify(&vibe_core::PackageRef::parse(spelling)?, &lock)?;
        let mut candidate = source.resolver().select_top_level(&reference)?;
        if candidate.manifest().application.is_some() {
            let authored = if vibe_core::PackageRef::parse(spelling)?.group.is_some() {
                spelling.clone()
            } else {
                reference.to_string()
            };
            let narrowed = super::app::global::install_reference(
                ctx,
                &args,
                embedded.as_deref(),
                &authored,
                offline || args.offline,
            )?;
            if narrowed != reference.to_string() {
                candidate = source
                    .resolver()
                    .select_top_level(&vibe_core::PackageRef::parse(&narrowed)?)?;
                if candidate.manifest().application.is_none() {
                    bail!(
                        "application version policy selected a package without an application declaration"
                    );
                }
            }
        }
        if candidate
            .manifest()
            .package
            .as_ref()
            .is_some_and(|package| package.kind == vibe_core::PackageKind::Remap)
        {
            bail!("remap packages retain their dedicated owner; split this request");
        }
        let destination = candidate.manifest().application.is_none();
        if personal.is_some_and(|prior| prior != destination) {
            bail!("request spans application and personal destinations; split the commands");
        }
        personal = Some(destination);
        qualified_requests.push(reference.to_string());
        selected.push(candidate);
    }
    if personal == Some(false) {
        if selected.len() != 1 {
            bail!("global application install requires exactly one package");
        }
        let candidate = &selected[0];
        if candidate
            .manifest()
            .require_package()?
            .materialization
            .is_in_place()
        {
            bail!(
                "global application acquisition requires a cached package, not an in-place project checkout"
            );
        }
        let store = vibe_registry::store::store_root()?;
        let acquired = source.acquire_candidate_for_install(
            candidate,
            &store,
            None,
            &source_root.join("unused-app-routing-slot"),
            &ctx.progress(),
        )?;
        let vibe_registry::PackageAcquisition::Cached(cached) = acquired else {
            bail!("application routing requires an acquired package tree");
        };
        let mut resolved = vibe_app::from_manifest(
            cached.manifest.clone(),
            cached.cache_dir.clone(),
            vibe_app::PackageCoordinate {
                group: candidate.group().to_string(),
                name: candidate.name().into(),
                version: candidate.version().to_string(),
            },
        )?;
        resolved.source_evidence = Some(cached.source_evidence());
        return super::app::global::install_with_selected(
            ctx,
            args,
            embedded,
            offline,
            Some(resolved),
        );
    }
    if args.explicit_path {
        bail!("--path conflicts with the personal destination");
    }
    refuse_installer_steps(&args.steps)?;
    if args.agent.is_some()
        || args.server.is_some()
        || args.platform.is_some()
        || args.arch.is_some()
        || args.installer.is_some()
        || args.library.is_some()
        || args.no_extras
        || args.download_header_file.is_some()
        || args.version_file.asked()
        || args.hidden
    {
        bail!("application/MCP controls do not apply to personal installation");
    }
    args.path = root()?;
    admit_automatic_personal_context(&args.path)?;
    args.global = false;
    args.user_project = false;
    args.packages = qualified_requests;
    if args.plan_only {
        return super::install::run_preview_selected(
            ctx,
            args,
            embedded,
            offline,
            crate::output::seeds_default_registry(),
            super::install::SelectedRouting {
                source,
                roots: selected,
            },
        );
    }
    super::install::run_selected(
        ctx,
        args,
        embedded,
        offline,
        super::install::SelectedRouting {
            source,
            roots: selected,
        },
    )
}

/// The global source owner cannot replay a personal lock or replace its source context.
/// Until exact-selection transfer is supported, refuse rather than silently solve
/// personal dependencies with the global catalog. Explicit selection uses the ordinary owner.
fn admit_automatic_personal_context(root: &std::path::Path) -> Result<()> {
    use vibe_core::manifest::{Lockfile, Manifest};
    let manifest = Manifest::read(root.join(Manifest::FILENAME))?;
    let lock = root.join(Lockfile::FILENAME);
    let held = if lock.try_exists()? {
        !Lockfile::read(&lock)?.packages.is_empty()
    } else {
        false
    };
    if held
        || !manifest.requires.is_empty()
        || !manifest.requires_any.is_empty()
        || !manifest.registries.is_empty()
        || !manifest.source_policy.is_empty()
        || !manifest.mirrors.is_empty()
        || !manifest.overrides.is_empty()
        || manifest.override_table.is_some()
        || vibe_core::project_local_registry_path(root).try_exists()?
        || root
            .join(vibe_core::layout::current_packages_root())
            .try_exists()?
    {
        bail!(
            "automatic personal installation cannot transfer this project's declared or locked source context; use `vibe install -g --user-project` to resolve under its ordinary policy"
        );
    }
    Ok(())
}

fn root() -> Result<PathBuf> {
    super::user::initialized_root()
}

pub(crate) fn install(
    ctx: &output::Context,
    mut args: InstallArgs,
    embedded: Option<PathBuf>,
    offline: bool,
    seed: bool,
) -> Result<()> {
    if args
        .packages
        .iter()
        .any(|package| package.starts_with("mcp:"))
    {
        bail!("mcp: service routing conflicts with --user-project");
    }
    refuse_installer_steps(&args.steps)?;
    if args.agent.is_some()
        || args.server.is_some()
        || args.platform.is_some()
        || args.arch.is_some()
        || args.installer.is_some()
        || args.library.is_some()
        || args.no_extras
        || args.download_header_file.is_some()
        || args.version_file.asked()
        || args.hidden
    {
        bail!("application/MCP controls do not apply to --user-project");
    }
    args.path = root()?;
    args.global = false;
    args.user_project = false;
    if args.plan_only {
        super::install::run_preview(ctx, args, embedded, offline, seed)
    } else {
        // Direct project dependency installation does not materialize application installers.
        super::install::run_direct(ctx, args, embedded, offline)
    }
}

pub(crate) fn update(
    ctx: &output::Context,
    mut args: UpdateArgs,
    embedded: Option<PathBuf>,
    offline: bool,
) -> Result<()> {
    if args
        .packages
        .iter()
        .any(|package| package.starts_with("mcp:"))
    {
        bail!("mcp: service routing conflicts with --user-project");
    }
    refuse_installer_steps(&args.steps)?;
    if args.platform.is_some()
        || args.arch.is_some()
        || args.installer.is_some()
        || args.library.is_some()
        || args.no_extras
        || args.version_file.asked()
        || args.agent.is_some()
        || args.force
        || args.registry.is_some()
    {
        bail!(
            "application update controls do not apply to --user-project; use the project's declared or locked sources"
        );
    }
    args.path = root()?;
    args.global = false;
    args.user_project = false;
    super::update::run(ctx, args, embedded, offline)
}

pub(crate) fn uninstall(ctx: &output::Context, mut args: UninstallArgs) -> Result<()> {
    if args.package.starts_with("mcp:") {
        bail!("mcp: service routing conflicts with --user-project");
    }
    if args.agent.is_some()
        || args.force
        || args.cascade
        || args.autoremove
        || args.plan_only
        || args.keep_remap
        || args.drop_remap
    {
        bail!("application/MCP/remap removal controls do not apply to --user-project");
    }
    args.path = root()?;
    args.global = false;
    args.user_project = false;
    super::uninstall::run(ctx, args)
}
