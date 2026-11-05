//! Deferred in-place materialisation after install consent.

use super::*;

/// Apply retained in-place preparations. No source/ref selection or network
/// fallback occurs here. The completed placement updates the same fetched and
/// resolution rows consumed by metadata, boot and hook owners.
pub(super) fn materialise_deferred_in_place<S: InstallSource + ?Sized>(
    _source: &S,
    workspace: &Workspace,
    fetched: &mut [Fetched],
    resolution: &mut [ResolvedDep],
) -> Result<()> {
    for (i, f) in fetched.iter_mut().enumerate() {
        let Some(prepared) = &f.in_place else {
            continue;
        };
        let pkgref = exact_pinned_pkgref(&ResolvedNode {
            group: f.cached.resolved.group.clone(),
            name: f.cached.resolved.name.clone(),
            version: f.cached.resolved.version.clone(),
            dependencies: Vec::new(),
            is_root: false,
        });
        crate::configuration_admission::package_manifest(&pkgref, prepared.manifest())?;
        let placed = prepared.materialise()?;
        vibedeps::ensure_gitignored(
            &workspace.root,
            &vibedeps::in_place_slot_rel_path(
                &f.cached.package_meta().group,
                &f.cached.resolved.name,
            ),
        )?;
        // Overwrite the lockfile-carried provenance with the freshly-fetched
        // values; the resolution's manifest follows so boot / hook composition
        // reads the updated tree. The version stays the solver's pick.
        f.cached.manifest = placed.manifest.clone();
        f.cached.content_hash = placed.content_hash;
        f.cached.source_uri = placed.source_uri;
        f.cached.source_ref = Some(placed.source_ref);
        f.cached.resolved_commit = placed.resolved_commit;
        resolution[i].manifest = placed.manifest;
        resolution[i].in_place_changed = Some(placed.changed);
    }
    Ok(())
}
