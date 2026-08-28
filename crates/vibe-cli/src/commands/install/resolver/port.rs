//! The package-resolver construction port and its CLI adapter.

use std::path::Path;

use anyhow::Result;
use vibe_core::GlobalRegistryConfig;
use vibe_core::manifest::{LockedPackage, Manifest};

use crate::cli::InstallArgs;

use super::{InstallResolver, build_install_resolver};

/// Borrowed inputs at the one package-resolver construction point.
///
/// The factory is deliberately invoked only after the empty-world fast path:
/// constructing a multi-registry resolver can inspect registry state and
/// credentials, so a lifecycle which needs no packages must never pay for it.
pub(crate) struct ResolverBuild<'a> {
    pub(crate) args: &'a InstallArgs,
    pub(crate) manifest: &'a Manifest,
    pub(crate) embedded_root: Option<&'a Path>,
    pub(crate) project_root: &'a Path,
    pub(crate) global: &'a GlobalRegistryConfig,
    pub(crate) offline: bool,
    pub(crate) locked: &'a [LockedPackage],
}

/// Construction port for the resolver an install run owns.
///
/// It is synchronous because the resolver, planner and apply pipeline are
/// synchronous. The owned result outlives these borrowed construction inputs
/// through short-name qualification, planning and materialisation.
pub(crate) trait ResolverFactory: Send + Sync {
    fn build(&self, input: ResolverBuild<'_>) -> Result<InstallResolver>;
}

/// The CLI composition-root implementation. It preserves the existing
/// constructor and its exact error identity; the port adds no context of its
/// own.
pub(crate) struct CliResolverFactory;

impl ResolverFactory for CliResolverFactory {
    fn build(&self, input: ResolverBuild<'_>) -> Result<InstallResolver> {
        build_install_resolver(
            input.args,
            input.manifest,
            input.embedded_root,
            input.project_root,
            input.global,
            input.offline,
            input.locked,
        )
    }
}
