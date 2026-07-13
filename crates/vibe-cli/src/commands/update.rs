//! `vibe update [<pkgref>...] [--all]` — re-resolve and re-materialise.
//!
//! `vibe update` with no arguments, or `--all`, re-resolves the whole
//! declared graph — exactly the `vibe install` from-manifest path, so it
//! delegates there.
//!
//! `vibe update <pkgref>...` is **scoped**: only the named packages — and
//! the transitive subtree each pulls — are re-resolved against their
//! declared constraints and re-materialised. Every other package keeps
//! its lockfile version and its `vibedeps/` slot untouched. A package
//! whose version moves has its superseded slot removed, and the boot
//! artifacts are regenerated from the new `vibedeps/` state.
//!
//! Spec: spec://vibevm/modules/vibe-workspace/PROP-009-loading-model.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#command-summary");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, SourceKind};
use vibe_core::user_config::SlotIntegrity;
use vibe_core::{Group, PackageKind, PackageRef, VersionSpec};
use vibe_install::InstallSource;
use vibe_registry::{CachedPackage, ResolvedPackage};
use vibe_workspace::Workspace;
use vibe_workspace::install::{
    ResolvedDep, materialise_subtree, regenerate_boot, run_post_install_hooks,
};
use vibe_workspace::vibedeps;

use crate::cli::{InstallArgs, UpdateArgs};
use crate::commands::install::{build_install_resolver, exact_pinned_pkgref};
use crate::exit_code::InstallError;
use crate::output;

/// A subtree node the scoped update will refresh **in place** rather than
/// re-fetch: the lockfile already records it as `in-place` (PROP-022 §2.4) and
/// its slot is present, so it is `git fetch`-ed onto its own `.git` after
/// confirmation instead of re-cloned.
struct PendingInPlace {
    pkgref: PackageRef,
    kind: PackageKind,
    group: Group,
    name: String,
    version: semver::Version,
    registry: Option<String>,
    dependencies: Vec<PackageRef>,
}

