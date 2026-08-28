//! The CLI's package-source adapter — the surface's argument grammar
//! projected onto the shared composition in `vibe-package-source`.
//!
//! The resolver itself (the `InstallResolver` local / multi-registry /
//! embedded dispatch, its R-001 registry cells, and the neutral builder)
//! moved to that crate in R7.4 A15a so the CLI and the later hosted MCP
//! adapter construct the same algorithmic source. What stays HERE is
//! exactly the CLI's own share: the `InstallArgs` →
//! [`PackageSourceOptions`] projection (a pure field copy — no
//! normalisation, no validation reordering), the historical
//! [`build_install_resolver`] call shape this surface's sibling commands
//! (cache / reinstall / update) already speak, and the `--git`
//! source-flag cell plus the composition-root port adapters below.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use anyhow::Result;
use vibe_core::GlobalRegistryConfig;
use vibe_core::manifest::{LockedPackage, Manifest};
use vibe_package_source::PackageSourceOptions;

use crate::cli::InstallArgs;

mod git_source_flag;
mod port;
pub(crate) use port::{CliGitSourceMutation, CliPackageSourceFactory};
// The shared resolver type, re-exported under this module's historical
// path for the sibling commands that name it (`short_name`, cache,
// reinstall, update).
pub(crate) use vibe_package_source::InstallResolver;

/// Project this surface's `InstallArgs` onto the neutral options the
/// shared builder reads — exactly the ten fields that builder consumes, as
/// a pure copy. Everything else (pkgrefs, features, pinning, trace) has
/// its own consumer elsewhere and never crosses here.
pub(crate) fn package_source_options(args: &InstallArgs) -> PackageSourceOptions {
    PackageSourceOptions {
        registry: args.registry.clone(),
        solver: args.solver.clone(),
        auth_required: args.auth_required,
        prefer_embedded: args.prefer_embedded,
        no_prefer_embedded: args.no_prefer_embedded,
        no_default_registry: args.no_default_registry,
        embedded_short_circuit: args.embedded_short_circuit,
        prefer_local: args.prefer_local,
        no_prefer_local: args.no_prefer_local,
        has_git_source_flag: args.git.is_some(),
    }
}

/// The CLI composition-root resolver constructor — the historical call
/// shape (`&InstallArgs`) preserved for this surface's sibling commands,
/// now a projection plus the shared lower builder. The constructor and its
/// exact error identity are unchanged; the port adds no context of its
/// own.
pub(crate) fn build_install_resolver(
    args: &InstallArgs,
    manifest: &Manifest,
    embedded_root: Option<&Path>,
    project_root: &Path,
    global: &GlobalRegistryConfig,
    offline: bool,
    locked: &[LockedPackage],
) -> Result<InstallResolver> {
    vibe_package_source::build_install_resolver(
        &package_source_options(args),
        manifest,
        embedded_root,
        project_root,
        global,
        offline,
        locked,
    )
}

#[cfg(test)]
#[path = "flag_tests.rs"]
mod flag_tests;
