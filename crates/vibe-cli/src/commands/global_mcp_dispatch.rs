//! The CLI boundary for project and user-scoped package mutations.
//!
//! A user-scoped MCP package keeps its own Vibe project under settings, while
//! agent configuration remains a second, explicitly managed destination.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

use super::{application, install, mcp, uninstall, update};

pub(crate) fn run_install(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    offline: bool,
) -> Result<()> {
    if !args.global {
        return install::run_direct(ctx, args, embedded_root, offline);
    }
    if !args
        .packages
        .iter()
        .any(|package| package.starts_with("mcp:"))
    {
        if args.server.is_some() {
            bail!("--server applies only to global MCP packages (`mcp:<group>/<name>`)");
        }
        return application::install(ctx, args, embedded_root, offline);
    }
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

pub(crate) fn run_uninstall(
    ctx: &output::Context,
    args: UninstallArgs,
    offline: bool,
) -> Result<()> {
    if !args.global {
        return uninstall::run(ctx, args);
    }
    if !args.package.starts_with("mcp:") {
        if args.agent.is_some() {
            bail!("--agent applies only to global MCP packages (`mcp:<group>/<name>`)");
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
            mcp::restore_global_package(&removal).with_context(|| {
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
        return update::run(ctx, args, embedded_root, offline);
    }
    let Some(package) = args
        .packages
        .iter()
        .find(|package| package.starts_with("mcp:"))
        .cloned()
    else {
        return application::update(ctx, args, embedded_root, offline);
    };
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
