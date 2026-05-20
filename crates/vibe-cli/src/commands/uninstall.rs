//! `vibe uninstall <kind>:<name>` — remove an installed package.
//!
//! Spec: `VIBEVM-SPEC.md` §9.1, §11.1.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_install::{InstallError, apply_uninstall, plan_uninstall, unregister_installed};

use crate::cli::UninstallArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: UninstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let mut manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_lockfile(&project_root)?;

    let pkgref = PackageRef::parse(&args.package)
        .with_context(|| format!("parsing `{}`", args.package))?;

    let plan = plan_uninstall(&project_root, &lockfile, &pkgref)?;

    if !ctx.is_json() && !ctx.is_quiet() {
        ctx.heading(&format!(
            "\nPlan for uninstall {}:{}@{}",
            plan.kind, plan.name, plan.version
        ));
        for rel in &plan.removed_paths {
            println!("  remove  {}", rel.to_string_lossy().replace('\\', "/"));
        }
        println!();
    }

    let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        true
    } else if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to uninstall non-interactively"
        );
    } else {
        Confirm::new()
            .with_prompt(format!(
                "Remove {} file{} for {}:{}?",
                plan.removed_paths.len(),
                if plan.removed_paths.len() == 1 { "" } else { "s" },
                plan.kind,
                plan.name,
            ))
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    let removed = apply_uninstall(&plan)?;
    let _entry = unregister_installed(
        &mut lockfile,
        &pkgref,
        crate::commands::init::current_timestamp_utc(),
    )?;

    // Drop the pkgref from `vibe.toml` `[requires].packages` if it was
    // declared there. `unregister_installed` already removed the
    // matching entry from `lockfile.meta.root_dependencies`; this
    // mirror keeps the manifest authoritative for user-declared deps
    // (PROP-002 §2.7). No-op when uninstalling a pure transitive that
    // was never declared in the manifest.
    let manifest_changed = drop_from_manifest_requires(&mut manifest, &pkgref);
    if manifest_changed {
        manifest.write(project_root.join(Manifest::FILENAME))?;
    }

    lockfile.write(project_root.join(Lockfile::FILENAME))?;

    emit_report(ctx, &plan.kind.to_string(), &plan.name, &plan.version.to_string(), &removed)
}

/// Remove the matching pkgref from the project manifest's
/// `[requires].packages` AND `[requires].git_packages`. Returns `true`
/// iff an entry was actually removed from either list (caller persists
/// only on change). Pkgrefs are matched on `(kind, name)` — the version
/// constraint / git ref policy is irrelevant for uninstall.
fn drop_from_manifest_requires(manifest: &mut Manifest, pkgref: &PackageRef) -> bool {
    let before_pkgs = manifest.requires.packages.len();
    manifest
        .requires
        .packages
        .retain(|r| !(r.kind == pkgref.kind && r.name == pkgref.name));
    let before_git = manifest.requires.git_packages.len();
    manifest
        .requires
        .git_packages
        .retain(|g| !(g.kind == pkgref.kind && g.name == pkgref.name));
    manifest.requires.packages.len() != before_pkgs
        || manifest.requires.git_packages.len() != before_git
}

#[derive(Debug, Serialize)]
struct UninstallReport {
    ok: bool,
    command: &'static str,
    package: String,
    version: String,
    removed_count: usize,
    paths: Vec<String>,
}

fn emit_report(
    ctx: &output::Context,
    kind: &str,
    name: &str,
    version: &str,
    removed: &[std::path::PathBuf],
) -> Result<()> {
    let paths: Vec<String> = removed
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();

    if ctx.is_json() {
        let report = UninstallReport {
            ok: true,
            command: "uninstall",
            package: format!("{kind}:{name}"),
            version: version.to_string(),
            removed_count: paths.len(),
            paths,
        };
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe uninstall: {kind}:{name}@{version}, {} file{} removed",
            paths.len(),
            if paths.len() == 1 { "" } else { "s" }
        ));
        return Ok(());
    }
    for p in &paths {
        ctx.removed(p);
    }
    ctx.summary(&format!(
        "\nUninstalled {kind}:{name}@{version} ({} file{}).",
        paths.len(),
        if paths.len() == 1 { "" } else { "s" }
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
