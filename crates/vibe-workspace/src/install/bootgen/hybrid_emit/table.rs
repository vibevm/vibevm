//! Selected snippet and edge composition for the package unit table.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#units");
use super::*;
use crate::install::bootgen;

/// Build the per-unit table (PROP-038 §2.1) from the resolution: every
/// materialised package becomes a [`UnitInput`] whose edges carry the link
/// mode from **that package's own manifest** (§2.2) — the fix for the
/// shipped bootgen's root-only seeding. Per edge `X→Y` the mode resolves by
/// the same precedence [`crate::boot::compute_effective_boot`] uses: `X`'s
/// declared `link`, then `Y`'s `[boot_snippet]` suggestion, then `X`'s
/// `[boot].default_link`, then `dynamic`.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#units",
    r = 1
)]
pub(in crate::install::bootgen) fn build_unit_table(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    targets: TargetSelection,
) -> HashMap<UnitId, UnitInput> {
    let result = build_unit_table_with(workspace_root, resolution, targets, |dep, slot, world| {
        Ok::<_, std::convert::Infallible>(bootgen::active_snippet(
            workspace_root,
            slot,
            dep.manifest.boot_snippet.as_ref(),
            world,
        ))
    });
    match result {
        Ok(table) => table,
        Err(never) => match never {},
    }
}

pub(in crate::install::bootgen) fn build_unit_table_bound(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    targets: TargetSelection,
    sources: Option<&crate::boot_artifacts::declared_sources::DeclaredBootSources>,
) -> Result<HashMap<UnitId, UnitInput>, WorkspaceError> {
    build_unit_table_with(
        workspace_root,
        resolution,
        targets,
        |dep, slot, world| match sources {
            Some(sources) => {
                bootgen::conditions::active_snippet_bound(workspace_root, slot, dep, world, sources)
            }
            None => Ok(bootgen::active_snippet(
                workspace_root,
                slot,
                dep.manifest.boot_snippet.as_ref(),
                world,
            )),
        },
    )
}

fn build_unit_table_with<E>(
    _workspace_root: &Path,
    resolution: &[ResolvedDep],
    targets: TargetSelection,
    mut active: impl FnMut(
        &ResolvedDep,
        &str,
        &bootgen::conditions::ConditionWorld,
    ) -> Result<bootgen::conditions::ActiveSnippet, E>,
) -> Result<HashMap<UnitId, UnitInput>, E> {
    let world = bootgen::conditions::ConditionWorld::new(resolution, targets);
    // The target-suggestion precedence tier: a package's own `[boot_snippet].link`.
    let suggested: HashMap<(&Group, &str), Option<LinkType>> = resolution
        .iter()
        .map(|d| {
            (
                (&d.group, d.name.as_str()),
                d.manifest.boot_snippet.as_ref().and_then(|bs| bs.link),
            )
        })
        .collect();

    resolution
        .iter()
        .map(|dep| {
            let slot = slot_rel_path(dep);
            // LOGICAL resolution against the materialised slot (PROP-045
            // ##BOOT-LANE-SCOPE): the manifest names the authored form; a
            // transforming materialisation may hold the other extension.
            let active = active(dep, &slot, &world)?;
            let (own_boot_path, when) = match active.main {
                Some(contribution) => (Some(contribution.path), contribution.when),
                None => (None, None),
            };
            let default_link = dep.manifest.boot.default_link;
            let edges = dep
                .requires
                .iter()
                .map(|(rg, rn)| {
                    let link = dep
                        .manifest
                        .requires
                        .declared_link(rg, rn)
                        .or_else(|| suggested.get(&(rg, rn.as_str())).copied().flatten())
                        .or(default_link)
                        .unwrap_or_default();
                    UnitEdge {
                        target: (rg.clone(), rn.clone()),
                        link,
                    }
                })
                .collect();
            Ok((
                (dep.group.clone(), dep.name.clone()),
                UnitInput {
                    own_boot_path,
                    fragments: active.fragments,
                    origin: format!("{}/{}", dep.group, dep.name),
                    when,
                    edges,
                    // PROP-035 §3 — carry the package's format so a `normal`
                    // unit is compiled (not concatenated) when it enters a
                    // static zone (PROP-035 §8). Absent a `[package]`, `simple`.
                    format: dep
                        .manifest
                        .package
                        .as_ref()
                        .map(|p| p.format)
                        .unwrap_or_default(),
                },
            ))
        })
        .collect()
}

/// The dependency-slot path (workspace-root-relative, forward-slashed) for a
/// resolved dependency — its versioned slot, or its unversioned in-place slot
/// (PROP-022 §2.4).
pub(crate) fn slot_rel_path(dep: &ResolvedDep) -> String {
    let in_place = dep
        .manifest
        .package
        .as_ref()
        .is_some_and(|p| p.materialization.is_in_place());
    if in_place {
        vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
    } else {
        vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
    }
}
