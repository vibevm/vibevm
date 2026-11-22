//! Lifecycle mutations reuse the ordinary metadata planner for exact root witnesses.
use super::*;
use specmark::spec;
/// Refresh derived authority after a durable lifecycle mutation using the
/// ordinary metadata planner and an operation-owned, locked source.
#[allow(clippy::too_many_arguments)]
#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub fn refresh_from_source(
    workspace: &vibe_workspace::Workspace,
    root: &std::path::Path,
    manifest: &vibe_core::manifest::Manifest,
    lock: &Lockfile,
    source: &dyn vibe_install::InstallSource,
    scratch: &std::path::Path,
    request: &InstallRequest,
    format: vibe_core::manifest::SpecFormat,
    observer: &dyn vibe_install::PlanObserver,
) -> Result<()> {
    let mut node = manifest.clone();
    let mut world = workspace.clone();
    let preview = vibe_install::preview_prepared(
        source,
        root,
        &mut node,
        &mut world,
        lock,
        vibe_install::PreviewSourceStorage {
            read_only_store: None,
            acquisition_store: scratch,
        },
        request.clone(),
        format,
        observer,
    )?;
    vibe_workspace::specmap_authority::publish(workspace, root, preview.node_roots())?;
    Ok(())
}
