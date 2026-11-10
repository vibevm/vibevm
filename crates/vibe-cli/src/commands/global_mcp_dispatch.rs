//! The CLI boundary for project and user-scoped package mutations.
//!
//! A user-scoped MCP package keeps its own Vibe project under settings, while
//! agent configuration remains a second, explicitly managed destination.
//! Without `-g`, app dependencies of a project are materialised into its
//! project home; `update` and `uninstall` outside a project act on the user
//! home's applications (PROP-059 `#project-scope`).

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

use super::app::global as application;
use super::app::home as application_home;
use super::app::project as application_project;
use super::{install, mcp, remap, uninstall, update};

pub(crate) fn run_install(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    offline: bool,
    seed_default_registry: bool,
) -> Result<()> {
    if !args.global {
        refuse_configuration_input(&args.steps.choice, "workspace package acquisition")?;
        if args.plan_only {
            return install::run_preview(ctx, args, embedded_root, offline, seed_default_registry);
        }
        // PROP-066 `#layers-packages`: a project's `[requires.remaps]` is
        // resolved and pinned in the lock like any other requirement, and
        // recorded in this user's home as an offer — so every clone of
        // the project sees the same list and each machine still decides.
        // It runs after the install because the pin belongs beside the
        // rest of the resolution, and an offer that nobody has accepted
        // yet changes nothing about the fetches this run performed.
        let path = args.path.clone();
        let projected = args.clone();
        // A project install publishes an opted-in node's prefix along with
        // its other boot artifacts, and whoever builds the artifacts says
        // what it did and where (PROP-076
        // `#SYS-OPERATIONS-PATH-AND-FLAGS`).
        crate::commands::prompt::reporting_the_prompt(ctx, &path, || {
            application_project::install(ctx, args, embedded_root, offline)
        })?;
        return pin_project_remaps(ctx, &projected, &path, offline);
    }
    if !args
        .packages
        .iter()
        .any(|package| package.starts_with("mcp:"))
    {
        if args.server.is_some() {
            bail!("--server applies only to global MCP packages (`mcp:<group>/<name>`)");
        }
        // PROP-066 `#package`: a remap package goes into the user home as
        // an **offer**, not into a library as an application. The kind is
        // read from the manifest the registry would serve, so the operator
        // writes `vibe install -g org.corp/intranet` without knowing it;
        // a probe that cannot answer falls through, so the application
        // path keeps producing its own errors exactly as before.
        if let Some(refs) = remap::claims_install(&args, offline) {
            refuse_configuration_input(&args.steps.choice, "remap package acquisition")?;
            return remap::install_packages(ctx, &args, &refs, offline);
        }
        return application::install(ctx, args, embedded_root, offline);
    }
    refuse_configuration_input(&args.steps.choice, "MCP package acquisition")?;
    let overall = ctx.progress().task("Installing global MCP package");
    let scoped = ctx.with_progress_scope(overall.progress());
    let result = install_mcp(&scoped, args, embedded_root, offline);
    if result.is_ok() {
        overall.finish();
    } else {
        overall.fail("global MCP installation failed");
    }
    result
}

fn install_mcp(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    offline: bool,
) -> Result<()> {
    let consent = args.assume_yes || ctx.is_unattended();
    let servers = args.server.clone();
    let installed = install::run_global_mcp(ctx, args, embedded_root, offline)?;
    let registration = ctx
        .progress()
        .task("Registering MCP package in selected agents");
    let result = mcp::register_global_package(
        ctx,
        &installed.project_root,
        &installed.package,
        &installed.agents,
        servers.as_deref(),
        consent,
    );
    if result.is_ok() {
        registration.finish();
    } else {
        registration.fail("MCP agent registration failed");
    }
    result
}

/// Resolve and pin the project's `[requires.remaps]`
/// (`PROP-066#layers-packages`).
///
/// A project with no such table clears the lock's `[[remap]]` rows and
/// does nothing else, so the table and the lock cannot disagree.
fn pin_project_remaps(
    ctx: &output::Context,
    args: &InstallArgs,
    path: &std::path::Path,
    offline: bool,
) -> Result<()> {
    let selected = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let workspace = remap::packages::owning_workspace(&selected)?;
    let lock_path = workspace.root.join(vibe_core::manifest::Lockfile::FILENAME);
    let manifest_path = selected.join(vibe_core::manifest::Manifest::FILENAME);
    if !lock_path.exists() || !manifest_path.exists() {
        return Ok(());
    }
    let manifest = vibe_core::manifest::Manifest::read(&manifest_path)?;
    let publication = vibe_install::LockPublication::capture(&workspace.root)?;
    let mut lockfile = publication.lockfile().clone();
    let workspace_requirements = std::iter::once(&workspace.root_manifest)
        .chain(workspace.members.iter().map(|member| &member.manifest))
        .flat_map(|manifest| manifest.requires.remaps.iter().cloned())
        .collect::<Vec<_>>();
    let before = lockfile.remaps.clone();
    let count = remap::adopt_project_requirements(
        args,
        &manifest,
        &mut lockfile,
        offline,
        &workspace_requirements,
    )?;
    if lockfile.remaps == before {
        return Ok(());
    }
    publication.publish(&lockfile)?;
    if count > 0 && !ctx.is_json() && !ctx.is_quiet() {
        ctx.step(&format!(
            "{count} remap package(s) required by this project are offered in your home —              `vibe remap list` shows their rows"
        ));
    }
    Ok(())
}

