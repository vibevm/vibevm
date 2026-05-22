//! `vibe reinstall [<path>] [--force]` — recompute the materialised state
//! and the boot artifacts of a workspace.
//!
//! `vibe reinstall` regenerates the computed loading model (PROP-009
//! §2.10). It **never re-resolves** — the versions stay exactly as
//! `vibe.lock` pins them; moving a version is `vibe update`'s job.
//!
//! Two modes:
//!
//! - **`vibe reinstall`** (no `--force`) — recompute every node's boot
//!   artifacts from the materialised `vibedeps/` tree already on disk.
//!   No fetch, no network. The fix for a stale or hand-edited boot
//!   artifact — a previous generation pass that produced a wrong
//!   `INDEX.md`. Every locked package must have its `vibedeps/` slot
//!   present; a missing slot is content this mode cannot recover, so it
//!   stops and points the operator at `--force`.
//! - **`vibe reinstall --force`** — re-fetch every locked package's
//!   content from its source repository at the lockfile-pinned version,
//!   bypassing the project cache, then re-materialise `vibedeps/` and
//!   regenerate boot. The escape hatch for a corrupted `vibedeps/`
//!   subtree.
//!
//! Discovery bubbles to the absolute workspace root, so reinstalling
//! regenerates the whole workspace — a node and every ancestor
//! (PROP-009 §2.10): a node's aggregated boot depends on its members'.
//!
//! Spec: spec://vibevm/modules/vibe-workspace/PROP-009-loading-model §2.10.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{PackageKind, PackageRef, VersionSpec};
use vibe_workspace::Workspace;
use vibe_workspace::install::{ResolvedDep, apply_resolution, regenerate_boot};
use vibe_workspace::vibedeps;

use crate::cli::{InstallArgs, ReinstallArgs};
use crate::commands::install::build_install_resolver;
use crate::exit_code::InstallError;
use crate::output;

pub fn run(ctx: &output::Context, args: ReinstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let lockfile = load_lockfile(&workspace.root)?;

    if args.force {
        run_force(ctx, &workspace, &lockfile, &args)
    } else {
        run_regenerate(ctx, &workspace, &lockfile, &args)
    }
}

/// `vibe reinstall` — regenerate every node's boot artifacts from the
/// materialised `vibedeps/` tree already on disk. No fetch, no network.
fn run_regenerate(
    ctx: &output::Context,
    workspace: &Workspace,
    lockfile: &Lockfile,
    args: &ReinstallArgs,
) -> Result<()> {
    // Without `--force` the materialised `vibedeps/` tree is the only
    // content source. Every locked package must have its slot on disk —
    // a missing slot is content this mode cannot conjure; only a fetch
    // (`--force`) can.
    let missing: Vec<String> = lockfile
        .packages
        .iter()
        .filter(|p| !vibedeps::is_materialised(&workspace.root, p.kind, &p.name, &p.version))
        .map(|p| vibedeps::slot_rel_path(p.kind, &p.name, &p.version))
        .collect();
    if !missing.is_empty() {
        bail!(
            "the materialised `vibedeps/` tree is incomplete — {} slot{} missing:\n  {}\n\
             Run `vibe reinstall --force` to re-fetch the content from source.",
            missing.len(),
            if missing.len() == 1 { "" } else { "s" },
            missing.join("\n  "),
        );
    }

    let node_count = workspace.iter_nodes().count();
    ctx.heading(&format!(
        "\nReinstall — regenerate boot artifacts for {node_count} node{} from vibedeps/.",
        if node_count == 1 { "" } else { "s" },
    ));

    if !confirm(
        ctx,
        args,
        "Regenerate the boot artifacts from the materialised vibedeps/ tree?",
    )? {
        return Err(InstallError::UserDeclined.into());
    }

    let nodes = regenerate_boot(workspace).context("regenerating boot artifacts")?;
    emit_report(ctx, false, &nodes, &[]);
    Ok(())
}

