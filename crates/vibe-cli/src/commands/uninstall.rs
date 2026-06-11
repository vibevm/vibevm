//! `vibe uninstall <group>/<name>` — remove an installed package.
//!
//! In the PROP-009 loading model, uninstalling a package removes its
//! `vibedeps/` slot, drops its lockfile entry and its `[requires]`
//! declaration, and regenerates every node's boot artifacts so the
//! package no longer appears in the computed boot sequence.
//!
//! Spec: spec://vibevm/modules/vibe-workspace/PROP-009-loading-model.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};

use crate::exit_code::InstallError;
use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{Group, PackageRef};
use vibe_workspace::Workspace;
use vibe_workspace::install::regenerate_boot;
use vibe_workspace::vibedeps;

use crate::cli::UninstallArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: UninstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let mut manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_lockfile(&workspace.root)?;

    let pkgref =
        PackageRef::parse(&args.package).with_context(|| format!("parsing `{}`", args.package))?;
    // Identity is `(group, name)` — `vibe uninstall` needs the qualified
    // form (PROP-008 §2.4).
    let group = require_group(&pkgref)?;

    // The materialised slot is keyed by `(kind, name, version)`; the
    // resolved version and the package `kind` (metadata) are both read
    // from the lockfile entry.
    let locked = lockfile.find(group, &pkgref.name).ok_or_else(|| {
        anyhow!(
            "package `{}/{}` is not installed in `{}`",
            group,
            pkgref.name,
            workspace.root.display()
        )
    })?;
    let version = locked.version.clone();
    let kind = locked.kind;

    let slot = vibedeps::slot_rel_path(kind, &pkgref.name, &version);
    if !ctx.is_json() && !ctx.is_quiet() {
        ctx.heading(&format!(
            "\nUninstall {}/{}@{} — remove `{slot}` and regenerate boot.",
            group, pkgref.name, version
        ));
    }

    let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        true
    } else if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to uninstall non-interactively"
        );
    } else {
        Confirm::new()
            .with_prompt(format!("Uninstall {}/{}@{}?", group, pkgref.name, version))
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // Remove the package's materialised slot.
    vibedeps::remove_slot(&workspace.root, kind, &pkgref.name, &version)
        .context("removing the vibedeps/ slot")?;

    // Drop the lockfile entry and its root-dependency mirror. Identity is
    // `(group, name)` (PROP-008 §2.3).
    lockfile.remove(group, &pkgref.name);
    lockfile
        .meta
        .root_dependencies
        .retain(|r| !(r.group.as_ref() == Some(group) && r.name == pkgref.name));
    lockfile.meta.generated_at = crate::commands::init::current_timestamp_utc();

    // Drop the `[requires]` declaration from the project manifest.
    let manifest_changed = drop_from_manifest_requires(&mut manifest, group, &pkgref.name);
    if manifest_changed {
        manifest.write(project_root.join(Manifest::FILENAME))?;
    }

    // Regenerate every node's boot artifacts from the remaining
    // materialised state — the uninstalled package is gone from boot.
    regenerate_boot(&workspace).context("regenerating boot artifacts")?;

    lockfile.write(workspace.lockfile_path())?;

    emit_report(ctx, group, &pkgref.name, &version.to_string(), &slot)
}

/// Extract the `(group, …)` half of a pkgref's identity, rejecting an
/// unqualified `vibe uninstall` argument (PROP-008 §2.4).
fn require_group(pkgref: &PackageRef) -> Result<&Group> {
    pkgref.group.as_ref().ok_or_else(|| {
        anyhow!("package reference `{pkgref}` is not group-qualified — write `<group>/<name>`")
    })
}

/// Remove the matching pkgref from the project manifest's
/// `[requires].packages` AND `[requires].git_packages`. Returns `true`
/// iff an entry was actually removed from either list (caller persists
/// only on change). Pkgrefs are matched on `(group, name)` — the version
/// constraint / git ref policy is irrelevant for uninstall (PROP-008 §2.3).
fn drop_from_manifest_requires(manifest: &mut Manifest, group: &Group, name: &str) -> bool {
    let before_pkgs = manifest.requires.packages.len();
    manifest
        .requires
        .packages
        .retain(|r| !(r.group.as_ref() == Some(group) && r.name == name));
    let before_git = manifest.requires.git_packages.len();
    manifest
        .requires
        .git_packages
        .retain(|g| !(&g.group == group && g.name == name));
    manifest.requires.packages.len() != before_pkgs
        || manifest.requires.git_packages.len() != before_git
}

fn emit_report(
    ctx: &output::Context,
    group: &Group,
    name: &str,
    version: &str,
    slot: &str,
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "uninstall",
            "package": format!("{group}/{name}"),
            "version": version,
            "removed_slot": slot,
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!("vibe uninstall: {group}/{name}@{version} removed"));
        return Ok(());
    }
    ctx.removed(slot);
    ctx.summary(&format!(
        "\nUninstalled {group}/{name}@{version} — removed its vibedeps/ slot, regenerated boot."
    ));
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join("vibe.toml").exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    Ok(Lockfile::read(&path)?)
}

fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    Ok(Manifest::read(&path)?)
}
