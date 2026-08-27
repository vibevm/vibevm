//! One-contribution lifecycle execution with per-transition checkpoints.

use anyhow::Result;
use vibe_lifecycle::handlers::{
    BinaryBackend, HandlerRuntime, PackageBindingArtifact, PackageBindingBackend,
    PackageBindingOutcome,
};
use vibe_lifecycle::process::{StreamMode, SystemProcessRunner};
use vibe_lifecycle::{
    Delegation, ExecutionReuse, HandlerExecution, LifecycleRun, LifecycleRunHandle, RunMetadata,
};
use vibe_lifecycle::{REMOVED_DECLARATION, UNKNOWN_PROVENANCE};
use vibe_wire::generated::lifecycle_report::LifecycleContributionReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordScope, ExecutionRecordStatus};

use crate::output;

use super::agent::CliAgentBackend;
use super::world;

/// What one dispatch pass produced: the contribution rows it reported and,
/// when a hosted agent row parked, the typed handoff plus the phase the chain
/// stopped at. A park is NOT a failure — it travels as a value so the caller
/// can truncate its step list and render one handoff.
#[derive(Debug, Default)]
pub(super) struct DispatchOutcome {
    pub reports: Vec<LifecycleContributionReport>,
    pub parked: Option<(String, Delegation)>,
}

pub(super) fn dispatch_plan_untracked(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    metadata: RunMetadata,
) -> Result<Vec<LifecycleContributionReport>> {
    let mut run =
        LifecycleRun::untracked(plan.project.clone(), plan.world.clone(), metadata.clone());
    let mut reports = Vec::with_capacity(plan.executions.len());
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let agent = CliAgentBackend::for_plan(plan);
    let runtime = runtime(ctx, &package_binding, &agent);
    for execution in plan.executions.iter() {
        let handler = HandlerExecution::from_row(&execution.row);
        let outcome =
            match run.execute_one(&handler, &execution.phase, ExecutionReuse::Always, &runtime) {
                Ok(outcome) => outcome,
                Err(error) => {
                    if let Some(failed) = error.failed_transition() {
                        reports.push(contribution_status_report(
                            execution,
                            "fail",
                            Some(failed.message.clone()),
                            Some(&failed.streams),
                        ));
                        super::emit_failure_outcome(ctx, &metadata, &execution.phase, &reports)?;
                    }
                    return Err(error.into());
                }
            };
        let report = contribution_status_report(
            execution,
            state_status(&outcome.status),
            outcome.message,
            Some(&outcome.streams),
        );
        render_outcome(ctx, &report);
        reports.push(report);
    }
    Ok(reports)
}

pub(super) fn dispatch_plan(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    metadata: RunMetadata,
    state_chain: Vec<String>,
) -> Result<DispatchOutcome> {
    let run = LifecycleRun::begin(
        &plan.workspace_root,
        plan.project.clone(),
        plan.world.clone(),
        metadata.clone(),
        state_chain,
    )?
    .shared();
    dispatch_plan_with_run(ctx, plan, &run, &metadata)
}

