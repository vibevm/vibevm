//! Shared source-free eligibility for the existing Fresh installation owner.
use super::*;
use vibe_workspace::freshness::Freshness;

/// Determine whether the prepared authored world can use its materialized lock
/// without constructing a registry. The caller still admits and uses its held
/// durable world before publication; this is not an alternative apply path.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#skip-resolution"
)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
pub fn freshness_prepared(
    project_root: &Path,
    manifest: &Manifest,
    workspace: &Workspace,
    request: &InstallRequest,
    lock: &Lockfile,
    spec_format: SpecFormat,
) -> Result<Freshness> {
    if !request.roots.is_empty() {
        return Ok(Freshness::Stale("explicit roots require resolution".into()));
    }
    crate::configuration_admission::workspace_inputs_for_install(
        workspace,
        project_root,
        manifest,
    )?;
    // Missing declarations may need the existing in-memory legacy migration;
    // this pre-source query does not invent those declarations or their input.
    if !roots::has_declarations(workspace) {
        return Ok(Freshness::Stale(
            "no authored root declarations are available for Fresh admission".into(),
        ));
    }
    let selected = roots::selected_node(workspace, project_root)?;
    let requirements = roots::RootRequirements::collect(workspace, &selected, &request.roots)?;
    let configured = requirements
        .configuration_inputs()
        .values()
        .any(|c| c.values().any(Option::is_some));
    if configured
        && (request.features.all
            || request.features.no_defaults
            || !request.features.explicit.is_empty())
    {
        return Err(configured::mixed_flags());
    }
    let freshness = vibe_workspace::freshness::check(workspace, lock);
    if !freshness.is_fresh() {
        return Ok(freshness);
    }
    if !super::selection::slots_match_spec_format(&workspace.root, lock, spec_format) {
        return Ok(Freshness::Stale(format!(
            "materialised spec representation is not {}",
            spec_format.as_str()
        )));
    }
    requirements.check_locked_kinds(lock)?;
    crate::configuration_admission::locked_packages(workspace, lock)?;
    Ok(Freshness::Fresh)
}
