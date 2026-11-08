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
    pub(super) dependencies: Vec<PackageRef>,
    pub(super) embedded_sources: Vec<vibe_core::manifest::LockedEmbeddedSource>,
}

impl PendingInPlace {
    // Metadata-only: retained preparation owns the prospective bytes until stage.
    fn record_projection(&self) -> CachedPackage {
        let evidence = self.prepared.source_evidence();
        let source_kind = self.prepared.recorded_source_kind();
        CachedPackage {
            locked_selection: self.prepared.locked_selection().cloned(),
            route_publication: self.prepared.publication_scope(),
            recorded_source_kind: source_kind,
            store_reused: evidence.store_reused,
            acquisition: evidence.acquisition,
            resolved: ResolvedPackage {
                group: self.group.clone(),
                name: self.name.clone(),
                version: self.version.clone(),
                source_dir: self.prepared.slot().to_path_buf(),
            },
            cache_dir: self.prepared.slot().to_path_buf(),
            manifest: self.prepared.manifest().clone(),
            content_hash: self.prepared.content_hash().to_owned(),
            source_uri: self.prepared.source_uri().to_owned(),
            // An in-place slot is placed through the same clone chain as
            // any other fetch; which candidate served it is recorded by
            // that chain, not re-derived here (`PROP-066#provenance`).
            fetched_from: evidence.fetched_from,
            registry_name: evidence.registry,
            source_ref: Some(self.prepared.source_ref().to_owned()),
            resolved_commit: Some(self.prepared.resolved_commit().to_owned()),
            overridden: source_kind == Some(vibe_core::manifest::SourceKind::Override),
            is_git_source: source_kind == Some(vibe_core::manifest::SourceKind::Git),
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: evidence.via_redirect,
        }
    }
}
pub(super) fn prepare_records(
    capture: vibe_install::ProjectRecordCapture,
    base: &Lockfile,
    updated: &[Resolved],
    pending: &[PendingInPlace],
) -> Result<(Lockfile, vibe_install::ProjectRecordPublication)> {
    let mut prospective = updated.to_vec();
    prospective.extend(pending.iter().map(|pending| {
        (
            pending.record_projection(),
            pending.dependencies.clone(),
            None,
            pending.embedded_sources.clone(),
        )
    }));
    let mut planned = base.clone();
    for (cached, deps, _, embedded) in &prospective {
        let old = planned.find(&cached.resolved.group, &cached.resolved.name);
        let entry = super::super::inputs::locked_package(cached, deps, old, embedded);
        match planned
            .packages
            .iter()
            .position(|row| row.group == entry.group && row.name == entry.name)
        {
            Some(index) => planned.packages[index] = entry,
            None => planned.packages.push(entry),
        }
    }
    planned.record_app_requirements(|row| {
        prospective
            .iter()
            .map(|(cached, ..)| cached)
            .find(|cached| cached.resolved.group == row.group && cached.resolved.name == row.name)
            .map(|cached| &cached.manifest)
    });
    let records = capture.prepare(
        &planned,
        &prospective
            .iter()
            .map(|(cached, ..)| cached)
            .collect::<Vec<_>>(),
    )?;
    Ok((planned, records))
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
        if placed.changed {
            // Outside the apply's undo log: a later stop cannot honestly be
            // classified as not-started or restored.
            ctx.progress().cancellation().mark_unowned_effects();
        }
        vibedeps::ensure_gitignored(
            &workspace.root,
            &vibedeps::in_place_slot_rel_path(&p.group, &p.name),
        )
        .context("gitignoring the in-place slot")?;
        let cached = p.record_projection();
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
    )?
    // The invocation's `--stop-file`: hooks this run spawns are watched, and a
    // stop they request is acknowledged at the apply's checkpoints.
    .with_cancellation(ctx.progress().cancellation().clone());
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
            configuration: None,
        });
    }
    let world = vibe_orchestrator::provisional_world(workspace, lockfile, &selected)?;
    vibe_install::configuration_admission::resolved_packages(&world)?;
    Ok(world)
}
/// The resolved subtree as the materialise pass consumes it — pure, so the
/// no-op probe reads the same rows the apply would place.
pub(super) fn project_rows(workspace: &Workspace, updated: &[Resolved]) -> Vec<ResolvedDep> {
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
                configuration: None,
            },
        )
        .collect()
}