/// `vibe reinstall --force` — re-fetch every locked package from source,
/// bypassing the project cache, then re-materialise and regenerate boot.
fn run_force(
    ctx: &output::Context,
    workspace: &Workspace,
    lockfile: &Lockfile,
    args: &ReinstallArgs,
) -> Result<()> {
    // No locked packages — `--force` has nothing to re-fetch. Still
    // regenerate boot so a stale artifact is recomputed.
    if lockfile.packages.is_empty() {
        ctx.heading("\nReinstall --force — no packages locked; regenerate boot only.");
        if !confirm(ctx, args, "No packages are locked — regenerate boot artifacts only?")? {
            return Err(InstallError::UserDeclined.into());
        }
        // `--force` always re-materialises — `SlotIntegrity::Verify` —
        // though with an empty resolution there is nothing to copy.
        let outcome = apply_resolution(workspace, &[], SlotIntegrity::Verify)
            .context("regenerating the workspace")?;
        emit_report(ctx, true, &outcome.nodes_regenerated, &outcome.pruned);
        return Ok(());
    }

    ctx.heading(&format!(
        "\nReinstall --force — re-fetch {} package{} from source:",
        lockfile.packages.len(),
        if lockfile.packages.len() == 1 { "" } else { "s" },
    ));
    for p in &lockfile.packages {
        ctx.step(&format!("{}:{}@{}", p.kind, p.name, p.version));
    }

    if !confirm(
        ctx,
        args,
        &format!(
            "Re-fetch {} package{} from source and re-materialise vibedeps/?",
            lockfile.packages.len(),
            if lockfile.packages.len() == 1 { "" } else { "s" },
        ),
    )? {
        return Err(InstallError::UserDeclined.into());
    }

    // The resolver is built from the workspace root manifest — registries,
    // mirrors, overrides, and git-source declarations are root-level.
    let resolver = build_install_resolver(&resolver_args(), &workspace.root_manifest)
        .context("building the install resolver")?;

    // Bypass the cache — wipe the project package cache so every fetch
    // re-downloads from source (PROP-009 §2.10).
    let cache_root = workspace.root.join(".vibe/cache");
    if cache_root.exists() {
        fs::remove_dir_all(&cache_root)
            .with_context(|| format!("clearing the cache `{}`", cache_root.display()))?;
    }
    fs::create_dir_all(&cache_root)
        .with_context(|| format!("creating the cache dir `{}`", cache_root.display()))?;

    // Re-fetch every locked package at its exact pinned version — no
    // re-resolution, the lockfile decides the version. The recorded
    // `content_hash` is forwarded so a source serving disagreeing bytes
    // is rejected: `vibe reinstall` reproduces the lock, never drifts it.
    let mut resolution: Vec<ResolvedDep> = Vec::with_capacity(lockfile.packages.len());
    for locked in &lockfile.packages {
        let pkgref = exact_pkgref(locked.kind, &locked.name, &locked.version)?;
        let cached = resolver
            .resolve_and_fetch(&pkgref, &cache_root, Some(&locked.content_hash))
            .with_context(|| {
                format!(
                    "re-fetching `{}:{}@{}` from source",
                    locked.kind, locked.name, locked.version
                )
            })?;
        resolution.push(ResolvedDep {
            kind: cached.resolved.kind,
            name: cached.resolved.name.clone(),
            version: cached.resolved.version.clone(),
            content_dir: cached.cache_dir.clone(),
            manifest: cached.manifest.clone(),
            // The recorded resolution edges — `apply_resolution` walks
            // them to compose each node's dependency boot.
            requires: locked
                .dependencies
                .iter()
                .map(|p| (p.kind, p.name.clone()))
                .collect(),
        });
    }

    // `--force` re-fetched every slot's content; `SlotIntegrity::Verify`
    // makes `apply_resolution` overwrite every slot rather than trust a
    // present one — re-materialisation is the whole point of `--force`.
    let outcome = apply_resolution(workspace, &resolution, SlotIntegrity::Verify)
        .context("re-materialising the workspace")?;
    emit_report(ctx, true, &outcome.nodes_regenerated, &outcome.pruned);
    Ok(())
}

/// Build the `=<version>` pkgref that re-fetches exactly the locked
/// version — `vibe reinstall` never re-resolves.
fn exact_pkgref(kind: PackageKind, name: &str, version: &semver::Version) -> Result<PackageRef> {
    let req = semver::VersionReq::parse(&format!("={version}"))
        .expect("`=<version>` always parses as a VersionReq");
    Ok(PackageRef::new(kind, name.to_string(), VersionSpec::Req(req))?)
}

/// The `InstallArgs` `build_install_resolver` reads. `vibe reinstall`
/// carries no `--registry` / `--git` / feature flags, so they default
/// off — the resolver is built purely from the manifest's `[[registry]]`
/// / `[[mirror]]` / `[[override]]` / git-source declarations.
fn resolver_args() -> InstallArgs {
    InstallArgs {
        packages: Vec::new(),
        path: PathBuf::from("."),
        registry: None,
        assume_yes: false,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: false,
        auth_required: false,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
    }
}

/// Interactive confirmation, matching the install / update / uninstall
/// contract: `--assume-yes`, `--unattended`, and `--json` all imply yes;
/// a non-TTY with none of those set is a hard error.
fn confirm(ctx: &output::Context, args: &ReinstallArgs, prompt: &str) -> Result<bool> {
    if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        return Ok(true);
    }
    if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to reinstall \
             non-interactively"
        );
    }
    Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .context("reading user confirmation")
}

/// Report the outcome — JSON envelope, quiet one-liner, or human summary.
fn emit_report(
    ctx: &output::Context,
    forced: bool,
    nodes_regenerated: &[String],
    pruned: &[String],
) {
    if ctx.is_json() {
        let _ = ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "reinstall",
            "forced": forced,
            "nodes_regenerated": nodes_regenerated,
            "pruned": pruned,
        }));
        return;
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe reinstall: boot artifacts regenerated for {} node{}",
            nodes_regenerated.len(),
            if nodes_regenerated.len() == 1 { "" } else { "s" },
        ));
        return;
    }
    ctx.summary(&format!(
        "\nReinstalled — regenerated boot artifacts for {} node{}{}.",
        nodes_regenerated.len(),
        if nodes_regenerated.len() == 1 { "" } else { "s" },
        if forced { " from a fresh fetch" } else { "" },
    ));
    if !pruned.is_empty() {
        ctx.step(&format!(
            "pruned {} stale vibedeps/ slot{}",
            pruned.len(),
            if pruned.len() == 1 { "" } else { "s" },
        ));
    }
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

/// Load the workspace lockfile, or an empty one when none exists yet.
/// `vibe reinstall` does not require a lockfile — without one it simply
/// regenerates the boot artifacts from the authored `spec/boot/` tree.
fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            super::init::current_timestamp_utc(),
        ))
    }
}
