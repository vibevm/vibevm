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
//! Spec: spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009-loading-model.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

mod inputs;
pub(crate) mod lifecycle;
mod report;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::user_config::{SlotIntegrity, UserConfig};
use vibe_core::{ContentHash, Group, PackageRef, VersionSpec};
use vibe_install::{InstallSlotLifecycle, InstallSource};
use vibe_registry::{CachedPackage, ResolvedPackage};
use vibe_workspace::Workspace;
use vibe_workspace::install::{
    ResolvedDep, SlotCheck, SlotLifecycleMode, SlotVerifier,
    materialise_subtree_with_spec_format_and_slot_lifecycle, regenerate_boot_with_spec_format,
    run_post_install_slot_lifecycle,
};
use vibe_workspace::vibedeps;

use crate::cli::UpdateArgs;
use crate::commands::install::{
    LifecycleHookView, build_install_resolver, emit_closure_diff, exact_pinned_pkgref, lane_sizes,
};
use crate::commands::short_name;
use crate::exit_code::InstallError;
use crate::output;

use inputs::{
    install_args_from, load_lockfile, load_project_manifest, locked_package, resolve_project_root,
};
use report::{emit_report, emit_update_document};

/// A subtree node the scoped update will refresh **in place** rather than
/// re-fetch: the lockfile already records it as `in-place` (PROP-022 §2.4) and
/// its slot is present, so it is `git fetch`-ed onto its own `.git` after
/// confirmation instead of re-cloned.
struct PendingInPlace {
    pkgref: PackageRef,
    group: Group,
    name: String,
    version: semver::Version,
    registry: Option<String>,
    dependencies: Vec<PackageRef>,
}

struct SourceHashes(HashMap<(Group, String), String>);

impl SlotVerifier for SourceHashes {
    fn source_hash<'a>(&'a self, dep: &ResolvedDep) -> Option<&'a str> {
        self.0
            .get(&(dep.group.clone(), dep.name.clone()))
            .map(String::as_str)
    }

    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        SlotCheck::Unverifiable
    }
}

