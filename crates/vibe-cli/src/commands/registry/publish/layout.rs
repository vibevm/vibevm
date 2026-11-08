//! `vibe registry publish` into a `monorepo` or `localdir` registry
//! (PROP-002 `#registry-type`), and the per-package repository url of a
//! generic `git` registry.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::manifest::{Manifest, RegistrySection};
use vibe_publish::layout_publish::{LayoutAction, LayoutPublishConfig, LayoutTarget, publish};
use vibe_wire::generated::registry_layout_publish_report::{
    LayoutAction as WireAction, LayoutMode, RegistryLayoutPublishReport,
};

use crate::cli::RegistryPublishArgs;
use crate::commands::registry::observed_stage;
use crate::output;

pub(super) fn run(
    ctx: &output::Context,
    args: &RegistryPublishArgs,
    registry: &RegistrySection,
    target: LayoutTarget,
    source_dir: PathBuf,
) -> Result<()> {
    if args.index_checkout.is_some() || args.push_index {
        bail!(
            "--index-checkout / --push-index record a per-package repository and tag; registry \
             `{}` is a `{}` registry, whose packages carry neither",
            registry.name,
            registry.effective_type()
        );
    }
    let (mode, shown) = match &target {
        LayoutTarget::Monorepo { url, branch } => (
            LayoutMode::Monorepo,
            format!(
                "{} (branch `{branch}`)",
                vibe_publish::git_publish::redact_credentials(url)
            ),
        ),
        LayoutTarget::Localdir { root } => (LayoutMode::Localdir, root.display().to_string()),
    };
    ctx.heading(&format!(
        "Publishing {} → {} registry `{}` at {shown}{}",
        source_dir.display(),
        match mode {
            LayoutMode::Monorepo => "monorepo",
            LayoutMode::Localdir => "localdir",
        },
        registry.name,
        if args.dry_run { " [dry-run]" } else { "" },
    ));
    let config = LayoutPublishConfig {
        source_dir,
        target,
        replace: args.replace,
        dry_run: args.dry_run,
    };
    let outcome = observed_stage(ctx, "Publishing into the registry layout", || {
        publish(&config, ctx.progress().cancellation()).map_err(anyhow::Error::from)
    })?;

    if ctx.is_json() {
        return ctx.emit_json(&RegistryLayoutPublishReport {
            ok: true,
            command: "registry:publish".to_owned(),
            mode,
            registry: registry.name.clone(),
            target: shown,
            package: format!("{}/{}", outcome.group, outcome.name),
            version: outcome.version.to_string(),
            package_path: outcome.package_path.clone(),
            action: match outcome.action {
                LayoutAction::Created => WireAction::Created,
                LayoutAction::Replaced => WireAction::Replaced,
                LayoutAction::Unchanged => WireAction::Unchanged,
            },
            commit: outcome.commit.clone(),
            retried: outcome.retried,
            content_hash: outcome.content_hash.clone(),
            dry_run: outcome.dry_run,
        });
    }
    if outcome.retried {
        ctx.step("The branch moved during the publish; refetched and retried once.");
    }
    let what = match (outcome.dry_run, outcome.action) {
        (_, LayoutAction::Unchanged) => "already holds identical content; nothing written",
        (true, LayoutAction::Created) => "would be created",
        (true, LayoutAction::Replaced) => "would be replaced",
        (false, LayoutAction::Created) => "created",
        (false, LayoutAction::Replaced) => "replaced",
    };
    let commit = outcome
        .commit
        .as_deref()
        .map(|commit| format!(" (commit {commit})"))
        .unwrap_or_default();
    ctx.summary(&format!(
        "\nvibe registry publish: `{}` {what} in `{}`{commit}.",
        outcome.package_path, registry.name
    ));
    Ok(())
}

/// The existing repository a `git`-type registry keeps this package in:
/// `<registry url>/<repo name>.git` under the registry's naming convention.
pub(super) fn package_repo_url(registry: &RegistrySection, source_dir: &Path) -> Result<String> {
    let manifest_path = source_dir.join(Manifest::FILENAME);
    let manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;
    let package = manifest
        .package
        .ok_or_else(|| anyhow!("`{}` has no [package] table", manifest_path.display()))?;
    let repo =
        registry
            .naming
            .repo_name(Some(package.kind), &package.group, package.name.as_str())?;
    Ok(format!("{}/{repo}.git", registry.url.trim_end_matches('/')))
}
