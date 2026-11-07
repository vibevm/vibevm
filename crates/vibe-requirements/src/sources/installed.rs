//! Join lock-selected package identity to the sole workspace binding owner.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-QUERY-READ",
    r = 1
);
use super::*;
use std::path::Path;
use std::sync::Arc;
use vibe_core::manifest::SpecFormat;
use vibe_spec::{FileResolver, InstalledSourceExpectation, SelectedPackage, SelfCoordinate};

pub(super) fn observe(
    workspace_root: &Path,
    coord: &SourceCoord,
    coords: &[SourceCoord],
    format: Option<SpecFormat>,
) -> Result<Option<AuthoredSourceObservation>, String> {
    let locked = coord
        .selection
        .as_ref()
        .ok_or("package has no lock selection")?;
    let root = coord.root.as_ref().ok_or("package has no slot")?;
    let relative = root
        .strip_prefix(workspace_root)
        .map_err(|e| e.to_string())?;
    let relative = vibe_core::machine_json_path(relative);
    let project = Arc::new(vibe_safefs::Project::open(workspace_root).map_err(|e| e.to_string())?);
    let slot = project
        .dir(&relative.split('/').collect::<Vec<_>>(), false)
        .map_err(|e| e.to_string())?;
    if locked.materialization.is_in_place() {
        let policy = vibe_spec::source_policy::observe_package_policy_in(&project, &slot)
            .map_err(|e| e.to_string())?;
        if policy
            .package()
            .is_some_and(|package| package.has_explicit_source_policy())
        {
            return Err("declared in-place source binding is unsupported".into());
        }
        policy
            .recheck_in(&project, &slot)
            .map_err(|e| e.to_string())?;
        return Ok(None);
    }
    // A missing selection cannot be guessed for a declared result. The fallback
    // value only permits the common validator's unchanged legacy None branch.
    let expected = InstalledSourceExpectation {
        kind: locked.kind,
        group: locked.group.clone(),
        name: locked.name.to_string(),
        version: locked.version.to_string(),
        source_hash: locked.content_hash.clone(),
        materialized_format: format.unwrap_or_default(),
    };
    let binding = vibe_workspace::vibedeps::prepare_declared_query_read(
        &project, &slot, &relative, &expected,
    )
    .map_err(|e| e.to_string())?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    if format.is_none() {
        return Err("declared package requires selected materialized format".into());
    }
    let selected = coords
        .iter()
        .filter_map(|coord| {
            coord.selection.as_ref().map(|locked| {
                let path = if locked.materialization.is_in_place() {
                    vibe_workspace::vibedeps::in_place_slot_abs_path(
                        workspace_root,
                        &locked.group,
                        &locked.name,
                    )
                } else {
                    vibe_workspace::vibedeps::slot_abs_path(
                        workspace_root,
                        &locked.group,
                        &locked.name,
                        &locked.version,
                    )
                };
                (
                    (locked.group.to_string(), locked.name.to_string()),
                    SelectedPackage::new(locked.version.to_string(), path),
                )
            })
        })
        .collect();
    let host = coords
        .iter()
        .find(|coord| coord.kind == RequirementSourceKind::Host)
        .ok_or("query lacks host coordinate")?;
    let (group, name) = host
        .package
        .split_once('/')
        .ok_or("invalid host coordinate")?;
    let resolver = FileResolver::with_selected_world(
        workspace_root,
        workspace_root,
        selected,
        SelfCoordinate::new(Some(group.into()), name.into()),
    )
    .with_installed_source_bindings(project, vec![binding.clone()], None)
    .map_err(|e| e.to_string())?;
    vibe_facts::observe_installed_source(&resolver, expected.group.as_str(), &expected.name)
        .map(Some)
        .map_err(|e| e.to_string())
}