pub fn run(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    // No arguments / `--all`: re-resolve the whole graph. That is the
    // `vibe install` from-manifest path exactly, so delegate to it — the
    // root offline flag travels along and the delegate resolves the full
    // posture against its own user-config load.
    if args.all || args.packages.is_empty() {
        // The whole-graph update runs the install substrate, but it is still
        // `vibe update`: it supplies its OWN requested phase, chain and run
        // identity, so the handoff a hosted row publishes resumes with
        // `vibe update` rather than impersonating install.
        let project_root = resolve_project_root(&args.path)?;
        let user_config = UserConfig::load().context("loading the user config")?;
        let metadata = lifecycle::metadata(
            ctx,
            &project_root,
            "update",
            root_offline || user_config.net.offline,
            args.assume_yes,
        )?;
        let run = super::install::run_with_lifecycle_context(
            ctx,
            install_args_from(&args),
            embedded_root,
            root_offline,
            Some(metadata),
            None,
            |_, _, _| Ok(super::install::WorldCallbackOutcome::default()),
        )?;
        if let Some(delegation) = run.parked.as_ref() {
            crate::commands::lifecycle::check_delegation(delegation)?;
        }
        return emit_update_document(
            ctx,
            report::UpdateOutcome {
                project_root: &project_root,
                args: &args,
                progress: &run.progress,
                packages_resolved: run.packages_resolved,
                bumps: &[],
                rows: &run.slot_reports,
                delegation: run.parked.as_ref(),
            },
        );
    }

    // Scoped update: only the named packages and their subtrees move.
    let project_root = resolve_project_root(&args.path)?;
    let workspace = Workspace::discover(&project_root)
        .context("discovering the workspace enclosing the project")?;
    let manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_lockfile(&workspace.root)?;
    // PROP-050 ##VERIFY-LOCK-DIFF — the scoped update's own pre-apply
    // snapshot: the lock as loaded (before the entry replacements below
    // mutate it in place) and the boot lanes' byte sizes. Diffed against
    // the post-write state once the apply is durable.
    let old_lock = lockfile.clone();
    let lanes_before = lane_sizes(&workspace.root);
    let user_config = UserConfig::load().context("loading the user config")?;
    let spec_format = crate::commands::install::resolve_spec_format(&manifest, &user_config);

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` configured in `{}/vibe.toml` — `vibe update` re-fetches \
             from the registry.",
            project_root.display()
        );
    }

    // Each named package must already be installed; re-resolve it against
    // its original root constraint so a caret bumps within range. `vibe
    // update` refreshes an installed package, so a bare short name resolves
    // against `vibe.lock` alone — no index, no network — and a name not
    // locked fails here with a clear "not installed".
    let mut roots: Vec<PackageRef> = Vec::with_capacity(args.packages.len());
    for raw in &args.packages {
        let pkgref = PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
        let pkgref = short_name::qualify_locked(&pkgref, &lockfile)?;
        let group = match pkgref.group.as_ref() {
            Some(g) => g.clone(),
            None => bail!(
                "`{pkgref}` resolved without a group — internal: `qualify_locked` should qualify"
            ),
        };
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

    // PROP-010 §2.5 — the offline posture reaches `vibe update` through
    // the same ladder as install: root `--offline` / `VIBE_OFFLINE` /
    // user-config `[net].offline` (resolved via [`output::resolve_offline`]).
    // `vibe update` carries no local `--offline` flag of its own, so the
    // CLI rung here is the root flag alone. A scoped offline update with no
    // local source fails in `build_install_resolver` with the same
    // actionable bail `vibe install --offline` gives.
    let global = vibe_core::GlobalRegistryConfig::load()?;
    let offline = output::resolve_offline(root_offline, user_config.net.offline);
    let resolver = build_install_resolver(
        &install_args_from(&args),
        &manifest,
        embedded_root.as_deref(),
        &project_root,
        &global,
        offline,
        &lockfile.packages,
    )?;

    ctx.heading(&format!(
        "Re-resolving {} package{}…",
        roots.len(),
        if roots.len() == 1 { "" } else { "s" },
    ));
    let graph = resolver
        .solve(&roots)
        .context("dependency resolution failed")?;

    // Fetched payload lands in the machine-global store (PROP-010
    // §2.7) — no project cache directory exists to create.
    let store_root =
        vibe_registry::store::store_root().context("resolving the machine package store root")?;

    // Fetch every node of the named subtree. A package the lockfile already
    // records as in-place with a present slot is NOT re-fetched: it is updated
    // incrementally on its own `.git` (PROP-022 §2.4) — a version bump on a
    // giant transfers only changed objects rather than re-cloning the tree. We
    // resolve those nodes here but defer the slot mutation past the confirm.
    let mut updated: Vec<(CachedPackage, Vec<PackageRef>, Option<bool>)> = Vec::new();
    let mut pending_in_place: Vec<PendingInPlace> = Vec::new();
    for node in graph.iter() {
        let pkgref = exact_pinned_pkgref(node);
        if let Some(old) = lockfile.find(&node.group, &node.name)
            && old.materialization.is_in_place()
            && vibedeps::is_in_place_slot(&workspace.root, &old.group, &node.name)
        {
            pending_in_place.push(PendingInPlace {
                pkgref,
                group: node.group.clone(),
                name: node.name.clone(),
                version: node.version.clone(),
                registry: old.registry.clone(),
                dependencies: node.dependencies.clone(),
            });
            continue;
        }
        let cached = resolver.resolve_and_fetch(&pkgref, &store_root, None)?;
        updated.push((cached, node.dependencies.clone(), None));
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
        let slot = vibedeps::in_place_slot_abs_path(&workspace.root, &p.group, &p.name);
        let placed = resolver
            .materialise_in_place(&p.pkgref, &slot)
            .with_context(|| format!("updating in-place `{}/{}`", p.group, p.name))?;
        vibedeps::ensure_gitignored(
            &workspace.root,
            &vibedeps::in_place_slot_rel_path(&p.group, &p.name),
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
            is_local: false,
            via_redirect: None,
        };
        updated.push((cached, p.dependencies, Some(placed.changed)));
    }

    // Build the partial resolution for the subtree — the form the shared
    // materialise + hook flow consumes (the same `ResolvedDep` shape
    // `vibe install` hands to `apply_resolution`).
    let resolution: Vec<ResolvedDep> = updated
        .iter()
        .map(|(cached, deps, in_place_changed)| ResolvedDep {
            kind: cached.package_meta().kind,
            group: cached.resolved.group.clone(),
            name: cached.resolved.name.clone(),
            version: cached.resolved.version.clone(),
            content_dir: cached.cache_dir.clone(),
            source_hash: Some(ContentHash::from_validated(cached.content_hash.clone())),
            manifest: cached.manifest.clone(),
            requires: deps
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                .collect(),
            admitted_by: None,
            via_override: None,
            // Mutable iff an in-workspace `file://` self-hosting source the
            // author edits in place (PROP-011 §2.6); recorded so the materialise
            // pass re-copies its slot.
            source_mutable: vibe_workspace::freshness::is_in_workspace_file_source(
                &cached.source_uri,
                &workspace.root,
            ),
            in_place_changed: *in_place_changed,
        })
        .collect();

    // Remove any superseded *versioned* slot so a bump leaves no stale slot
    // (an in-place slot is unversioned — nothing to prune), and record the
    // bumps for the report.
    //
    // `pruned` is measured HERE, at the removal itself, and only when the
    // removal actually removed something: `bumps` is prose for a human, and a
    // bump whose slot was already absent (or is in-place, so unversioned)
    // pruned nothing. Deriving one from the other would report a slot path
    // that no longer describes anything this run did.
    let mut bumps: Vec<String> = Vec::new();
    let mut pruned: Vec<String> = Vec::new();
    for (cached, _, _) in &updated {
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
            let group = &cached.package_meta().group;
            let removed = vibedeps::remove_slot(&workspace.root, group, name, &old_v)
                .context("removing the superseded vibedeps/ slot")?;
            if removed {
                pruned.push(vibedeps::slot_rel_path(group, name, &old_v));
            }
        }
    }
    pruned.sort();

    // Materialise the subtree (copy / hardlink / in-place move) and
    // run each freshly-placed slot's pre-install hook (PROP-020 §2.1) — no
    // prune, no boot here; boot is regenerated below from the whole tree.
    // `Verify` re-materialises every named slot from the fresh fetch.
    let source_hashes = SourceHashes(
        updated
            .iter()
            .map(|(cached, _, _)| {
                (
                    (cached.resolved.group.clone(), cached.resolved.name.clone()),
                    cached.content_hash.clone(),
                )
            })
            .collect(),
    );
    let lifecycle_metadata = lifecycle::metadata(
        ctx,
        &project_root,
        "update",
        root_offline || user_config.net.offline,
        args.assume_yes,
    )?;
    let lifecycle_observer =
        crate::commands::install::LifecycleSlotObserver::new(ctx, lifecycle_metadata.clone());
    let provisional_world = lifecycle::provisional_world(&workspace, &lockfile, &resolution)?;
    let lifecycle = InstallSlotLifecycle::from_projection_observed(
        &project_root,
        &manifest,
        &provisional_world,
        &resolution,
        lifecycle_metadata.clone(),
        lifecycle::stream_mode(ctx),
        vibe_install::SlotLifecycleSeams {
            observer: std::sync::Arc::new(lifecycle_observer),
            agent: std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend(
                &project_root,
            )?),
        },
    )?;
    // The prune already happened; a park inside the materialise pass must
    // still report it. Every later `lifecycle.progress()` inherits it.
    lifecycle.record_pruned(pruned.clone());
    let materialised = materialise_subtree_with_spec_format_and_slot_lifecycle(
        &workspace.root,
        &resolution,
        SlotIntegrity::Verify,
        spec_format,
        Some(&source_hashes),
        &lifecycle,
    );
    // A hosted row parked. That is a durable handoff, not a failure — and it
    // belongs to THIS command: `vibe update` reports `update`, its own run,
    // and `resume: vibe update`. It never impersonates install.
    if let Some(delegation) = lifecycle.parked() {
        crate::commands::lifecycle::check_delegation(&delegation)?;
        return emit_update_document(
            ctx,
            report::UpdateOutcome {
                project_root: &project_root,
                args: &args,
                progress: &lifecycle.progress(),
                packages_resolved: updated.len(),
                bumps: &bumps,
                rows: &lifecycle.take_reports().unwrap_or_default(),
                delegation: Some(&delegation),
            },
        );
    }
    let mut subtree = materialised.context("re-materialising the updated subtree")?;

    // Regenerate every node's boot from the new `vibedeps/` state.
    let nodes_regenerated = regenerate_boot_with_spec_format(&workspace, spec_format)
        .context("regenerating boot artifacts")?;

    // The scoped update's own complete record, assembled from what each step
    // really returned: the subtree pass's slot lists, the removals measured
    // above, and the nodes boot regeneration actually rewrote. Recorded on the
    // lifecycle BEFORE the post-install callbacks, so a row that parks there
    // reports a finished materialisation instead of the partial snapshot the
    // materialise boundary left behind.
    lifecycle.record_complete(vibe_install::InstallProgress {
        complete: true,
        fresh: false,
        materialised: subtree.materialised.clone(),
        skipped: subtree.skipped.clone(),
        pruned,
        nodes_regenerated,
    });

    // Replace each subtree package's lockfile entry, carrying the
    // install-scoped metadata (features / language) the version bump does
    // not change.
    for (cached, deps, _) in &updated {
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

    // PROP-050 ##VERIFY-LOCK-DIFF — the closure diff after the apply is
    // durable (lock written, boot regenerated): entering/leaving members,
    // version moves, and the lane byte delta against the pre-apply
    // snapshot. Ahead of the post-install hooks and the report, mirroring
    // the `vibe install` emit order.
    emit_closure_diff(
        ctx,
        "update",
        &old_lock,
        &lockfile,
        &lanes_before,
        &lane_sizes(&workspace.root),
    );

    if let Some(plan) = subtree.take_post_install_plan() {
        let ran = run_post_install_slot_lifecycle(plan, SlotLifecycleMode::Callback(&lifecycle));
        if let Some(delegation) = lifecycle.parked() {
            crate::commands::lifecycle::check_delegation(&delegation)?;
            return emit_update_document(
                ctx,
                report::UpdateOutcome {
                    project_root: &project_root,
                    args: &args,
                    progress: &lifecycle.progress(),
                    packages_resolved: updated.len(),
                    bumps: &bumps,
                    rows: &lifecycle.take_reports().unwrap_or_default(),
                    delegation: Some(&delegation),
                },
            );
        }
        ran.context("running post-install lifecycle")?;
    }
    // A scoped update whose slot is already materialised raises no payload
    // event, so its post-install pass never revisits a live park. The
    // persisted continuation is exactly the mechanism for that: service it
    // before anything reports a completed update.
    if let Some(done) = report::service_continuation(
        ctx,
        &lifecycle,
        &project_root,
        &workspace,
        &manifest,
        &lifecycle_metadata,
        spec_format,
        &args,
        updated.len(),
        &bumps,
    )? {
        return done;
    }
    lifecycle.clear_continuation().map_err(anyhow::Error::msg)?;
    let lifecycle_reports = lifecycle.take_reports()?;
    let hook_reports = LifecycleHookView::new(&lifecycle_reports);

    emit_report(
        ctx,
        report::UpdateOutcome {
            project_root: &project_root,
            args: &args,
            progress: &lifecycle.progress(),
            packages_resolved: updated.len(),
            bumps: &bumps,
            rows: &lifecycle_reports,
            delegation: None,
        },
        &hook_reports,
    )?;
    Ok(())
}
