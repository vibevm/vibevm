//! Install adapter for legacy `[hooks]` sugar over the lifecycle handler engine.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT");

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use vibe_core::PackageName;
use vibe_core::lifecycle::{ExtensionPoint, SlotPoint};
use vibe_core::manifest::{Manifest, Materialization};
use vibe_lifecycle::handlers::{BinaryBackend, HandlerRuntime, HandlerStreams};
use vibe_lifecycle::process::{StreamMode, SystemProcessRunner};
use vibe_lifecycle::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, DispatchError,
    ExecutionReuse, ExtensionProvider, HandlerExecution, HostExtensionSource, HostIdentity,
    HostProvider, LifecycleRun, LifecycleRunError, LifecycleRunHandle, Phase, RunMetadata,
    SlotTarget, inclusive_chain,
};
use vibe_wire::generated::lifecycle::e1::context::{Project, World, WorldPackage};
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;
use vibe_workspace::Workspace;
use vibe_workspace::hooks::SystemProbe;
use vibe_workspace::install::{
    ResolvedDep, SlotLifecycle, SlotLifecycleContext, SlotLifecycleTarget,
};
use vibe_workspace::vibedeps::{in_place_slot_abs_path, slot_abs_path};

use crate::error::{Error, Result};
use crate::plan::PlannedInstall;

mod plan;
use plan::build_slot_plan;
pub use plan::{
    NoSlotLifecycleObserver, SlotLifecycleObserver, SlotLifecyclePlan, SlotLifecyclePlanEntry,
};

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

pub struct InstallSlotLifecycle {
    installed: Vec<DependencyExtensionSource>,
    plan: SlotLifecyclePlan,
    streams: StreamMode,
    run: LifecycleRunHandle,
    reports: Mutex<Vec<SlotLifecycleReport>>,
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
        Ok(Self {
            installed,
            plan,
            streams,
            run,
            reports: Mutex::new(Vec::new()),
            observer: seams.observer,
            agent: seams.agent,
        })
    }

    pub(crate) fn run_handle(&self) -> LifecycleRunHandle {
        self.run.clone()
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
            Ok(outcome) => self.push_report(SlotLifecycleReport {
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
            }),
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
        self.observer.observe(&self.plan.for_targets(targets))
    }

    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> std::result::Result<(), String> {
        self.dispatch(context, SlotPoint::PreInstall)
    }

    fn post_install(&self, context: SlotLifecycleContext<'_>) -> std::result::Result<(), String> {
        self.dispatch(context, SlotPoint::PostInstall)
    }
}

fn dependency_provider(workspace_root: &Path, dep: &ResolvedDep) -> Result<DependencyProvider> {
    let root = if dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|package| package.materialization == Materialization::InPlace)
    {
        in_place_slot_abs_path(workspace_root, &dep.group, &dep.name)
    } else {
        slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version)
    };
    Ok(DependencyProvider {
        id: DependencyProviderId::new(dep.group.clone(), PackageName::parse(&dep.name)?),
        root,
        version: dep.version.to_string(),
        kind: dep.kind,
        content_hash: dep.source_hash.clone().ok_or_else(|| {
            Error::Lifecycle(format!(
                "planned dependency `{}/{}@{}` has no source hash",
                dep.group, dep.name, dep.version,
            ))
        })?,
    })
}

fn project_envelope(root: &Path, manifest: &Manifest) -> Project {
    let (name, version, kind) = if let Some(package) = &manifest.package {
        (
            package.name.clone(),
            package.version.to_string(),
            package.kind.as_str().to_string(),
        )
    } else if let Some(project) = &manifest.project {
        (
            project.name.clone(),
            project.version.clone(),
            "project".into(),
        )
    } else {
        (
            "<virtual-workspace>".into(),
            String::new(),
            "workspace".into(),
        )
    };
    Project {
        kind,
        manifest: vibe_core::machine_json_path(&root.join(Manifest::FILENAME)),
        name,
        root: vibe_core::machine_json_path(root),
        spec_roots: vec![vibe_core::machine_json_path(
            &root.join(vibe_core::layout::current_specs_root()),
        )],
        version,
    }
}

fn host_source(root: &Path, manifest: &Manifest) -> Result<HostExtensionSource> {
    let (identity, version, kind) = if let Some(package) = &manifest.package {
        (
            HostIdentity::coordinate(DependencyProviderId::new(
                package.group.clone(),
                PackageName::parse(&package.name)?,
            )),
            package.version.to_string(),
            Some(package.kind),
        )
    } else if let Some(project) = &manifest.project {
        let identity = match &project.group {
            Some(group) => HostIdentity::coordinate(DependencyProviderId::new(
                group.clone(),
                PackageName::parse(&project.name)?,
            )),
            None => HostIdentity::ungrouped_project(project.name.clone()),
        };
        (identity, project.version.clone(), None)
    } else {
        (HostIdentity::virtual_workspace(), String::new(), None)
    };
    Ok(HostExtensionSource {
        provider: HostProvider {
            identity,
            root: root.to_path_buf(),
            version,
            kind,
            content_hash: None,
        },
        declarations: manifest.extensions.clone(),
        controls: manifest.extension_controls.clone(),
    })
}

fn world_envelope(workspace: &Workspace, resolution: &[ResolvedDep]) -> World {
    World {
        deps_root: vibe_core::machine_json_path(&workspace.vibedeps_root()),
        lockfile: vibe_core::machine_json_path(&workspace.lockfile_path()),
        packages: resolution
            .iter()
            .map(|dep| WorldPackage {
                group: dep.group.to_string(),
                kind: dep.kind.as_str().to_string(),
                name: dep.name.clone(),
                slot: vibe_core::machine_json_path(&dependency_slot(&workspace.root, dep)),
                version: dep.version.to_string(),
            })
            .collect(),
    }
}

fn dependency_slot(workspace_root: &Path, dep: &ResolvedDep) -> PathBuf {
    if dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|package| package.materialization == Materialization::InPlace)
    {
        in_place_slot_abs_path(workspace_root, &dep.group, &dep.name)
    } else {
        slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version)
    }
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn slot_target(context: SlotLifecycleContext<'_>) -> SlotTarget {
    SlotTarget {
        group: context.group.to_string(),
        name: context.name.to_string(),
        version: context.version.to_string(),
        kind: context.kind.to_string(),
        root: vibe_core::machine_json_path(context.slot),
    }
}
