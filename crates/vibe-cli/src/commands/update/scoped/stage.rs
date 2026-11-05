//! The staging region of a scoped `vibe update`: the first checkout mutation,
//! and the run that will own everything after it.
//!
//! Every candidate and the complete provider world were admitted before this
//! region. Bound in-place preparations run first; the accumulator records each
//! successful checkout before lifecycle construction and later materialization.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use anyhow::{Context, Result};
use std::path::Path;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{ContentHash, Group, PackageRef};
use vibe_install::InstallSlotLifecycle;
use vibe_lifecycle::{LifecycleLease, RunMetadata};
use vibe_registry::{CachedPackage, ResolvedPackage};
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::vibedeps;

use crate::commands::install::LifecycleSlotObserver;
use crate::output;

use super::super::lifecycle;
use super::measured::Measured;
use super::{Resolved, SourceHashes};

/// A subtree node the scoped update will refresh **in place** rather than
/// re-fetch: the lockfile already records it as `in-place` (PROP-022 §2.4) and
/// its slot is present. The admitted immutable preparation is applied after
/// confirmation without another source/ref selection.
pub(super) struct PendingInPlace {
    pub(super) prepared: vibe_registry::PreparedInPlace,
    pub(super) pkgref: PackageRef,
    pub(super) group: Group,
    pub(super) name: String,
    pub(super) version: semver::Version,
    pub(super) registry: Option<String>,
    pub(super) dependencies: Vec<PackageRef>,
    pub(super) embedded_sources: Vec<vibe_core::manifest::LockedEmbeddedSource>,
}

/// What the staging region produced: the run, and the world it runs over.
pub(super) struct Staged {
    pub(super) lifecycle: InstallSlotLifecycle,
    /// The complete post-update epoch: unchanged durable rows with this
    /// scoped subtree's replacements substituted in lock order.
    pub(super) full_world: Vec<ResolvedDep>,
    /// Only the re-resolved subtree that materialisation and lock replacement
    /// consume.
    pub(super) resolution: Vec<ResolvedDep>,
    pub(super) source_hashes: SourceHashes,
    pub(super) updated: Vec<Resolved>,
}

pub(super) struct Stage<'a> {
    pub(super) workspace: &'a Workspace,
    pub(super) project_root: &'a Path,
    pub(super) manifest: &'a Manifest,
    pub(super) metadata: &'a RunMetadata,
    /// The command's mutation lease: the slot run staged here is built on
    /// the ONE acquisition the update boundary made.
    pub(super) lease: &'a std::sync::Arc<LifecycleLease>,
    pub(super) updated: Vec<Resolved>,
    pub(super) pending_in_place: Vec<PendingInPlace>,
    pub(super) full_world: Vec<ResolvedDep>,
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
        workspace,
        project_root,
        manifest,
        metadata,
        lease,
        mut updated,
        pending_in_place,
        mut full_world,
    } = inputs;
    // Confirmed — perform the deferred incremental in-place updates, then fold
    // each into the same `updated` set so the resolution / lockfile / hook flow
    // treats it uniformly. The built `CachedPackage`'s `cache_dir` IS the slot,
    // which signals "already placed" to the materialise pass (it runs the hook
    // but skips any move).
    let in_place_total = pending_in_place.len();
    let in_place = ctx.progress().task("Refreshing in-place packages");
    in_place.set_progress(0, Some(in_place_total as u64), "packages");
    for (index, p) in pending_in_place.into_iter().enumerate() {
        let package = in_place
            .progress()
            .task(format!("Refreshing {}/{}", p.group, p.name));
        vibe_install::configuration_admission::package_manifest(&p.pkgref, p.prepared.manifest())?;
        let slot = p.prepared.slot().to_path_buf();
        let placed = match p
            .prepared
            .materialise()
            .with_context(|| format!("updating in-place `{}/{}`", p.group, p.name))
        {
            Ok(placed) => placed,
            Err(error) => {
                package.fail("in-place refresh failed");
                in_place.fail("in-place package refresh failed");
                return Err(error);
            }
        };
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
            // An in-place slot is placed through the same clone chain as
            // any other fetch; which candidate served it is recorded by
            // that chain, not re-derived here (`PROP-066#provenance`).
            fetched_from: None,
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
        updated.push((
            cached,
            p.dependencies,
            Some(placed.changed),
            p.embedded_sources,
        ));
        package.finish();
        in_place.set_progress((index + 1) as u64, Some(in_place_total as u64), "packages");
    }
    if in_place_total == 0 {
        in_place.skip("no in-place packages selected");
    } else {
        in_place.finish();
    }

    // Build the partial resolution for the subtree — the form the shared
    // materialise + hook flow consumes (the same `ResolvedDep` shape
    // `vibe install` hands to `apply_resolution`).
    let resolution = project_rows(workspace, &updated);

    let source_hashes = SourceHashes(
        updated
            .iter()
            .map(|(cached, _, _, _)| {
                (
                    (cached.resolved.group.clone(), cached.resolved.name.clone()),
                    cached.content_hash.clone(),
                )
            })
            .collect(),
    );
    for replacement in &resolution {
        if let Some(row) = full_world
            .iter_mut()
            .find(|row| row.group == replacement.group && row.name == replacement.name)
        {
            *row = replacement.clone();
        }
    }
    vibe_install::configuration_admission::resolved_packages(&full_world)?;
    // The PREPARED constructor: this command already owns the tree its identity
    // and its trace were selected against, so the wrapper's own discovery would
    // be a second read of a tree this run has already started changing.
    let slot_lifecycle = InstallSlotLifecycle::from_projection_observed_prepared(
        project_root,
        manifest,
        &full_world,
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
        full_world,
        resolution,
        source_hashes,
        updated,
    })
}

/// Read the full provider world before any selected checkout is advanced.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub(super) fn preflight_world(
    workspace: &Workspace,
    lockfile: &Lockfile,
    updated: &[Resolved],
    pending: &[PendingInPlace],
) -> Result<Vec<ResolvedDep>> {
    let mut selected = project_rows(workspace, updated);
    for candidate in pending {
        let manifest = candidate.prepared.manifest();
        let package = manifest
            .package
            .as_ref()
            .context("prepared in-place source lacks package metadata")?;
        selected.push(ResolvedDep {
            kind: package.kind,
            group: candidate.group.clone(),
            name: candidate.name.clone(),
            version: candidate.version.clone(),
            content_dir: candidate.prepared.slot().to_path_buf(),
            source_hash: Some(ContentHash::from_validated(
                candidate.prepared.content_hash().to_owned(),
            )),
            embedded_sources: candidate.embedded_sources.clone(),
            manifest: manifest.clone(),
            requires: candidate
                .dependencies
                .iter()
                .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                .collect(),
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        });
    }
    let world = vibe_orchestrator::provisional_world(workspace, lockfile, &selected)?;
    vibe_install::configuration_admission::resolved_packages(&world)?;
    Ok(world)
}
fn project_rows(workspace: &Workspace, updated: &[Resolved]) -> Vec<ResolvedDep> {
    updated
        .iter()
        .map(
            |(cached, deps, in_place_changed, embedded_sources)| ResolvedDep {
                kind: cached.package_meta().kind,
                group: cached.resolved.group.clone(),
                name: cached.resolved.name.clone(),
                version: cached.resolved.version.clone(),
                content_dir: cached.cache_dir.clone(),
                source_hash: Some(ContentHash::from_validated(cached.content_hash.clone())),
                embedded_sources: embedded_sources.clone(),
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
            },
        )
        .collect()
}