pub(super) fn dispatch_plan_with_run(
    ctx: &output::Context,
    plan: &world::RitualPlan,
    run: &LifecycleRunHandle,
    metadata: &RunMetadata,
) -> Result<DispatchOutcome> {
    let package_binding = ProjectPackageBindingBackend::new(plan);
    let agent = CliAgentBackend::for_plan(plan);
    let runtime = runtime(ctx, &package_binding, &agent);
    let mut outcome = DispatchOutcome {
        reports: Vec::with_capacity(plan.executions.len()),
        parked: None,
    };
    let mut run = run
        .lock()
        .map_err(|_| anyhow::anyhow!("lifecycle run lock was poisoned"))?;
    run.rebind_world(plan.project.clone(), plan.world.clone())?;
    reconcile_removed_parks(&mut run, plan, &mut outcome.reports)?;
    for execution in plan.executions.iter() {
        let handler = HandlerExecution::from_row(&execution.row);
        let transition = match run.execute_one(
            &handler,
            &execution.phase,
            ExecutionReuse::FreshnessAware,
            &runtime,
        ) {
            Ok(transition) => transition,
            Err(error) => {
                if let Some(failed) = error.failed_transition() {
                    outcome.reports.push(contribution_status_report(
                        execution,
                        "fail",
                        Some(failed.message.clone()),
                        Some(&failed.streams),
                    ));
                    super::emit_failure_outcome(ctx, metadata, &execution.phase, &outcome.reports)?;
                }
                return Err(anyhow::Error::new(error).context(format!(
                    "phase `{}` stopped before any later lifecycle contribution",
                    execution.phase
                )));
            }
        };
        let status = state_status(&transition.status);
        let fresh = transition.is_fresh();
        let parked = transition.delegation.clone();
        let report = contribution_status_report(
            execution,
            status,
            transition.message,
            (!fresh).then_some(&transition.streams),
        );
        render_outcome(ctx, &report);
        outcome.reports.push(report);
        // The first unsatisfied hosted row wins: every later contribution AND
        // every later phase of this plan is skipped. The rows are already in
        // chain order, so returning here stops both.
        if let Some(delegation) = parked {
            outcome.parked = Some((execution.phase.clone(), delegation));
            return Ok(outcome);
        }
    }
    if plan.package_phase_planned {
        let reconciled = outcome.reports.iter().any(|report| {
            report.key == world::PACKAGE_SKILL_RECONCILE_KEY
                && matches!(report.status.as_str(), "ok" | "fresh")
        });
        if reconciled {
            let mut keep = plan.package_desired_keys.clone();
            keep.insert(world::PACKAGE_SKILL_RECONCILE_KEY.to_string());
            keep.insert(world::PACKAGE_SKILL_RECOVER_KEY.to_string());
            run.retain_execution_prefix(vibe_mcp::pkgskill::PROJECT_SKILL_PREFIX, &keep)?;
        }
    }
    Ok(outcome)
}

/// Reconcile live PHASE-scoped parks against the COMPLETE current phase plan.
///
/// Same-id adoption deliberately retains delegated rows — that is how a resume
/// finds its own work. But if the declaration that parked one has since been
/// removed, the current plan never visits its key, so the row would sit live
/// forever while every later invocation reported a clean completion. That is
/// the one thing this must never do.
///
/// The POLICY is cancellation, chosen once and applied deterministically: the
/// row is removed by exact state-owned cleanup — recompute the `(run, key)`
/// task path, remove only that file, drop only that record, prune only a
/// proven-empty run directory — and the run continues, reporting the
/// cancellation as a contribution row rather than swallowing it. Refusing
/// instead would strand the operator on a declaration they already deleted.
///
/// Scope comes from the typed tag the engine recorded, never from parsing the
/// execution key or a task filename: a `slot`-scoped row belongs to the slot
/// plan and is invisible to this phase plan, so it is left alone here.
fn reconcile_removed_parks(
    run: &mut LifecycleRun,
    plan: &world::RitualPlan,
    reports: &mut Vec<LifecycleContributionReport>,
) -> Result<()> {
    let planned: std::collections::BTreeSet<String> = plan
        .executions
        .iter()
        .map(|execution| execution.row.key().to_string())
        .collect();
    let project_root = std::path::PathBuf::from(&plan.project.root);
    for (key, record) in run.delegated_rows() {
        if record.scope != Some(ExecutionRecordScope::Phase) || planned.contains(&key) {
            continue;
        }
        let Some(message) = run.cancel_delegated(&key, &project_root)? else {
            continue;
        };
        // Report ONLY what survives the declaration's removal. The row's own
        // persisted phase is authoritative, so the point is exact; everything
        // else — who provided it, at which tier, under what reference — died
        // with the declaration, and a host row that vanished never had a
        // `dependency` tier to begin with. Naming a sentinel is honest; naming
        // the first surviving execution's phase, or guessing `dependency`,
        // invents provenance the cancelled row cannot corroborate.
        reports.push(LifecycleContributionReport {
            flagged: None,
            handler: "agent".into(),
            key,
            message: Some(message),
            stderr: None,
            stderr_truncated: None,
            stdout: None,
            stdout_truncated: None,
            point: format!("phase:{}", record.phase),
            phase: record.phase,
            provider: REMOVED_DECLARATION.into(),
            reference: Some(REMOVED_DECLARATION.into()),
            slot_target: None,
            status: "cancelled".into(),
            tier: UNKNOWN_PROVENANCE.into(),
            version: None,
        });
    }
    Ok(())
}

