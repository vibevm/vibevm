//! Forward the single current registry into invocation admission and its plan.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ADMITTED-INVOCATION-ORDER");
use super::*;

/// The same complete retained registry before the phase-only projection.
pub(crate) fn capture_default_prepared(
    selected: &Path,
    workspace: &Workspace,
) -> Result<LoadedRegistry> {
    load_registry_prepared(selected, workspace, WorldLoadMode::Default)
}

/// Consume exactly the registry checked by invocation admission; never reload.
pub(crate) fn plan_from_loaded(loaded: LoadedRegistry, phases: &[Phase]) -> Result<RitualPlan> {
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
