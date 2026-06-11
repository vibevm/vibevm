//! `vibe registry …` — registry cache management.
//!
//! Spec: `VIBEVM-SPEC.md` §8.3 (cache layout, refresh).
//! Decentralized per-package model: PROP-002.
//!
//! `vibe registry sync` walks the lockfile and refreshes the on-disk
//! clone of every installed package. For `[[registry]]`-served entries
//! that means `git fetch` + hard-reset on the per-package clone under
//! `<cache>/<canonical-url-hash>/packages/<kind>-<name>/clone/`. For
//! `[[override]]`-served entries that means the same against the
//! `__overrides__/<kind>-<name>/clone/` subtree. Local-directory
//! registries (`--registry <path>`) and legacy v1 entries are reported
//! as skipped — there is no per-package clone to refresh for them.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

mod config;
mod publish;
mod redirect;
mod sync;
mod vendor;

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::cli::{RegistryArgs, RegistrySubcommand};
use crate::output;

pub fn run(ctx: &output::Context, args: RegistryArgs) -> Result<()> {
    match args.command {
        RegistrySubcommand::Sync(sub) => sync::run_sync(ctx, sub),
        RegistrySubcommand::Publish(sub) => publish::run_publish(ctx, sub),
        RegistrySubcommand::List(sub) => config::run_list(ctx, sub),
        RegistrySubcommand::Add(sub) => config::run_add(ctx, sub),
        RegistrySubcommand::SetMirror(sub) => config::run_set_mirror(ctx, sub),
        RegistrySubcommand::Remove(sub) => config::run_remove(ctx, sub),
        RegistrySubcommand::Vendor(sub) => vendor::run_vendor(ctx, sub),
        RegistrySubcommand::Test(sub) => config::run_test(ctx, sub),
        RegistrySubcommand::Redirect(sub) => redirect::run_redirect(ctx, sub),
        RegistrySubcommand::RedirectSync(sub) => redirect::run_redirect_sync(ctx, sub),
        RegistrySubcommand::RedirectUpdate(sub) => redirect::run_redirect_update(ctx, sub),
    }
}

#[derive(Debug, Serialize)]
struct SkippedReportEntry {
    group: String,
    name: String,
    reason: String,
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|e| anyhow!("canonicalizing `{}`: {e}", path.display()))?;
    Ok(super::init::strip_unc_public(canonical))
}
