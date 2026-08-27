//! Install adapter for legacy `[hooks]` sugar over the lifecycle handler engine.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT");

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use vibe_core::PackageName;
use vibe_core::lifecycle::{ExtensionPoint, SlotPoint};
use vibe_core::manifest::Manifest;
use vibe_lifecycle::handlers::{BinaryBackend, HandlerRuntime, HandlerStreams};
use vibe_lifecycle::process::{StreamMode, SystemProcessRunner};
use vibe_lifecycle::{
    Delegation, DependencyExtensionSource, DependencyProviderId, DispatchError, ExecutionReuse,
    ExtensionProvider, HandlerExecution, HostIdentity, LifecycleRun, LifecycleRunError,
    LifecycleRunHandle, Phase, RunMetadata, inclusive_chain,
};
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, SlotTargetRecord};
use vibe_workspace::Workspace;
use vibe_workspace::hooks::SystemProbe;
use vibe_workspace::install::{
    ResolvedDep, SlotLifecycle, SlotLifecycleContext, SlotLifecycleTarget,
};

use crate::error::{Error, Result};
use crate::plan::PlannedInstall;

mod envelope;
mod plan;
mod progress;
mod reconcile;
use envelope::{
    dependency_provider, host_source, nonempty, project_envelope, slot_target, world_envelope,
};
use plan::build_slot_plan;
pub use plan::{
    NoSlotLifecycleObserver, SlotLifecycleObserver, SlotLifecyclePlan, SlotLifecyclePlanEntry,
};
pub use progress::InstallProgress;
use reconcile::reconcile_removed_slot_parks;

/// One legacy hook after translation into and execution by the lifecycle engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotLifecycleReport {
    pub key: String,
    pub reference: String,
    pub slot_target: Option<vibe_wire::generated::lifecycle::e1::context::SlotTarget>,
    pub point: String,
    pub provider: String,
    pub handler: String,
    pub tier: String,
    pub version: Option<String>,
    pub status: String,
    pub flagged: bool,
    pub message: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

/// The sentinel a parked slot row returns through the `Result<(), String>`
/// slot-lifecycle seam. It is NOT a failure message: the caller recognises it,
/// reads the typed handoff off the lifecycle, and reports a durable handoff
/// with exit 0. Returning through the error channel is what STOPS the install
/// — materialisation of later slots, the lockfile barrier and every
/// post-install row — before any of it runs.
pub(crate) const PARKED_SENTINEL: &str = "@vibe/lifecycle/parked";

pub struct InstallSlotLifecycle {
    installed: Vec<DependencyExtensionSource>,
    plan: SlotLifecyclePlan,
    streams: StreamMode,
    run: LifecycleRunHandle,
    reports: Mutex<Vec<SlotLifecycleReport>>,
    /// The first hosted handoff this install parked, if any.
    parked: Mutex<Option<Delegation>>,
    /// What the materialise pass changed, recorded as it happened. A park in a
    /// deferred pre-install row reads this rather than an outcome that never
    /// came back.
    progress: Mutex<InstallProgress>,
    observer: Arc<dyn SlotLifecycleObserver>,
    /// `agent` is legal at slot points as well as phase points, so a direct or
    /// chained install must be able to execute one. It arrives through the
    /// required seams parameter — there is no defaulting path that could turn
    /// a selected contribution into a silent refusal.
    agent: Arc<dyn vibe_lifecycle::AgentBackend>,
}

impl InstallSlotLifecycle {
    pub(crate) fn from_plan_observed(
        planned: &PlannedInstall,
        run: RunMetadata,
        streams: StreamMode,
        seams: crate::SlotLifecycleSeams,
    ) -> Result<Self> {
        Self::from_resolution_observed(
            &planned.project_root,
            &planned.manifest,
            &planned.resolution,
            run,
            streams,
            seams,
        )
    }

    pub fn from_resolution_observed(
        project_root: &Path,
        manifest: &Manifest,
        resolution: &[ResolvedDep],
        run: RunMetadata,
        streams: StreamMode,
        seams: crate::SlotLifecycleSeams,
    ) -> Result<Self> {
        Self::from_projection_observed(
            project_root,
            manifest,
            resolution,
            resolution,
            run,
            streams,
            seams,
        )
    }

