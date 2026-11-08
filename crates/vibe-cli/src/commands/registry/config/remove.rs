//! `vibe registry remove registry|mirror` — drop a `[[registry]]` or
//! `[[mirror]]` block, refusing removals that would orphan mirrors.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use anyhow::{Result, bail};
use serde::Serialize;
use vibe_core::RegistryScope;
use vibe_core::manifest::MirrorSection;

use crate::cli::{
    RegistryRemoveArgs, RegistryRemoveMirrorArgs, RegistryRemoveRegistryArgs, RegistryRemoveTarget,
};
use crate::output;

use super::scope::{ScopeFile, parse_scope};

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
    let scope = parse_scope(args.scope.as_deref())?.unwrap_or(RegistryScope::Project);
    let mut file = ScopeFile::open(scope, &args.path)?;
    let manifest_path = file.path().to_path_buf();

    if !file.registries().iter().any(|r| r.name == args.name) {
        let known: Vec<&str> = file.registries().iter().map(|r| r.name.as_str()).collect();
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
    let orphaned: Vec<&MirrorSection> = file
        .mirrors()
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

    file.registries_mut().retain(|r| r.name != args.name);
    file.save()?;
    let remaining = file.registries().len();

    if ctx.is_json() {
        ctx.emit_json(&RemoveReport {
            ok: true,
            command: "registry:remove",
            target: "registry",
            identity: args.name.clone(),
            total_registries: remaining,
            total_mirrors: file.mirrors().len(),
        })?;
        return Ok(());
    }

    ctx.step(&format!(
        "Removed `[[registry]]` `{}` from `{}`",
        args.name,
        manifest_path.display()
    ));
    ctx.summary(&format!(
        "\nvibe registry remove: {} registr{} remain.",
        remaining,
        if remaining == 1 { "y" } else { "ies" }
    ));
    Ok(())
}

fn run_remove_mirror(ctx: &output::Context, args: RegistryRemoveMirrorArgs) -> Result<()> {
    let scope = parse_scope(args.scope.as_deref())?.unwrap_or(RegistryScope::Project);
    let mut file = ScopeFile::open(scope, &args.path)?;
    let manifest_path = file.path().to_path_buf();

    let before = file.mirrors().len();
    file.mirrors_mut()
        .retain(|m| !(m.of == args.of && m.url == args.url));
    let after = file.mirrors().len();

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
        ctx.diagnostic(&format!(
            "removed {} `[[mirror]]` blocks matching of=`{}` url=`{}` (duplicates were present)",
            before - after,
            args.of,
            args.url
        ));
    }

    file.save()?;

    if ctx.is_json() {
        ctx.emit_json(&RemoveReport {
            ok: true,
            command: "registry:remove",
            target: "mirror",
            identity: format!("{}:{}", args.of, args.url),
            total_registries: file.registries().len(),
            total_mirrors: file.mirrors().len(),
        })?;
        return Ok(());
    }

    ctx.step(&format!(
        "Removed `[[mirror]]` of=`{}` url=`{}`",
        args.of, args.url
    ));
    ctx.summary(&format!(
        "\nvibe registry remove: {} mirror{} remain.",
        file.mirrors().len(),
        if file.mirrors().len() == 1 { "" } else { "s" }
    ));
    Ok(())
}
