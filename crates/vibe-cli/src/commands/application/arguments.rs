//! Global application command flag validation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use super::source::qualified_ref;
use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use anyhow::{Result, bail};
use std::path::Path;

pub(super) fn validate_install_args(args: &InstallArgs) -> Result<()> {
    if args.packages.len() != 1 {
        bail!("global install requires exactly one fully qualified application package");
    }
    qualified_ref(&args.packages[0])?;
    if (!args.local_source && args.path != Path::new("."))
        || args.agent.is_some()
        || args.language.is_some()
        || !args.features.is_empty()
        || args.no_default_features
        || args.all_features
        || args.exact
        || args.auth_required
        || args.solver.is_some()
        || args.prefer_embedded
        || args.no_prefer_embedded
        || args.no_default_registry
        || args.embedded_short_circuit
        || args.prefer_local
        || args.no_prefer_local
        || args.git.is_some()
        || args.tag.is_some()
        || args.branch.is_some()
        || args.rev.is_some()
        || args.git_auth.is_some()
        || args.git_token_env.is_some()
        || args.trace_compile
    {
        bail!("global install received project-only package flags");
    }
    Ok(())
}

pub(super) fn validate_update_args(args: &UpdateArgs) -> Result<()> {
    if args.packages.len() != 1 || args.all {
        bail!("global update requires exactly one fully qualified application package");
    }
    qualified_ref(&args.packages[0])?;
    if (!args.local_source && args.path != Path::new("."))
        || args.exact
        || args.auth_required
        || args.trace_compile
    {
        bail!("global update received project-only package flags");
    }
    Ok(())
}

pub(super) fn validate_uninstall_args(args: &UninstallArgs) -> Result<()> {
    qualified_ref(&args.package)?;
    if args.path != Path::new(".") {
        bail!("global uninstall received project-only --path");
    }
    Ok(())
}