fn state_status(status: &ExecutionRecordStatus) -> &'static str {
    match status {
        ExecutionRecordStatus::Ok => "ok",
        ExecutionRecordStatus::Skip => "skip",
        ExecutionRecordStatus::Fresh => "fresh",
        ExecutionRecordStatus::Fail => "fail",
        ExecutionRecordStatus::Delegated => "delegated",
    }
}

fn contribution_status_report(
    execution: &world::PlannedExecution,
    status: &str,
    message: Option<String>,
    streams: Option<&vibe_lifecycle::handlers::HandlerStreams>,
) -> LifecycleContributionReport {
    let row = &execution.row;
    let (provider, version) = super::provider_and_version(row.provider());
    LifecycleContributionReport {
        flagged: None,
        handler: row.declaration().handler.kind().to_string(),
        key: row.key().to_string(),
        message,
        stderr: streams
            .and_then(|streams| (!streams.stderr.is_empty()).then(|| streams.stderr.clone())),
        stderr_truncated: streams.and_then(|streams| streams.stderr_truncated.then_some(true)),
        stdout: streams
            .and_then(|streams| (!streams.stdout.is_empty()).then(|| streams.stdout.clone())),
        stdout_truncated: streams.and_then(|streams| streams.stdout_truncated.then_some(true)),
        phase: execution.phase.clone(),
        point: row.declaration().point.to_string(),
        provider,
        reference: None,
        slot_target: None,
        status: status.to_string(),
        tier: super::tier_name(row.effective_tier()).to_string(),
        version,
    }
}

fn runtime<'a>(
    ctx: &output::Context,
    package_binding: &'a dyn PackageBindingBackend,
    agent: &'a dyn vibe_lifecycle::AgentBackend,
) -> HandlerRuntime<'a> {
    static PROCESS: SystemProcessRunner = SystemProcessRunner;
    static BINARY_INHERIT: WorkspaceBinaryBackend = WorkspaceBinaryBackend { quiet: false };
    static BINARY_QUIET: WorkspaceBinaryBackend = WorkspaceBinaryBackend { quiet: true };
    static PROBE: vibe_workspace::hooks::SystemProbe = vibe_workspace::hooks::SystemProbe;
    HandlerRuntime {
        process: &PROCESS,
        binary: if ctx.is_json() || ctx.is_quiet() {
            &BINARY_QUIET
        } else {
            &BINARY_INHERIT
        },
        package_binding,
        agent,
        probe: &PROBE,
        streams: if ctx.is_json() {
            StreamMode::Capture
        } else if ctx.is_quiet() {
            StreamMode::Null
        } else {
            StreamMode::Inherit
        },
    }
}

struct ProjectPackageBindingBackend<'a> {
    project_root: &'a std::path::Path,
    bindings: &'a std::collections::BTreeMap<String, vibe_mcp::pkgskill::ProjectSkillBinding>,
    desired: &'a std::collections::BTreeSet<String>,
}

impl<'a> ProjectPackageBindingBackend<'a> {
    fn new(plan: &'a world::RitualPlan) -> Self {
        Self {
            project_root: std::path::Path::new(&plan.project.root),
            bindings: &plan.package_bindings,
            desired: &plan.package_desired_keys,
        }
    }
}

