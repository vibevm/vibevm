//! Forward the single current registry into invocation admission and its plan.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ADMITTED-INVOCATION-ORDER");
use super::*;

/// Observe the complete registry for classification without preparing skill
/// payloads. Callers discard this value after the admission gate.
pub(crate) fn capture_admission_metadata(
    selected: &Path,
    workspace: &Workspace,
) -> Result<LoadedRegistry> {
    load_registry_prepared(selected, workspace, WorldLoadMode::Admission)
}

/// The same complete retained registry before the phase-only projection.
pub(crate) fn capture_default_prepared(
    selected: &Path,
    workspace: &Workspace,
) -> Result<LoadedRegistry> {
    load_registry_prepared(selected, workspace, WorldLoadMode::Default)
}

/// Consume exactly the registry checked by invocation admission; never reload.
pub(crate) fn plan_from_loaded(loaded: LoadedRegistry, phases: &[Phase]) -> Result<RitualPlan> {
    anyhow::ensure!(
        !loaded.admission_only,
        "admission metadata is not prepared execution payload"
    );
    let native_candidates = vibe_lifecycle::native::enabled_native_candidates(&loaded.registry);
    let executions = ExecutablePlan::from_points(
        &loaded.registry,
        phases.iter().map(|phase| {
            (
                phase.to_string(),
                ExtensionPoint::Phase(PhasePoint::Default(*phase)),
            )
        }),
        SelectorSubject::unscoped(),
    );
    Ok(RitualPlan {
        executions,
        notices: loaded
            .registry
            .notices()
            .iter()
            .map(ToString::to_string)
            .collect(),
        project: loaded.project,
        world: loaded.world,
        mechanisms: loaded.mechanisms,
        mechanism_routes: loaded.mechanism_routes,
        native_candidates,
        workspace_root: loaded.workspace_root,
        package_bindings: loaded.package_bindings,
        package_desired_keys: loaded.package_desired_keys,
        package_phase_planned: phases.contains(&Phase::Package),
    })
}

/// Shared callback/phase planning boundary. An install-produced runtime and
/// sidecar travel together; the query fallback never prepares missing inputs.
pub(crate) fn plan_with_runtime(
    selected: &Path,
    workspace: &Workspace,
    runtime: Option<(&OwnerRuntimeEpoch, &RuntimePlanSidecar)>,
    phases: &[Phase],
) -> Result<RitualPlan> {
    match runtime {
        Some((epoch, sidecar)) => {
            anyhow::ensure!(
                epoch.lowered().selected_root() == selected
                    && epoch.lowered().workspace_root() == workspace.root,
                "foreign prepared runtime selection"
            );
            plan_default_from_runtime(epoch, sidecar, phases)
        }
        None => plan_default_prepared(selected, workspace, phases),
    }
}