pub fn run(ctx: &output::Context, args: UpdateArgs, embedded_root: Option<PathBuf>) -> Result<()> {
    // No arguments / `--all`: re-resolve the whole graph. That is the
    // `vibe install` from-manifest path exactly, so delegate to it.
    if args.all || args.packages.is_empty() {
        return super::install::run(ctx, install_args_from(&args), embedded_root);
    }

    // Scoped update: only the named packages and their subtrees move.
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_lockfile(&workspace.root)?;

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` configured in `{}/vibe.toml` — `vibe update` re-fetches \
             from the registry.",
            project_root.display()
        );
    }

    // Each named package must already be installed; re-resolve it against
    // its original root constraint so a caret bumps within range. Identity
    // is `(group, name)` — `vibe update` needs the qualified form
    // (PROP-008 §2.4).
    let mut roots: Vec<PackageRef> = Vec::with_capacity(args.packages.len());
    for raw in &args.packages {
        let pkgref = PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
        let group = require_group(&pkgref)?.clone();
        if lockfile.find(&group, &pkgref.name).is_none() {
            bail!(
                "package `{group}/{}` is not installed — `vibe update` only refreshes installed \
                 packages; use `vibe install {group}/{}` to add it.",
                pkgref.name,
                pkgref.name,
            );
        }
        // The constraint to re-resolve against: the manifest `[requires]`
        // declaration is authoritative — the operator edits it to widen a
        // pin before updating — and the lockfile's `root_dependencies`
        // mirror is only the fallback.
        let constraint = manifest
            .requires
            .packages
            .iter()
            .find(|r| r.group.as_ref() == Some(&group) && r.name == pkgref.name)
            .or_else(|| {
                lockfile
                    .meta
                    .root_dependencies
                    .iter()
                    .find(|r| r.group.as_ref() == Some(&group) && r.name == pkgref.name)
            })
            .map(|r| r.version.clone())
            .unwrap_or(VersionSpec::Latest);
        roots.push(PackageRef::new(
            pkgref.kind,
            Some(group),
            pkgref.name,
            constraint,
        )?);
    }

    let resolver = build_install_resolver(
        &install_args_from(&args),
        &manifest,
        embedded_root.as_deref(),
    )?;

    ctx.heading(&format!(
        "Re-resolving {} package{}…",
        roots.len(),
        if roots.len() == 1 { "" } else { "s" },
    ));
    let graph = resolver
        .solve(&roots)
        .context("dependency resolution failed")?;

    let cache_root = workspace.root.join(".vibe/cache");
    fs::create_dir_all(&cache_root)
        .with_context(|| format!("creating cache dir `{}`", cache_root.display()))?;

    // Fetch every node of the named subtree. A package the lockfile already
    // records as in-place with a present slot is NOT re-fetched: it is updated
    // incrementally on its own `.git` (PROP-022 §2.4) — a version bump on a
    // giant transfers only changed objects rather than re-cloning the tree. We
    // resolve those nodes here but defer the slot mutation past the confirm.
    let mut updated: Vec<(CachedPackage, Vec<PackageRef>)> = Vec::new();
    let mut pending_in_place: Vec<PendingInPlace> = Vec::new();
    for node in graph.iter() {
        let pkgref = exact_pinned_pkgref(node);
        if let Some(old) = lockfile.find(&node.group, &node.name)
            && old.materialization.is_in_place()
            && vibedeps::is_in_place_slot(&workspace.root, old.kind, &node.name)
        {
            pending_in_place.push(PendingInPlace {
                pkgref,
                kind: old.kind,
                group: node.group.clone(),
                name: node.name.clone(),
                version: node.version.clone(),
                registry: old.registry.clone(),
                dependencies: node.dependencies.clone(),
            });
            continue;
        }
        let cached = resolver.resolve_and_fetch(&pkgref, &cache_root, None)?;
        updated.push((cached, node.dependencies.clone()));
    }
    let total = updated.len() + pending_in_place.len();

    let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        true
    } else if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to update non-interactively"
        );
    } else {
        Confirm::new()
            .with_prompt(format!(
                "Re-materialise {} package{} into vibedeps/ and regenerate boot?",
                total,
                if total == 1 { "" } else { "s" },
            ))
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // Confirmed — perform the deferred incremental in-place updates, then fold
    // each into the same `updated` set so the resolution / lockfile / hook flow
    // treats it uniformly. The built `CachedPackage`'s `cache_dir` IS the slot,
    // which signals "already placed" to the materialise pass (it runs the hook
    // but skips any move).
    for p in pending_in_place {
        let slot = vibedeps::in_place_slot_abs_path(&workspace.root, p.kind, &p.name);
        let placed = resolver
            .materialise_in_place(&p.pkgref, &slot)
            .with_context(|| format!("updating in-place `{}/{}`", p.group, p.name))?;
        vibedeps::ensure_gitignored(
            &workspace.root,
            &vibedeps::in_place_slot_rel_path(p.kind, &p.name),
        )
        .context("gitignoring the in-place slot")?;
        let cached = CachedPackage {
            resolved: ResolvedPackage {
                group: p.group.clone(),
                name: p.name.clone(),
                version: p.version.clone(),
                source_dir: slot.clone(),
            },
            cache_dir: slot,
            manifest: placed.manifest,
            content_hash: placed.content_hash,
            source_uri: placed.source_uri,
            registry_name: p.registry,
            source_ref: Some(placed.source_ref),
            resolved_commit: placed.resolved_commit,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            via_redirect: None,
        };
        updated.push((cached, p.dependencies));
    }

    // Build the partial resolution for the subtree — the form the shared
    // materialise + hook flow consumes (the same `ResolvedDep` shape
    // `vibe install` hands to `apply_resolution`).
    let resolution: Vec<ResolvedDep> = updated
        .iter()
        .map(|(cached, deps)| ResolvedDep {
            kind: cached.package_meta().kind,
            group: cached.resolved.group.clone(),
            name: cached.resolved.name.clone(),
            version: cached.resolved.version.clone(),
            content_dir: cached.cache_dir.clone(),
            manifest: cached.manifest.clone(),
            requires: deps
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                .collect(),
            // Mutable iff an in-workspace `file://` self-hosting source the
            // author edits in place (PROP-011 §2.6); recorded so the materialise
            // pass re-copies its slot.
            source_mutable: vibe_workspace::freshness::is_in_workspace_file_source(
                &cached.source_uri,
                &workspace.root,
            ),
        })
        .collect();

    // PROP-020 §2.1 — `vibe update` resets and re-runs install hooks. Resolve
    // hook trust (allow-list / interactive consent / abort) before touching
    // any slot, exactly as `vibe install` does.
    let hook_policy =
        crate::commands::install::resolve_hook_policy(ctx, &install_args_from(&args), &resolution)?;

    // Remove any superseded *versioned* slot so a bump leaves no stale slot
    // (an in-place slot is unversioned — nothing to prune), and record the
    // bumps for the report.
    let mut bumps: Vec<String> = Vec::new();
    for (cached, _) in &updated {
        let name = &cached.resolved.name;
        let Some(old_v) = lockfile
            .find(&cached.resolved.group, name)
            .map(|o| o.version.clone())
            .filter(|v| *v != cached.resolved.version)
        else {
            continue;
        };
        bumps.push(format!(
            "{}/{} {} -> {}",
            cached.resolved.group, name, old_v, cached.resolved.version
        ));
        if !cached.package_meta().materialization.is_in_place() {
            vibedeps::remove_slot(&workspace.root, cached.package_meta().kind, name, &old_v)
                .context("removing the superseded vibedeps/ slot")?;
        }
    }

    // Materialise the subtree (snapshot copy / hardlink / in-place move) and
    // run each freshly-placed slot's pre-install hook (PROP-020 §2.1) — no
    // prune, no boot here; boot is regenerated below from the whole tree.
    // `Verify` re-materialises every named slot from the fresh fetch.
    let subtree = materialise_subtree(
        &workspace.root,
        &resolution,
        SlotIntegrity::Verify,
        Some(&hook_policy),
    )
    .context("re-materialising the updated subtree")?;

    // Regenerate every node's boot from the new `vibedeps/` state.
    regenerate_boot(&workspace).context("regenerating boot artifacts")?;

    // Replace each subtree package's lockfile entry, carrying the
    // install-scoped metadata (features / language) the version bump does
    // not change.
    for (cached, deps) in &updated {
        let old = lockfile.find(&cached.resolved.group, &cached.resolved.name);
        let entry = locked_package(cached, deps, old);
        match lockfile
            .packages
            .iter()
            .position(|p| p.group == entry.group && p.name == entry.name)
        {
            Some(i) => lockfile.packages[i] = entry,
            None => lockfile.packages.push(entry),
        }
    }
    lockfile.meta.generated_at = crate::commands::init::current_timestamp_utc();
    lockfile.write(workspace.lockfile_path())?;

    // PROP-020 §2.1 — post-install hooks run once the updated packages are
    // durable (lockfile written, boot regenerated).
    run_post_install_hooks(
        &workspace.root,
        &resolution,
        &subtree.materialised,
        &hook_policy,
    )
    .context("running post-install hooks")?;

    emit_report(ctx, updated.len(), &bumps);
    Ok(())
}

