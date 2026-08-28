//! The staging region of a scoped `vibe update`: the first durable mutation,
//! and the run that will own everything after it.
//!
//! Two things happen here and they are ordered on purpose. The deferred
//! in-place updates run first — each one `git fetch`-es a slot onto its own
//! working tree, which is irreversible — and only then is the resolution
//! completed, the provisional world built and the slot lifecycle constructed.
//!
//! That ordering is why the accumulator exists. Between the first fetch and the
//! lifecycle there is no run to record anything, so every mutation is written
//! into [`Measured`] AS IT HAPPENS rather than after the loop: a fetch that
//! succeeds followed by one that does not must leave the first one reported.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use anyhow::{Context, Result};
use std::path::Path;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{ContentHash, Group, PackageRef};
use vibe_install::{InstallSlotLifecycle, InstallSource};
use vibe_lifecycle::{LifecycleLease, RunMetadata};
use vibe_registry::{CachedPackage, ResolvedPackage};
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::vibedeps;

use crate::commands::install::{InstallResolver, LifecycleSlotObserver};
use crate::output;

use super::super::lifecycle;
use super::measured::Measured;
use super::{Resolved, SourceHashes};

/// A subtree node the scoped update will refresh **in place** rather than
/// re-fetch: the lockfile already records it as `in-place` (PROP-022 §2.4) and
/// its slot is present, so it is `git fetch`-ed onto its own `.git` after
/// confirmation instead of re-cloned.
pub(super) struct PendingInPlace {
    pub(super) pkgref: PackageRef,
    pub(super) group: Group,
    pub(super) name: String,
    pub(super) version: semver::Version,
    pub(super) registry: Option<String>,
    pub(super) dependencies: Vec<PackageRef>,
}

/// What the staging region produced: the run, and the world it runs over.
pub(super) struct Staged {
    pub(super) lifecycle: InstallSlotLifecycle,
    pub(super) resolution: Vec<ResolvedDep>,
    pub(super) source_hashes: SourceHashes,
    pub(super) updated: Vec<Resolved>,
}

pub(super) struct Stage<'a> {
    pub(super) resolver: &'a InstallResolver,
    pub(super) workspace: &'a Workspace,
    pub(super) project_root: &'a Path,
    pub(super) manifest: &'a Manifest,
    pub(super) lockfile: &'a Lockfile,
    pub(super) metadata: &'a RunMetadata,
    /// The command's mutation lease: the slot run staged here is built on
    /// the ONE acquisition the update boundary made.
    pub(super) lease: &'a std::sync::Arc<LifecycleLease>,
    pub(super) updated: Vec<Resolved>,
    pub(super) pending_in_place: Vec<PendingInPlace>,
}

/// Perform the deferred in-place updates and build the run they belong to.
///
/// Every mutation here is recorded in `measured` AS IT HAPPENS, not after the
/// loop: a fetch that succeeds and a next one that does not must leave the
/// first one reported.
pub(super) fn stage(
    ctx: &output::Context,
    measured: &mut Measured,
    inputs: Stage<'_>,
) -> Result<Staged> {
    let Stage {
        resolver,
        workspace,
        project_root,
        manifest,
        lockfile,
        metadata,
        lease,
        mut updated,
        pending_in_place,
    } = inputs;
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
        // The slot's working tree has now really moved. Recorded before the
        // gitignore write below, because it is already true regardless of it.
        measured.record_in_place(
            vibedeps::in_place_slot_rel_path(&p.group, &p.name),
            placed.changed,
        );
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
    let provisional_world = vibe_orchestrator::provisional_world(workspace, lockfile, &resolution)?;
    // The PREPARED constructor: this command already owns the tree its identity
    // and its trace were selected against, so the wrapper's own discovery would
    // be a second read of a tree this run has already started changing.
    let slot_lifecycle = InstallSlotLifecycle::from_projection_observed_prepared(
        project_root,
        manifest,
        &provisional_world,
        &resolution,
        workspace,
        metadata.clone(),
        lifecycle::stream_mode(ctx),
        vibe_install::SlotLifecycleSeams {
            observer: std::sync::Arc::new(LifecycleSlotObserver::new(ctx, metadata.clone())),
            // Built from the values this command already holds — no
            // discovery, no second manifest read.
            agent: std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend(
                &workspace.root,
                manifest,
            )),
        },
        lease.clone(),
    )?;
    Ok(Staged {
        lifecycle: slot_lifecycle,
        resolution,
        source_hashes,
        updated,
    })
}