pub(crate) fn run_uninstall(
    ctx: &output::Context,
    args: UninstallArgs,
    offline: bool,
) -> Result<()> {
    if !args.global {
        if let Some(root) = application_home::project_at(&args.path)? {
            return application_project::uninstall(ctx, args, offline, &root);
        }
        if args.package.starts_with("mcp:") {
            return uninstall::run(ctx, args);
        }
        if remap::claims_uninstall(&args.package) {
            return remap::run_uninstall(ctx, &args);
        }
        return application::uninstall(ctx, args, offline);
    }
    if !args.package.starts_with("mcp:") {
        if args.agent.is_some() {
            bail!("--agent applies only to global MCP packages (`mcp:<group>/<name>`)");
        }
        // PROP-066 `#uninstall`: removing a remap package asks about the
        // rows that would stop applying before anything is removed.
        if remap::claims_uninstall(&args.package) {
            return remap::run_uninstall(ctx, &args);
        }
        return application::uninstall(ctx, args, offline);
    }

    install::preflight_global_mcp_uninstall(&args)?;
    let owned = mcp::global_package_agents(ctx, &args.package)?;
    let agents = if let Some(explicit) = args.agent.as_deref() {
        let parsed = super::global_mcp_agents::parse_explicit_filter(explicit)?;
        if explicit.trim() != "all" && parsed.iter().any(|agent| !owned.contains(agent)) {
            bail!(
                "agent `{explicit}` has no owned registration for `{}`",
                args.package
            );
        }
        owned
            .iter()
            .copied()
            .filter(|agent| parsed.contains(agent))
            .collect()
    } else if !args.assume_yes && super::global_mcp_agents::interactive(ctx) && !owned.is_empty() {
        super::global_mcp_agents::prompt_uninstall(
            ctx,
            &mcp::global_package_registration_statuses(ctx, &args.package)?,
        )?
    } else {
        owned
    };
    let mut removal = mcp::preflight_unregister_global_package(ctx, &args.package, &agents)?;
    mcp::unregister_global_package(&mut removal)?;
    if !mcp::removes_global_package(&removal) {
        return mcp::report_global_uninstall(ctx, &removal);
    }
    match install::run_global_mcp_uninstall(ctx, args) {
        Ok(()) => mcp::report_global_uninstall(ctx, &removal),
        Err(error) => {
            mcp::restore_global_package(&mut removal).with_context(|| {
                format!(
                    "global MCP package removal failed: {error}; restoring agent registrations also failed"
                )
            })?;
            Err(error)
        }
    }
}

pub(crate) fn run_update(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    offline: bool,
) -> Result<()> {
    if !args.global {
        if let Some(root) = application_home::project_at(&args.path)? {
            refuse_configuration_input(&args.steps.choice, "workspace package update")?;
            return application_project::update(ctx, args, embedded_root, offline, &root);
        }
        if args.all
            || args.packages.is_empty()
            || args
                .packages
                .iter()
                .any(|package| package.starts_with("mcp:"))
        {
            refuse_configuration_input(&args.steps.choice, "workspace package update")?;
            return update::run(ctx, args, embedded_root, offline);
        }
        return application::update(ctx, args, embedded_root, offline);
    }
    let Some(package) = args
        .packages
        .iter()
        .find(|package| package.starts_with("mcp:"))
        .cloned()
    else {
        return application::update(ctx, args, embedded_root, offline);
    };
    refuse_configuration_input(&args.steps.choice, "MCP package update")?;
    let consent = args.assume_yes || ctx.is_unattended();
    mcp::preflight_refresh_global_package(ctx, &package)?;
    let updated = install::run_global_mcp_update(ctx, args, embedded_root, offline)?;
    mcp::refresh_global_package(ctx, &updated.project_root, &updated.package, consent).with_context(
        || {
            format!(
                "global MCP package `{}` was updated, but agent registration refresh failed; \
                 re-run `vibe mcp install mcp:{}` with --scope user for the affected agent",
                updated.package,
                updated.package.trim_start_matches("mcp:")
            )
        },
    )
}

#[cfg(test)]
mod tests;

fn refuse_configuration_input(answers: &crate::cli::AppChoiceArgs, scope: &str) -> Result<()> {
    if answers.configuration_input.is_some() || answers.parsed_configuration.is_some() {
        return Err(vibe_app::AppError::from(vibe_app::configuration::ConfigurationError::Invalid{option:scope.into(),reason:"--configuration-input belongs to a selected app installer; this route does not evaluate that submission".into()}).into());
    }
    Ok(())
}
