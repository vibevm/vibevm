//! `vibe registry remove registry|mirror` — drop a `[[registry]]` or
//! `[[mirror]]` block, refusing removals that would orphan mirrors.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::manifest::{Manifest, MirrorSection};

use crate::cli::{
    RegistryRemoveArgs, RegistryRemoveMirrorArgs, RegistryRemoveRegistryArgs, RegistryRemoveTarget,
};
use crate::commands::registry::resolve_project_root;
use crate::output;

#[derive(Debug, Serialize)]
struct RemoveReport {
    ok: bool,
    command: &'static str,
    /// `"registry"` or `"mirror"` — what was removed.
    target: &'static str,
    /// For `target == "registry"`, the name. For `target == "mirror"`,
    /// `<of>:<url>`.
    identity: String,
    total_registries: usize,
    total_mirrors: usize,
}

pub(in crate::commands::registry) fn run_remove(
    ctx: &output::Context,
    args: RegistryRemoveArgs,
) -> Result<()> {
    match args.target {
        RegistryRemoveTarget::Registry(sub) => run_remove_registry(ctx, sub),
        RegistryRemoveTarget::Mirror(sub) => run_remove_mirror(ctx, sub),
    }
}

fn run_remove_registry(ctx: &output::Context, args: RegistryRemoveRegistryArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    if manifest.registry_by_name(&args.name).is_none() {
        let known: Vec<&str> = manifest
            .registries
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        let known_text = if known.is_empty() {
            "(none configured)".to_string()
        } else {
            known.join(", ")
        };
        bail!(
            "no `[[registry]]` named `{}` in `{}`. Known: {}.",
            args.name,
            manifest_path.display(),
            known_text
        );
    }

    // Refuse to orphan named mirrors. A `[[mirror]] of = "<name>"`
    // referring to a now-removed registry would never be consulted —
    // the manifest would still be parseable but operationally nonsense.
    // Wildcard `of = "*"` mirrors are fine; they apply to whatever
    // registries exist.
    let orphaned: Vec<&MirrorSection> = manifest
        .mirrors
        .iter()
        .filter(|m| m.of == args.name)
        .collect();
    if !orphaned.is_empty() {
        let urls: Vec<String> = orphaned.iter().map(|m| m.url.clone()).collect();
        bail!(
            "cannot remove `[[registry]]` `{}`: {} `[[mirror]]` block(s) target it ({}). Remove those mirrors first with `vibe registry remove mirror <of> <url>`.",
            args.name,
            urls.len(),
            urls.join(", ")
        );
    }

    manifest.registries.retain(|r| r.name != args.name);

    manifest
        .write(&manifest_path)
        .with_context(|| format!("writing `{}`", manifest_path.display()))?;

    if ctx.is_json() {
        ctx.emit_json(&RemoveReport {
            ok: true,
            command: "registry:remove",
            target: "registry",
            identity: args.name.clone(),
            total_registries: manifest.registries.len(),
            total_mirrors: manifest.mirrors.len(),
        })?;
        return Ok(());
    }

    ctx.step(&format!("Removed `[[registry]]` `{}`", args.name));
    ctx.summary(&format!(
        "\nvibe registry remove: {} registr{} remain.",
        manifest.registries.len(),
        if manifest.registries.len() == 1 {
            "y"
        } else {
            "ies"
        }
    ));
    Ok(())
}

fn run_remove_mirror(ctx: &output::Context, args: RegistryRemoveMirrorArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let manifest_path = project_root.join(Manifest::FILENAME);
    if !manifest_path.exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            project_root.display()
        );
    }
    let mut manifest = Manifest::read(&manifest_path)
        .with_context(|| format!("reading `{}`", manifest_path.display()))?;

    let before = manifest.mirrors.len();
    manifest
        .mirrors
        .retain(|m| !(m.of == args.of && m.url == args.url));
    let after = manifest.mirrors.len();

    if before == after {
        bail!(
            "no `[[mirror]]` in `{}` matches of=`{}` url=`{}`. Use `vibe registry list` to see what's configured.",
            manifest_path.display(),
            args.of,
            args.url
        );
    }
    if before - after > 1 {
        // Shouldn't happen if `set-mirror` enforces uniqueness on
        // (of, url), but if a hand-edited manifest carries duplicates
        // we drop them all and tell the user.
        eprintln!(
            "warning: removed {} `[[mirror]]` blocks matching of=`{}` url=`{}` (duplicates were present)",
            before - after,
            args.of,
            args.url
        );
    }

    manifest
        .write(&manifest_path)
        .with_context(|| format!("writing `{}`", manifest_path.display()))?;

    if ctx.is_json() {
        ctx.emit_json(&RemoveReport {
            ok: true,
            command: "registry:remove",
            target: "mirror",
            identity: format!("{}:{}", args.of, args.url),
            total_registries: manifest.registries.len(),
            total_mirrors: manifest.mirrors.len(),
        })?;
        return Ok(());
    }

    ctx.step(&format!(
        "Removed `[[mirror]]` of=`{}` url=`{}`",
        args.of, args.url
    ));
    ctx.summary(&format!(
        "\nvibe registry remove: {} mirror{} remain.",
        manifest.mirrors.len(),
        if manifest.mirrors.len() == 1 { "" } else { "s" }
    ));
    Ok(())
}