    /// `seams` is not optional, and [`crate::SlotLifecycleSeams`] has no
    /// `Default`: `agent` is legal at slot points, so a construction site that
    /// could *forget* the caller's backend would silently degrade a selected
    /// contribution to a refusal. Requiring the argument turns "every CLI path
    /// injects it" from a habit into a compile error.
    pub fn from_projection_observed(
        project_root: &Path,
        manifest: &Manifest,
        world_resolution: &[ResolvedDep],
        event_targets: &[ResolvedDep],
        run: RunMetadata,
        streams: StreamMode,
        seams: crate::SlotLifecycleSeams,
    ) -> Result<Self> {
        let workspace = Workspace::discover(project_root)?;
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
        let world = world_envelope(&workspace, world_resolution);
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
        let run = LifecycleRun::begin(&workspace.root, project, world, run, state_chain)
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

    pub(crate) fn run_handle(&self) -> LifecycleRunHandle {
        self.run.clone()
    }

    /// The typed handoff this install parked, if a hosted agent row stopped
    /// it. Present exactly when the slot seam returned [`PARKED_SENTINEL`].
    pub fn parked(&self) -> Option<Delegation> {
        self.parked.lock().ok().and_then(|parked| parked.clone())
    }

    /// Persist the exact ordered payload-event target set BEFORE any
    /// pre-install callback can park, so a resume whose lock is already fresh
    /// can rebuild the SAME slot run rather than infer one.
    fn record_targets(&self, targets: &[SlotLifecycleTarget]) -> std::result::Result<(), String> {
        let records = targets
            .iter()
            .map(|target| SlotTargetRecord {
                group: target.group.to_string(),
                name: target.name.clone(),
                version: target.version.to_string(),
            })
            .collect();
        self.run
            .lock()
            .map_err(|_| "slot lifecycle run lock was poisoned".to_string())?
            .record_slot_continuation(records)
            .map_err(|error| error.to_string())
    }

    /// Stage the exact ordered target set a RESUME rebuilt from the persisted
    /// continuation.
    ///
    /// The materialise pass announces its selection through `targets_ready`;
    /// a resume has no materialise pass — the lock is already fresh, so it
    /// reconstructs the set directly from what the parked run recorded — and
    /// therefore announces it here instead. Skipping this leaves the store
    /// servicing the resume with nothing staged, and a SECOND declared row
    /// parking after the first was satisfied then has no target set to name:
    /// the moment the last delegated row flips to `ok` the continuation is
    /// correctly dropped, and only the staged set can put it back.
    pub fn stage_resumed_targets(&self, targets: &[ResolvedDep]) -> Result<()> {
        let records = targets
            .iter()
            .map(|dep| SlotTargetRecord {
                group: dep.group.to_string(),
                name: dep.name.clone(),
                version: dep.version.to_string(),
            })
            .collect();
        self.run
            .lock()
            .map_err(|_| Error::Lifecycle("slot lifecycle run lock was poisoned".into()))?
            .record_slot_continuation(records)
            .map_err(|error| Error::Lifecycle(error.to_string()))
    }

    /// Whether a slot-scoped park is still live in this run's state.
    pub fn owes_slot_work(&self) -> bool {
        self.run
            .lock()
            .map(|run| run.owes_slot_work())
            .unwrap_or(false)
    }

    /// The slot run reached its end with nothing owed.
    pub fn clear_continuation(&self) -> std::result::Result<(), String> {
        self.run
            .lock()
            .map_err(|_| "slot lifecycle run lock was poisoned".to_string())?
            .clear_slot_continuation()
            .map_err(|error| error.to_string())
    }

    /// What the materialise pass had really changed when the seam stopped.
    pub fn progress(&self) -> InstallProgress {
        self.progress
            .lock()
            .map(|progress| progress.clone())
            .unwrap_or_default()
    }

    /// Replace the recorded progress with the complete record of a finished
    /// apply — used when the park happened AFTER the apply, at a post-install
    /// row, so the report carries the whole outcome rather than the
    /// pre-install snapshot.
    pub fn record_complete(&self, progress: InstallProgress) {
        if let Ok(mut slot) = self.progress.lock() {
            *slot = progress;
        }
    }

    /// Record slot paths a caller pruned OUTSIDE the materialise pass.
    ///
    /// A scoped update removes each superseded versioned slot itself, before
    /// materialising the subtree, so this is the only place those removals can
    /// be measured. Recording them here — at the removal, not derived from a
    /// human-facing bump list — means a park taken anywhere later still
    /// reports the slots that really went away.
    pub fn record_pruned(&self, pruned: Vec<String>) {
        if let Ok(mut slot) = self.progress.lock() {
            slot.pruned = pruned;
        }
    }

    pub fn take_reports(&self) -> Result<Vec<SlotLifecycleReport>> {
        let mut reports = self
            .reports
            .lock()
            .map_err(|_| Error::Lifecycle("slot lifecycle report lock was poisoned".into()))?;
        Ok(std::mem::take(&mut *reports))
    }

    fn dispatch(
        &self,
        context: SlotLifecycleContext<'_>,
        point: SlotPoint,
    ) -> std::result::Result<(), String> {
        let provider_id = DependencyProviderId::new(
            context.group.clone(),
            PackageName::parse(context.name).map_err(|error| error.to_string())?,
        );
        let target = slot_target(context);
        let mut matched = false;
        for planned in self.plan.entries.iter().filter(|planned| {
            planned.point == ExtensionPoint::Slot(point).to_string()
                && planned.slot_target.group == target.group
                && planned.slot_target.name == target.name
                && planned.slot_target.version == target.version
        }) {
            matched = true;
            self.dispatch_execution(planned.execution.clone(), point)?;
        }
        if !matched
            && !self
                .installed
                .iter()
                .any(|source| source.provider.id == provider_id)
        {
            return Err(format!(
                "slot lifecycle context `{}/{}@{}` is absent from the planned resolution",
                context.group, context.name, context.version,
            ));
        }
        Ok(())
    }

    fn dispatch_execution(
        &self,
        execution: HandlerExecution,
        point: SlotPoint,
    ) -> std::result::Result<(), String> {
        let mut run = self
            .run
            .lock()
            .map_err(|_| "slot lifecycle run lock was poisoned".to_string())?;
        let binary = WorkspaceBinaryBackend {
            output: self.streams,
        };
        let runtime = HandlerRuntime {
            process: &SystemProcessRunner,
            binary: &binary,
            package_binding: &vibe_lifecycle::NoPackageBindingBackend,
            agent: self.agent.as_ref(),
            probe: &SystemProbe,
            streams: self.streams,
        };
        let result = run.execute_one(&execution, "install", ExecutionReuse::Always, &runtime);
        drop(run);
        match result {
            Ok(outcome) => {
                let handoff = outcome.delegation.clone();
                self.push_report(SlotLifecycleReport {
                    key: execution.key(),
                    reference: execution.reference(),
                    slot_target: execution.slot_target().cloned(),
                    point: execution.row().declaration().point.to_string(),
                    provider: execution.row().provider().to_string(),
                    handler: execution.row().declaration().handler.kind().to_string(),
                    tier: tier_name(execution.row().effective_tier()).into(),
                    version: provider_version(execution.row().provider()),
                    status: transition_status(&outcome.status).into(),
                    flagged: false,
                    message: outcome.message,
                    stdout: nonempty(outcome.streams.stdout),
                    stderr: nonempty(outcome.streams.stderr),
                    stdout_truncated: outcome.streams.stdout_truncated,
                    stderr_truncated: outcome.streams.stderr_truncated,
                })?;
                // A parked hosted row stops the install HERE — before any
                // later slot is materialised, before the lockfile barrier and
                // before every post-install row. The sentinel travels through
                // the seam's error channel because that is the only channel
                // that halts the orchestrator; the caller reads the typed
                // handoff off `parked()` and reports a handoff, not a failure.
                if let Some(handoff) = handoff {
                    *self
                        .parked
                        .lock()
                        .map_err(|_| "slot lifecycle park lock was poisoned".to_string())? =
                        Some(handoff);
                    return Err(PARKED_SENTINEL.to_string());
                }
                Ok(())
            }
            Err(error) => {
                let soft_post_failure =
                    point == SlotPoint::PostInstall && is_semantic_handler_failure(&error);
                let streams = failure_streams(&error);
                self.push_report(SlotLifecycleReport {
                    key: execution.key(),
                    reference: execution.reference(),
                    slot_target: execution.slot_target().cloned(),
                    point: execution.row().declaration().point.to_string(),
                    provider: execution.row().provider().to_string(),
                    handler: execution.row().declaration().handler.kind().to_string(),
                    tier: tier_name(execution.row().effective_tier()).into(),
                    version: provider_version(execution.row().provider()),
                    status: "fail".into(),
                    flagged: soft_post_failure,
                    message: Some(error.to_string()),
                    stdout: nonempty(streams.stdout),
                    stderr: nonempty(streams.stderr),
                    stdout_truncated: streams.stdout_truncated,
                    stderr_truncated: streams.stderr_truncated,
                })?;
                if soft_post_failure {
                    Ok(())
                } else {
                    Err(error.to_string())
                }
            }
        }
    }

    fn push_report(&self, report: SlotLifecycleReport) -> std::result::Result<(), String> {
        self.observer.outcome(&report)?;
        self.reports
            .lock()
            .map_err(|_| "slot lifecycle report lock was poisoned".to_string())?
            .push(report);
        Ok(())
    }
}

struct WorkspaceBinaryBackend {
    output: StreamMode,
}

impl BinaryBackend for WorkspaceBinaryBackend {
    fn resolve_or_build(
        &self,
        row: &vibe_lifecycle::ExtensionRegistryRow,
        name: &str,
    ) -> std::result::Result<PathBuf, String> {
        let (binary, home) = match row.provider() {
            ExtensionProvider::Dependency(provider) => (
                vibe_workspace::bins::find_binary_in_provider_slot(
                    &provider.root,
                    provider.id.group(),
                    provider.id.name().as_str(),
                    &provider.version,
                    name,
                ),
                vibe_workspace::bins::BinaryProviderHome::InstalledSlot,
            ),
            ExtensionProvider::Host(provider) => {
                let HostIdentity::Coordinate(id) = &provider.identity else {
                    return Err("binary handler host must be a package-role coordinate".into());
                };
                if provider.kind.is_none() {
                    return Err("binary handler host must be an authored package root".into());
                }
                (
                    vibe_workspace::bins::find_binary_in_authored_package_root(
                        &provider.root,
                        id.group(),
                        id.name().as_str(),
                        &provider.version,
                        name,
                    ),
                    vibe_workspace::bins::BinaryProviderHome::AuthoredPackageRoot,
                )
            }
        };
        let binary = binary.map_err(|error| error.to_string())?;
        if !binary.artifact().exists() {
            vibe_workspace::bins::build_binary_authorized_with_output(
                &binary,
                vibe_workspace::bins::BuildAuthorization::InstalledExtension { home },
                if self.output == StreamMode::Inherit {
                    vibe_workspace::bins::BuildOutput::Inherit
                } else {
                    vibe_workspace::bins::BuildOutput::Quiet
                },
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(binary.artifact())
    }
}

fn is_semantic_handler_failure(error: &LifecycleRunError) -> bool {
    error.is_durable_soft_post()
}

fn failure_streams(error: &LifecycleRunError) -> HandlerStreams {
    error
        .failed_transition()
        .map(|transition| transition.streams.clone())
        .or_else(|| {
            error
                .dispatch_error()
                .and_then(DispatchError::streams)
                .cloned()
        })
        .unwrap_or_default()
}

fn transition_status(status: &ExecutionRecordStatus) -> &'static str {
    match status {
        ExecutionRecordStatus::Ok => "ok",
        ExecutionRecordStatus::Skip => "skip",
        ExecutionRecordStatus::Fresh => "fresh",
        ExecutionRecordStatus::Fail => "fail",
        ExecutionRecordStatus::Delegated => "delegated",
    }
}

fn tier_name(tier: vibe_lifecycle::ContributionTier) -> &'static str {
    match tier {
        vibe_lifecycle::ContributionTier::Dependency => "dependency",
        vibe_lifecycle::ContributionTier::Preset => "preset",
        vibe_lifecycle::ContributionTier::HostDeclaration => "host-declaration",
        vibe_lifecycle::ContributionTier::HostActivation => "host-activation",
    }
}

fn provider_version(provider: &ExtensionProvider) -> Option<String> {
    match provider {
        ExtensionProvider::Dependency(provider) => Some(provider.version.clone()),
        ExtensionProvider::Host(provider) => {
            (!provider.version.is_empty()).then(|| provider.version.clone())
        }
    }
}

impl SlotLifecycle for InstallSlotLifecycle {
    fn targets_ready(&self, targets: &[SlotLifecycleTarget]) -> std::result::Result<(), String> {
        self.record_targets(targets)?;
        self.observer.observe(&self.plan.for_targets(targets))
    }

    fn materialised(&self, materialised: &[String], skipped: &[String]) {
        if let Ok(mut progress) = self.progress.lock() {
            progress.materialised = materialised.to_vec();
            progress.skipped = skipped.to_vec();
            // Pruning and boot regeneration happen after this boundary, so a
            // record captured here is explicitly partial.
            progress.complete = false;
        }
    }

    fn rolled_back(&self, slot: &str) {
        if let Ok(mut progress) = self.progress.lock() {
            progress.materialised.retain(|entry| entry != slot);
        }
    }

    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> std::result::Result<(), String> {
        self.dispatch(context, SlotPoint::PreInstall)
    }

    fn post_install(&self, context: SlotLifecycleContext<'_>) -> std::result::Result<(), String> {
        self.dispatch(context, SlotPoint::PostInstall)
    }
}