/// Build the `InstallArgs` that `vibe update`'s whole-graph path delegates
/// with, and that `build_install_resolver` reads. `vibe update` carries no
/// `--registry` / `--git` / feature flags, so those default off.
fn install_args_from(args: &UpdateArgs) -> InstallArgs {
    InstallArgs {
        packages: Vec::new(),
        path: args.path.clone(),
        registry: None,
        assume_yes: args.assume_yes,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: args.exact,
        auth_required: args.auth_required,
        solver: None,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
        // `vibe update` carries no `--allow-hooks`; hook consent on the
        // whole-graph path is resolved by the `vibe install` it delegates to.
        allow_hooks: false,
    }
}

/// Build the lockfile entry for a re-resolved package. Version, hash and
/// source come from the fresh fetch; the install-scoped `features` /
/// `subskills_active` / `language` are carried from the previous entry —
/// a version bump does not re-evaluate them.
fn locked_package(
    cached: &CachedPackage,
    dependencies: &[PackageRef],
    old: Option<&LockedPackage>,
) -> LockedPackage {
    let source_kind = if cached.overridden {
        SourceKind::Override
    } else if cached.is_path_source {
        SourceKind::Path
    } else if cached.is_git_source {
        SourceKind::Git
    } else {
        SourceKind::Registry
    };
    LockedPackage {
        kind: cached.package_meta().kind,
        group: cached.resolved.group.clone(),
        name: vibe_core::PackageName::from_validated(cached.resolved.name.clone()),
        version: cached.resolved.version.clone(),
        registry: cached.registry_name.clone(),
        source_url: vibe_core::SourceUrl::new(cached.source_uri.clone()),
        source_ref: cached.source_ref.clone(),
        resolved_commit: cached.resolved_commit.clone(),
        content_hash: vibe_core::ContentHash::from_validated(cached.content_hash.clone()),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: dependencies.to_vec(),
        overridden: cached.overridden,
        source_kind: Some(source_kind),
        via_redirect: cached.via_redirect.clone(),
        features: old.map(|o| o.features.clone()).unwrap_or_default(),
        subskills_active: old.map(|o| o.subskills_active.clone()).unwrap_or_default(),
        describes: cached
            .package_meta()
            .describes
            .as_ref()
            .map(|p| p.to_string()),
        language: old.and_then(|o| o.language.clone()),
        // A version bump does not change how the package is materialised —
        // carry the freshly-fetched manifest's declared mode (PROP-022 §2.1).
        materialization: cached.package_meta().materialization,
    }
}

fn emit_report(ctx: &output::Context, count: usize, bumps: &[String]) {
    if ctx.is_json() {
        let _ = ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "update",
            "packages_resolved": count,
            "version_bumps": bumps,
        }));
        return;
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe update: {count} package{} re-resolved, {} bump{}",
            if count == 1 { "" } else { "s" },
            bumps.len(),
            if bumps.len() == 1 { "" } else { "s" },
        ));
        return;
    }
    for b in bumps {
        ctx.created(b);
    }
    ctx.summary(&format!(
        "\nUpdated {count} package{} ({} version bump{}).",
        if count == 1 { "" } else { "s" },
        bumps.len(),
        if bumps.len() == 1 { "" } else { "s" },
    ));
}

/// Extract the `(group, …)` half of a pkgref's identity, rejecting an
/// unqualified `vibe update` argument (PROP-008 §2.4).
fn require_group(pkgref: &PackageRef) -> Result<&Group> {
    pkgref.group.as_ref().ok_or_else(|| {
        anyhow!("package reference `{pkgref}` is not group-qualified — write `<group>/<name>`")
    })
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

fn load_project_manifest(root: &Path) -> Result<Manifest> {
    Ok(Manifest::read(root.join(Manifest::FILENAME))?)
}

fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        bail!(
            "no `vibe.lock` in `{}` — nothing to update; run `vibe install` first",
            root.display()
        );
    }
}
