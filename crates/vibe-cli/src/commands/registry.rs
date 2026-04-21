//! `vibe registry …` — registry cache management.
//!
//! Spec: `VIBEVM-SPEC.md` §8.3 (cache layout, refresh),
//! [`spec://vibevm/modules/vibe-registry/PROP-001`] for the backend
//! decision.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use vibe_core::manifest::ProjectManifest;
use vibe_registry::GitRegistry;

use crate::cli::{RegistryArgs, RegistrySubcommand, RegistrySyncArgs};
use crate::output;

pub fn run(ctx: &output::Context, args: RegistryArgs) -> Result<()> {
    match args.command {
        RegistrySubcommand::Sync(sub) => run_sync(ctx, sub),
    }
}

#[derive(Debug, Serialize)]
struct SyncReport {
    ok: bool,
    command: &'static str,
    url: String,
    r#ref: String,
    cache_dir: String,
}

fn run_sync(ctx: &output::Context, args: RegistrySyncArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(ProjectManifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let manifest = ProjectManifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let Some(reg) = manifest.registry else {
        bail!(
            "no `[registry]` section in `{}`. `vibe registry sync` only refreshes configured git registries.",
            manifest_path.display()
        );
    };

    if reg.url.starts_with("file://") {
        ctx.summary(&format!(
            "vibe registry sync: `{}` is a local directory, nothing to sync",
            reg.url
        ));
        return Ok(());
    }

    ctx.heading(&format!("Syncing registry {}#{}", reg.url, reg.r#ref));
    let git = GitRegistry::open(&reg.url, &reg.r#ref)
        .with_context(|| format!("opening git registry `{}`", reg.url))?;
    git.sync()
        .with_context(|| format!("syncing registry `{}`", reg.url))?;

    if ctx.is_json() {
        let report = SyncReport {
            ok: true,
            command: "registry:sync",
            url: reg.url.clone(),
            r#ref: reg.r#ref.clone(),
            cache_dir: git.cache_dir().to_string_lossy().to_string(),
        };
        ctx.emit_json(&report)?;
        return Ok(());
    }

    ctx.summary(&format!(
        "\nRegistry cache up to date at {}.",
        git.cache_dir().display()
    ));
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|e| anyhow!("canonicalizing `{}`: {e}", path.display()))?;
    Ok(super::init::strip_unc_public(canonical))
}
