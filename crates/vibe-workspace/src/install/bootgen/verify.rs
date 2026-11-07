//! Read-only verification of the durable installed boot graph.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-BOOT-CHECK",
    r = 1
);
use super::hybrid_emit::{build_unit_table, verify_fingerprints};
use super::owner_plans::plan_digest_frames;
use super::*;

/// Return package units whose durable boot fingerprints are stale.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#tests",
    r = 1
)]
pub fn verify_boot_graph(workspace: &Workspace) -> Result<Vec<UnitId>, WorkspaceError> {
    verify_boot_graph_selected(workspace, None)
}

/// Verify installed boot units using the caller-selected materialized format.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-BOOT-CHECK",
    r = 1
)]
pub fn verify_boot_graph_with_spec_format(
    workspace: &Workspace,
    spec_format: SpecFormat,
) -> Result<Vec<UnitId>, WorkspaceError> {
    verify_boot_graph_selected(workspace, Some(spec_format))
}

fn verify_boot_graph_selected(
    workspace: &Workspace,
    spec_format: Option<SpecFormat>,
) -> Result<Vec<UnitId>, WorkspaceError> {
    let resolution = read_durable_resolution(&workspace.root)?;
    // The legacy wrapper cannot compute declared fingerprints from guessed suffixes.
    let sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
        workspace,
        &resolution,
        spec_format,
    )?;
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, &resolution)
        .map_err(owner_plans::world_error)?;
    let table = match spec_format {
        Some(_) => build_unit_table_bound(&workspace.root, &resolution, sources.as_ref())?,
        None => build_unit_table(&workspace.root, &resolution),
    };
    let versions: HashMap<UnitId, String> = resolution
        .iter()
        .map(|d| ((d.group.clone(), d.name.clone()), d.version.to_string()))
        .collect();
    // The check half constructs the SAME explicit world the generate half
    // does from the exact materialised resolution, and
    // frames the same owner-plan digests (R4 architecture §7.1). Recomputing
    // without them would call every unit whose owner activates a transform
    // stale on a tree the generator had just left fresh.
    let runtimes = lower_owner_runtimes(
        workspace,
        &world,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
    )?;
    let plans = match spec_format {
        Some(format) => plan_digest_frames_for(&runtimes, format),
        None => plan_digest_frames(&runtimes),
    };
    verify_fingerprints(
        &workspace.root,
        &resolution,
        &table,
        &versions,
        &plans,
        &runtimes,
    )
}
