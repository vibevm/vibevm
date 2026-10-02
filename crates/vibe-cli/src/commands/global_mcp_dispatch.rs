//! The CLI boundary for project and user-scoped package mutations.
//!
//! A user-scoped MCP package keeps its own Vibe project under settings, while
//! agent configuration remains a second, explicitly managed destination.

use std::path::PathBuf;

use anyhow::{Context, Result};

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
        return application::install(ctx, args, embedded_root, offline);
    }
    let consent = args.assume_yes || ctx.is_unattended();
    let installed = install::run_global_mcp(ctx, args, embedded_root, offline)?;
    mcp::register_global_package(
        ctx,
        &installed.project_root,
        &installed.package,
        &installed.agents,
        consent,
    )
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
        return application::uninstall(ctx, args, offline);
    }

    let mut removal = mcp::preflight_unregister_global_package(&args.package)?;
    mcp::unregister_global_package(&mut removal)?;
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
    mcp::preflight_refresh_global_package(&package)?;
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
