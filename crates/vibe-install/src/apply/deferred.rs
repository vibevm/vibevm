//! Deferred in-place materialisation after install consent.

use super::*;

/// Run the deferred incremental `in-place` updates (PROP-022 §2.4) the plan
/// held back. For each fetched node flagged `in_place_incremental`, the live
/// slot is `git fetch`-ed onto its own `.git` to the resolved ref through the
/// install `source` — only changed objects move, never a re-clone — and the
/// freshly-read manifest / commit / hash are folded back into both the fetched
/// node (so the rebuilt lockfile records the resolved commit, §2.5) and the
/// matching resolution entry (so boot + hooks read the updated tree). The slot
/// is the node's `content_dir`, so the subsequent materialise pass treats it as
/// "already placed": it runs the hook and skips the move. A no-op when no node
/// was deferred — every normal install and every fresh in-place clone.
pub(super) fn materialise_deferred_in_place<S: InstallSource + ?Sized>(
    source: &S,
    workspace: &Workspace,
    fetched: &mut [Fetched],
    resolution: &mut [ResolvedDep],
) -> Result<()> {
    for (i, f) in fetched.iter_mut().enumerate() {
        if !f.in_place_incremental {
            continue;
        }
        let pkgref = exact_pinned_pkgref(&ResolvedNode {
            group: f.cached.resolved.group.clone(),
            name: f.cached.resolved.name.clone(),
            version: f.cached.resolved.version.clone(),
            dependencies: Vec::new(),
            is_root: false,
        });
        // The provisional `cache_dir` IS the unversioned in-place slot (the
        // plan's deferral set it); the incremental fetch mutates it in place.
        let slot = f.cached.cache_dir.clone();
        let placed = source.materialise_in_place(&pkgref, &slot)?;
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
