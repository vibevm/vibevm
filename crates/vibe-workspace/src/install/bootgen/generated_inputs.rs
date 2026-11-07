//! Bind substituted inputs to the incumbent unit-freshness validation.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ",
    r = 1
);
use super::*;
use crate::boot_artifacts::declared_sources::DeclaredBootSources;
use vibe_spec::SelfCoordinate;

pub(super) fn admit(
    sources: &mut Option<DeclaredBootSources>,
    effective: &boot::EffectiveBoot,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
    runtimes: &LoweredOwnerRuntimes,
    format: SpecFormat,
    self_coord: &SelfCoordinate,
) -> Result<(), WorkspaceError> {
    let Some(sources) = sources else {
        return Ok(());
    };
    let before = sources.capture_generated(effective, format, self_coord)?;
    if before.is_empty() {
        return Ok(());
    }
    let versions = resolution
        .iter()
        .map(|dep| {
            (
                (dep.group.clone(), dep.name.clone()),
                dep.version.to_string(),
            )
        })
        .collect();
    let stale = verify_fingerprints(
        sources.root(),
        resolution,
        table,
        &versions,
        &plan_digest_frames_for(runtimes, format),
        runtimes,
    )?;
    for entry in effective
        .static_entries()
        .filter(|entry| entry.unit_substituted && !entry.elided && !entry.use_ref)
    {
        if let boot::BootProvenance::Dependency { group, name } = &entry.provenance
            && stale.contains(&(group.clone(), name.clone()))
        {
            return Err(WorkspaceError::SpecMaterialization {
                path: sources.root().join(&entry.path),
                reason: "generated artifact has missing, invalid or stale unit freshness evidence"
                    .into(),
            });
        }
    }
    let after = sources.capture_generated(effective, format, self_coord)?;
    if before != after {
        return Err(WorkspaceError::SpecMaterialization {
            path: sources.root().to_owned(),
            reason: "generated artifact changed during reuse validation".into(),
        });
    }
    sources.retain_generated(after);
    Ok(())
}