impl PackageBindingBackend for ProjectPackageBindingBackend<'_> {
    fn probe(
        &self,
        key: &str,
        artifacts: &[vibe_wire::generated::lifecycle_state::StateArtifact],
    ) -> Result<bool, String> {
        if key == world::PACKAGE_SKILL_RECOVER_KEY {
            return vibe_mcp::pkgskill::probe_recovered_project_skill_bindings(
                self.project_root,
                artifacts,
            )
            .map_err(|error| error.to_string());
        }
        if key == world::PACKAGE_SKILL_RECONCILE_KEY {
            return vibe_mcp::pkgskill::probe_vanished_project_skill_bindings(
                self.project_root,
                self.desired,
                artifacts,
            )
            .map_err(|error| error.to_string());
        }
        let binding = self.bindings.get(key).ok_or_else(|| {
            format!("package binding `{key}` was not present in the prepared plan")
        })?;
        vibe_mcp::pkgskill::probe_project_skill_binding(self.project_root, binding, artifacts)
            .map_err(|error| error.to_string())
    }

    fn execute(&self, key: &str) -> Result<PackageBindingOutcome, String> {
        if key == world::PACKAGE_SKILL_RECOVER_KEY {
            let reports = vibe_mcp::pkgskill::recover_project_skill_bindings(self.project_root)
                .map_err(|error| error.to_string())?;
            return Ok(PackageBindingOutcome {
                artifacts: Vec::new(),
                message: Some(format!(
                    "recovered {} pending package-skill target(s)",
                    reports.len()
                )),
            });
        }
        if key == world::PACKAGE_SKILL_RECONCILE_KEY {
            let reports = vibe_mcp::pkgskill::reconcile_vanished_project_skill_bindings(
                self.project_root,
                self.desired,
            )
            .map_err(|error| error.to_string())?;
            return Ok(PackageBindingOutcome {
                artifacts: Vec::new(),
                message: Some(format!(
                    "reconciled {} vanished project skill target(s)",
                    reports.len()
                )),
            });
        }
        let binding = self.bindings.get(key).ok_or_else(|| {
            format!("package binding `{key}` was not present in the prepared plan")
        })?;
        let reports =
            vibe_mcp::pkgskill::reconcile_project_skill_binding(self.project_root, binding)
                .map_err(|error| error.to_string())?;
        let artifacts = if binding.selected_files.is_some() {
            binding
                .targets
                .iter()
                .map(|target| PackageBindingArtifact {
                    id: binding.artifact_id(target.agent),
                    kind: "agent-skill".into(),
                    path: vibe_core::machine_json_path(&target.path),
                })
                .collect()
        } else {
            Vec::new()
        };
        let summary = if reports.is_empty() && binding.selected_files.is_none() {
            "source=missing, no receipt-owned target changed".to_string()
        } else {
            reports
                .iter()
                .map(|report| format!("{}={}", report.agent, report.status))
                .collect::<Vec<_>>()
                .join(", ")
        };
        Ok(PackageBindingOutcome {
            artifacts,
            message: Some(format!(
                "projected skill `{}` ({summary})",
                binding.skill.decl.name
            )),
        })
    }
}

struct WorkspaceBinaryBackend {
    quiet: bool,
}
impl BinaryBackend for WorkspaceBinaryBackend {
    fn resolve_or_build(
        &self,
        row: &vibe_lifecycle::ExtensionRegistryRow,
        name: &str,
    ) -> Result<std::path::PathBuf, String> {
        let (binary, home) = match row.provider() {
            vibe_lifecycle::ExtensionProvider::Dependency(provider) => (
                vibe_workspace::bins::find_binary_in_provider_slot(
                    &provider.root,
                    provider.id.group(),
                    provider.id.name().as_str(),
                    &provider.version,
                    name,
                ),
                vibe_workspace::bins::BinaryProviderHome::InstalledSlot,
            ),
            vibe_lifecycle::ExtensionProvider::Host(provider) => {
                let vibe_lifecycle::HostIdentity::Coordinate(id) = &provider.identity else {
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
                if self.quiet {
                    vibe_workspace::bins::BuildOutput::Quiet
                } else {
                    vibe_workspace::bins::BuildOutput::Inherit
                },
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(binary.artifact())
    }
}

fn render_outcome(ctx: &output::Context, report: &LifecycleContributionReport) {
    if ctx.is_json() || ctx.is_quiet() {
        return;
    }
    if report.status == "fresh" {
        ctx.step(&format!(
            "fresh `{}` — provider={}",
            report.key, report.provider
        ));
    } else if let Some(message) = &report.message {
        if report.key.starts_with("@vibe/package/skill/") {
            ctx.step(&format!("package binding [{}]: {message}", report.provider));
        } else {
            ctx.step(&format!("log [{}]: {message}", report.provider));
        }
    }
}
