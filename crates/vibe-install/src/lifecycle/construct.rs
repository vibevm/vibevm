//! How an [`InstallSlotLifecycle`] is BUILT — the prepared constructor and the
//! two compatibility wrappers that discover once and delegate to it.
//!
//! Split from the behaviour cell when the prepared sibling pushed the parent
//! past the file budget. The seam is a real one: everything here answers "what
//! world is this run over", and everything left in the parent answers "what
//! does the run do".

use std::path::Path;
use std::sync::Arc;

use vibe_core::manifest::Manifest;
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{LifecycleLease, LifecycleRun, Phase, RunMetadata, inclusive_chain};
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;

use std::sync::Mutex;

use super::envelope::{dependency_provider, host_source, project_envelope, world_envelope};
use super::plan::build_slot_plan;
use super::progress::InstallProgress;
use super::reconcile::reconcile_removed_slot_parks;
use super::{DependencyExtensionSource, InstallSlotLifecycle};
use crate::error::{Error, Result};
use crate::plan::PlannedInstall;

impl InstallSlotLifecycle {
    pub(crate) fn from_plan_observed(
        planned: &PlannedInstall,
        run: RunMetadata,
        streams: StreamMode,
        seams: crate::SlotLifecycleSeams,
        lease: Arc<LifecycleLease>,
    ) -> Result<Self> {
        // The plan's OWN workspace. This constructor runs inside apply, after
        // the plan already read the tree once; discovering again here would
        // re-read a tree the apply is midway through rewriting.
        Self::from_projection_observed_prepared(
            &planned.project_root,
            &planned.manifest,
            &planned.resolution,
            &planned.resolution,
            &planned.workspace,
            run,
            streams,
            seams,
            lease,
        )
    }

    /// Compatibility: discovers once, then delegates to the prepared sibling.
    pub fn from_resolution_observed(
        project_root: &Path,
        manifest: &Manifest,
        resolution: &[ResolvedDep],
        run: RunMetadata,
        streams: StreamMode,
        seams: crate::SlotLifecycleSeams,
        lease: Arc<LifecycleLease>,
    ) -> Result<Self> {
        Self::from_projection_observed(
            project_root,
            manifest,
            resolution,
            resolution,
            run,
            streams,
            seams,
            lease,
        )
    }

    /// `seams` is not optional, and [`crate::SlotLifecycleSeams`] has no
    /// `Default`: `agent` is legal at slot points, so a construction site that
    /// could *forget* the caller's backend would silently degrade a selected
    /// contribution to a refusal. Requiring the argument turns "every CLI path
    /// injects it" from a habit into a compile error.
    #[allow(clippy::too_many_arguments)]
    pub fn from_projection_observed(
        project_root: &Path,
        manifest: &Manifest,
        world_resolution: &[ResolvedDep],
        event_targets: &[ResolvedDep],
        run: RunMetadata,
        streams: StreamMode,
        seams: crate::SlotLifecycleSeams,
        lease: Arc<LifecycleLease>,
    ) -> Result<Self> {
        let workspace = Workspace::discover(project_root)?;
        Self::from_projection_observed_prepared(
            project_root,
            manifest,
            world_resolution,
            event_targets,
            &workspace,
            run,
            streams,
            seams,
            lease,
        )
    }

    /// The real constructor, over a workspace the caller ALREADY owns.
    ///
    /// It discovers nothing: the workspace root that anchors the lifecycle
    /// state and the slot lock, the node envelopes the world is built from,
    /// and the reconciliation of removed slot parks all read the supplied
    /// value. A caller holding the tree its own pass produced — apply's plan,
    /// or a resume's prepared workspace — reaches this directly; the two
    /// public wrappers above discover once and land here too.
    #[allow(clippy::too_many_arguments)]
    pub fn from_projection_observed_prepared(
        project_root: &Path,
        manifest: &Manifest,
        world_resolution: &[ResolvedDep],
        event_targets: &[ResolvedDep],
        workspace: &Workspace,
        run: RunMetadata,
        streams: StreamMode,
        seams: crate::SlotLifecycleSeams,
        lease: Arc<LifecycleLease>,
    ) -> Result<Self> {
        // The lease pins the workspace this constructor anchors state to. A
        // tree discovered under a DIFFERENT root than the one the command
        // leased would write state beside another process's lock — so a
        // disagreement is the typed lease refusal (carried by value through
        // `Error::Lease`, never re-stringified), never a second root answer.
        lease.ensure_root(&workspace.root, "at slot-lifecycle construction")?;
        let installed = world_resolution
            .iter()
            .map(|dep| {
                Ok(DependencyExtensionSource {
                    provider: dependency_provider(&workspace.root, dep)?,
                    declarations: dep.manifest.extensions.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let host = host_source(project_root, manifest)?;
        let plan = build_slot_plan(&installed, &host, event_targets)?;
        let project = project_envelope(project_root, manifest);
        let world = world_envelope(workspace, world_resolution);
        let state_chain = run
            .requested
            .parse::<Phase>()
            .map(|phase| {
                inclusive_chain(phase)
                    .iter()
                    .map(|phase| phase.to_string())
                    .collect()
            })
            .unwrap_or_else(|_| {
                run.chain
                    .iter()
                    .filter(|phase| phase.as_str() != "clean")
                    .cloned()
                    .collect()
            });
        let run = LifecycleRun::begin(lease, project, world, run, state_chain)
            .map_err(|error| Error::Lifecycle(error.to_string()))?
            .shared();
        // The slot half of the same reconciliation the phase plan performs at
        // its own boundary. This is the ONE place a slot plan is adopted, so
        // it is the one place that can notice a slot-scoped park whose
        // declaration is gone from the plan this run just built.
        let cancelled = reconcile_removed_slot_parks(&run, &plan, &workspace.root)?;
        Ok(Self {
            installed,
            plan,
            streams,
            run,
            reports: Mutex::new(cancelled),
            parked: Mutex::new(None),
            progress: Mutex::new(InstallProgress::default()),
            observer: seams.observer,
            agent: seams.agent,
        })
    }
}
