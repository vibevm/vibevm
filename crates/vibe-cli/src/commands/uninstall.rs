//! `vibe uninstall <group>/<name>` — remove an installed package.
//!
//! Removing a direct declaration prunes its newly orphaned locked closure,
//! preserves packages still required by any workspace node, and regenerates
//! every node's boot artifacts from the remaining durable package world.
//!
//! Spec: spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009-loading-model.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};

use crate::exit_code::InstallError;
use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::UserConfig;
use vibe_workspace::Workspace;
use vibe_workspace::install::regenerate_boot_with_spec_format;
use vibe_workspace::materialization::{DestructiveGuard, guard_destructive};

use crate::cli::UninstallArgs;
use crate::commands::short_name;
use crate::output;

mod plan;
mod report;
use report::{emit_report, handle_adoption_facts};
mod transaction;

pub fn run(ctx: &output::Context, args: UninstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let mut manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_lockfile(&workspace.root)?;
    let user_config = UserConfig::load().context("loading the user config")?;
    let spec_format =
        crate::commands::install::resolve_spec_format(&manifest, user_config.install.spec_format);

    let pkgref =
        PackageRef::parse(&args.package).with_context(|| format!("parsing `{}`", args.package))?;
    // `vibe uninstall` acts on an already-installed package, so a bare
    // short name resolves against `vibe.lock` alone — no index, no network
    // (the lockfile is the authority for what is installed). A name that is
    // not locked fails here with a clear "not installed", not a lookup.
    let pkgref = short_name::qualify_locked(&pkgref, &lockfile)?;
    let Some(group) = pkgref.group.as_ref() else {
        bail!("`{pkgref}` resolved without a group — internal: `qualify_locked` should qualify");
    };

    // The materialised slot is keyed by `(group, name, version)`; the
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
    let selected_slot = plan::slot(locked);
    let plan = plan::build(
        &workspace,
        &project_root,
        &mut manifest,
        &lockfile,
        group,
        &pkgref.name,
    )?;
    let package_removed = plan
        .removed
        .iter()
        .any(|row| row.group == *group && row.name == pkgref.name);
    let packages = plan
        .removed
        .iter()
        .map(|row| format!("{}/{}@{}", row.group, row.name, row.version))
        .collect::<Vec<_>>();
    let intended = if packages.is_empty() {
        "none".to_owned()
    } else {
        packages.join(", ")
    };
    let retention = if package_removed {
        ""
    } else {
        "; selected package slot retained because surviving packages still require it"
    };
    let preflight = ctx.progress().task("Preflighting package removal");
    preflight.detail(format!(
        "remove {} packages: {intended}{retention}",
        packages.len()
    ));
    preflight.finish();
    if !ctx.is_json() && !ctx.is_quiet() {
        ctx.heading(&format!(
            "\nUninstall {group}/{}@{version} — remove {} packages: {intended}{retention}.",
            pkgref.name,
            packages.len()
        ));
    }
    let interactive = console::user_attended() && !ctx.is_json() && !ctx.is_unattended();
    let opted_in = args.assume_yes || ctx.is_unattended();
    let mut requires_confirmation = false;
    // Every planned slot is guarded before staging even the first one.
    for row in &plan.removed {
        match guard_destructive(row.materialization, interactive, opted_in) {
            DestructiveGuard::Abort => bail!(
                "`{}/{}` is materialised in-place (PROP-022 §2.6) — refusing to remove its project-local git clone non-interactively; pass `--assume-yes` to confirm the full removal plan",
                row.group,
                row.name
            ),
            DestructiveGuard::ConfirmInteractively => requires_confirmation = true,
            DestructiveGuard::Proceed => {}
        }
    }
    let approved = if requires_confirmation
        || !(args.assume_yes || ctx.is_unattended() || ctx.is_json())
    {
        if !interactive {
            bail!(
                "no TTY available for confirmation; re-run with `--assume-yes` to uninstall non-interactively"
            );
        }
        ctx.suspend_progress(|| {
            Confirm::with_theme(&crate::output::prompt_theme())
                .with_prompt(format!(
                    "Remove declaration {group}/{} and {} packages ({intended}){retention}?{}",
                    pkgref.name,
                    packages.len(),
                    if requires_confirmation {
                        " Includes in-place git clones; restoring them needs a network re-clone."
                    } else {
                        ""
                    }
                ))
                .default(false)
                .interact()
                .context("reading user confirmation")
        })?
    } else {
        true
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    for row in &plan.removed {
        lockfile.remove(&row.group, &row.name);
    }
    lockfile.meta.active_features.retain(|feature| {
        !plan
            .removed
            .iter()
            .any(|row| feature.starts_with(&format!("{}/{}/", row.group, row.name)))
    });
    lockfile.meta.root_dependencies.clone_from(&plan.roots);
    lockfile.meta.generated_at = crate::commands::init::current_timestamp_utc();
    let recording = ctx.progress().task(format!(
        "Removing {} package slots and recording remaining world",
        packages.len()
    ));
    if let Err(error) =
        transaction::apply(&workspace.root, &project_root, &plan, &lockfile, &manifest)
    {
        recording.fail("package removal transaction failed");
        return Err(error);
    }
    recording.finish();

    // Regenerate every node's boot artifacts from the remaining
    // materialised state — the uninstalled package is gone from boot. Re-open
    // the workspace so neither its manifest nor lock snapshot can retain the
    // just-pruned package across this durable-world boundary.
    let boot = ctx.progress().task("Regenerating workspace boot artifacts");
    let regenerated = (|| {
        let workspace = Workspace::discover(&project_root)
            .context("rediscovering the pruned workspace before boot regeneration")?;
        regenerate_boot_with_spec_format(&workspace, spec_format)
            .context("regenerating boot artifacts")
    })();
    if let Err(error) = regenerated {
        boot.fail("boot regeneration failed");
        return Err(error);
    }
    boot.finish();

    let package = format!("{group}/{}", pkgref.name);
    let adoption_facts =
        handle_adoption_facts(ctx, &project_root, &package, interactive, args.assume_yes)?;
    emit_report(
        ctx,
        group,
        &pkgref.name,
        &version.to_string(),
        if package_removed { &selected_slot } else { "" },
        &adoption_facts,
        &plan,
    )
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
