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
//! `vibe registry sync -g` (or the plain form outside a project) walks the
//! user home's registry cache instead (`sync_home`).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

mod alias;
mod config;
mod index_step;
pub(crate) mod layout_root;
mod mirror;
mod publish;
mod redirect;
mod sync;
mod sync_home;
mod vendor;

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::cli::{RegistryArgs, RegistrySubcommand};
use crate::output;

/// `root_offline` is the root `--offline` flag; the commands that reach
/// the network resolve the rest of the ladder themselves.
pub fn run(ctx: &output::Context, args: RegistryArgs, root_offline: bool) -> Result<()> {
    match args.command {
        RegistrySubcommand::Sync(sub) => sync::run_sync(ctx, sub, root_offline),
        RegistrySubcommand::Publish(sub) => publish::run_publish(ctx, sub),
        RegistrySubcommand::List(sub) => config::run_list(ctx, sub),
        RegistrySubcommand::Add(sub) => config::run_add(ctx, sub),
        RegistrySubcommand::SetMirror(sub) => config::run_set_mirror(ctx, sub),
        RegistrySubcommand::Remove(sub) => config::run_remove(ctx, sub),
        RegistrySubcommand::Mirror(sub) => mirror::run_mirror(ctx, sub, root_offline),
        RegistrySubcommand::Alias(sub) => alias::run_alias(ctx, sub, root_offline),
        RegistrySubcommand::Vendor(sub) => vendor::run_vendor(ctx, sub),
        RegistrySubcommand::Test(sub) => config::run_test(ctx, sub),
        RegistrySubcommand::Redirect(sub) => redirect::run_redirect(ctx, sub),
        RegistrySubcommand::RedirectSync(sub) => redirect::run_redirect_sync(ctx, sub),
        RegistrySubcommand::RedirectUpdate(sub) => redirect::run_redirect_update(ctx, sub),
    }
}

pub(super) fn observed_stage<T>(
    ctx: &output::Context,
    label: impl Into<String>,
    run: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let task = ctx.progress().task(label);
    let result = run();
    if result.is_ok() {
        task.finish();
    } else {
        task.fail("operation failed");
    }
    result
}

#[derive(Debug, Serialize)]
struct SkippedReportEntry {
    group: String,
    name: String,
    reason: String,
}

/// The effective `[[registry]]` / `[[mirror]]` / `[[override]]` rows of a
/// project (PROP-002 `#registry-layers`): `.vibe/registry.local.toml`, then
/// `vibe.toml`, then `~/.vibe/registry.toml` (with the applied site profile
/// beneath it) — what resolution and `vibe registry publish` read.
pub(crate) fn effective_registries(
    project_root: &Path,
    manifest: &vibe_core::manifest::Manifest,
) -> Result<vibe_core::EffectiveRegistryConfig> {
    use anyhow::Context as _;
    let user =
        vibe_core::GlobalRegistryConfig::load().context("loading `~/.vibe/registry.toml`")?;
    let local = vibe_core::GlobalRegistryConfig::load_project_local(project_root)?;
    Ok(vibe_core::RegistryLayers {
        local: Some(&local),
        project: Some(manifest),
        user: &user,
        site: None,
    }
    .merge())
}

/// The effective `[source_policy]` (PROP-073 `#SOURCE-POLICY`, PROP-019
/// `#site-profile`): the applied site profile beneath the user's own
/// `~/.vibe/registry.toml`, with the project's `vibe.toml` above both. This
/// is the policy resolution itself applies, so a listing built on it marks
/// exactly the registries resolution skips — and nothing else.
pub(crate) fn effective_source_policy(
    manifest: Option<&vibe_core::manifest::Manifest>,
) -> Result<vibe_core::manifest::EffectiveSourcePolicy> {
    use anyhow::Context as _;
    let site = vibe_core::site_profile::installed()
        .context("reading the applied site profile")?
        .map(|site| site.profile.source_policy.clone())
        .unwrap_or_default();
    let user = vibe_core::GlobalRegistryConfig::load()
        .context("loading `~/.vibe/registry.toml`")?
        .source_policy
        .over(&site);
    let project = manifest
        .map(|manifest| manifest.source_policy.clone())
        .unwrap_or_default();
    let empty = vibe_core::manifest::SourcePolicyDecl::default();
    vibe_core::manifest::EffectiveSourcePolicy::merge(&user, &project, &empty)
        .context("merging the effective source policy")
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|e| anyhow!("canonicalizing `{}`: {e}", path.display()))?;
    Ok(super::init::strip_unc_public(canonical))
}
