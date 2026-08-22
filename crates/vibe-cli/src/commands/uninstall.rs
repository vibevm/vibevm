//! `vibe uninstall <group>/<name>` — remove an installed package.
//!
//! In the PROP-009 loading model, uninstalling a package removes its
//! `vibedeps/` slot, drops its lockfile entry and its `[requires]`
//! declaration, and regenerates every node's boot artifacts so the
//! package no longer appears in the computed boot sequence.
//!
//! Spec: spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009-loading-model.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};

use crate::exit_code::InstallError;
use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::UserConfig;
use vibe_core::{Group, PackageRef};
use vibe_facts::{package_file_path, remove_package_file};
use vibe_workspace::Workspace;
use vibe_workspace::install::regenerate_boot_with_spec_format;
use vibe_workspace::materialization::{DestructiveGuard, guard_destructive};
use vibe_workspace::vibedeps;

use crate::cli::UninstallArgs;
use crate::commands::short_name;
use crate::output;

pub fn run(ctx: &output::Context, args: UninstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let mut manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_lockfile(&workspace.root)?;
    let user_config = UserConfig::load().context("loading the user config")?;
    let spec_format = crate::commands::install::resolve_spec_format(&manifest, &user_config);

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
    // The recorded materialization mode drives the PROP-022 §2.6 destructive
    // guard below — an in-place slot is a non-vendored git clone.
    let mode = locked.materialization;

    // The slot path depends on the mode: an in-place slot is the unversioned
    // `vibedeps/<group>.<name>/` git working tree (PROP-022 §2.4); every other
    // mode is the versioned slot.
    let slot = if mode.is_in_place() {
        vibedeps::in_place_slot_rel_path(&locked.group, &pkgref.name)
    } else {
        vibedeps::slot_rel_path(&locked.group, &pkgref.name, &version)
    };
    if !ctx.is_json() && !ctx.is_quiet() {
        ctx.heading(&format!(
            "\nUninstall {}/{}@{} — remove `{slot}` and regenerate boot.",
            group, pkgref.name, version
        ));
    }

    // PROP-022 §2.6 — removing an `in-place` slot deletes a project-local git
    // clone whose only restoration is a network re-clone; it must never happen
    // silently. The guard refuses a non-interactive run with no explicit
    // opt-in, and forces a mandatory `y/n` that `--json` cannot auto-answer.
    let interactive = console::user_attended() && !ctx.is_json() && !ctx.is_unattended();
    let opted_in = args.assume_yes || ctx.is_unattended();
    let approved = match guard_destructive(mode, interactive, opted_in) {
        DestructiveGuard::Abort => bail!(
            "`{group}/{}` is materialised in-place (PROP-022 §2.6) — a project-local git \
             clone restorable only by re-cloning over the network. Refusing to remove it \
             non-interactively; re-run interactively or pass `--assume-yes` to confirm.",
            pkgref.name,
        ),
        DestructiveGuard::ConfirmInteractively => Confirm::new()
            .with_prompt(format!(
                "Remove the in-place slot for {}/{}@{}? This deletes the local git clone; \
                 restoring it needs a network re-clone.",
                group, pkgref.name, version
            ))
            .default(false)
            .interact()
            .context("reading user confirmation")?,
        // Non-in-place, or in-place with an explicit opt-in: the established
        // uninstall confirmation contract (`--assume-yes` / `--unattended` /
        // `--json` imply yes; a non-TTY without them is a hard error).
        DestructiveGuard::Proceed => {
            if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
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
            }
        }
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // Remove the package's materialised slot — the unversioned in-place git
    // working tree, or the versioned copy/hardlink slot.
    if mode.is_in_place() {
        vibedeps::remove_in_place_slot(&workspace.root, &locked.group, &pkgref.name)
            .context("removing the in-place vibedeps/ slot")?;
    } else {
        vibedeps::remove_slot(&workspace.root, &locked.group, &pkgref.name, &version)
            .context("removing the vibedeps/ slot")?;
    }

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
    regenerate_boot_with_spec_format(&workspace, spec_format)
        .context("regenerating boot artifacts")?;

    lockfile.write(workspace.lockfile_path())?;

    let package = format!("{group}/{}", pkgref.name);
    let adoption_facts =
        handle_adoption_facts(&project_root, &package, interactive, args.assume_yes)?;
    emit_report(
        ctx,
        group,
        &pkgref.name,
        &version.to_string(),
        &slot,
        &adoption_facts,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AdoptionFactsDisposition {
    Absent,
    Removed(String),
    Kept(String),
}

fn handle_adoption_facts(
    project_root: &Path,
    package: &str,
    interactive: bool,
    assume_yes: bool,
) -> Result<AdoptionFactsDisposition> {
    let file = package_file_path(project_root, package);
    if !file.is_file() {
        return Ok(AdoptionFactsDisposition::Absent);
    }
    let display = file
        .strip_prefix(project_root)
        .unwrap_or(&file)
        .to_string_lossy()
        .replace('\\', "/");
    let remove = if interactive && !assume_yes {
        Confirm::new()
            .with_prompt(format!("Remove its adoption facts ({display})?"))
            .default(false)
            .interact()
            .context("reading adoption-facts confirmation")?
    } else {
        false
    };
    if !remove {
        return Ok(AdoptionFactsDisposition::Kept(display));
    }
    if !remove_package_file(project_root, package)? {
        bail!("adoption facts `{display}` disappeared before removal");
    }
    Ok(AdoptionFactsDisposition::Removed(display))
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
    adoption_facts: &AdoptionFactsDisposition,
) -> Result<()> {
    if ctx.is_json() {
        let adoption_facts = match adoption_facts {
            AdoptionFactsDisposition::Absent => serde_json::json!({"status": "absent"}),
            AdoptionFactsDisposition::Removed(path) => {
                serde_json::json!({"status": "removed", "path": path})
            }
            AdoptionFactsDisposition::Kept(path) => serde_json::json!({
                "status": "kept",
                "path": path,
                "hint": "run `vibe facts clean` to drop orphaned overlays",
            }),
        };
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "uninstall",
            "package": format!("{group}/{name}"),
            "version": version,
            "removed_slot": slot,
            "adoption_facts": adoption_facts,
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let facts = match adoption_facts {
            AdoptionFactsDisposition::Kept(path) => {
                format!("; kept {path}; run `vibe facts clean` to drop orphaned overlays")
            }
            AdoptionFactsDisposition::Removed(path) => format!("; removed {path}"),
            AdoptionFactsDisposition::Absent => String::new(),
        };
        ctx.summary(&format!(
            "vibe uninstall: {group}/{name}@{version} removed{facts}"
        ));
        return Ok(());
    }
    ctx.removed(slot);
    match adoption_facts {
        AdoptionFactsDisposition::Absent => {}
        AdoptionFactsDisposition::Removed(path) => ctx.removed(path),
        AdoptionFactsDisposition::Kept(path) => ctx.skipped(
            path,
            "adoption facts kept; run `vibe facts clean` to drop orphaned overlays",
        ),
    }
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
