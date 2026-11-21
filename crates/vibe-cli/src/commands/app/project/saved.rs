//! Re-admission of what a parked or portable operation saved: the project
//! lock, read again now, is the only authority for a retained slot. Split
//! from `project.rs` for the file-length budget; `project` re-exports both
//! functions, so their paths are unchanged.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#project-scope");

use std::path::{Path, PathBuf};

use vibe_app::{PackageCoordinate, ResolvedPackage};

use super::{app_dependencies, from_slot};

/// Re-admit an exact saved slot through the current project lock and existing slot owner.
pub(in crate::commands::app) fn read_saved_source(
    route: &vibe_wire::generated::shared::ApplicationPendingSource,
    coordinate: &PackageCoordinate,
) -> std::result::Result<ResolvedPackage, vibe_app::AppError> {
    let failed = |reason: String| vibe_app::ResolveError::Registry {
        spelling: coordinate.key(),
        reason,
    };
    let project = route
        .project
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| failed("retained project source has no absolute project root".into()))?;
    let root = crate::commands::resolve_project_root(&project)
        .map_err(|e| failed(format!("saved project is unavailable: {e:#}")))?;
    let rows = app_dependencies(&root)
        .map_err(|e| failed(format!("saved project lock is unavailable: {e:#}")))?;
    let row = rows
        .iter()
        .find(|row| super::coordinate(row) == *coordinate)
        .ok_or_else(|| {
            failed(
                "the current project lock no longer binds this exact application coordinate".into(),
            )
        })?;
    from_slot(&root, row)
        .map_err(|e| failed(format!("saved slot cannot be admitted: {e:#}")).into())
}

/// Answer a saved requirement spelling from the project lock's own app rows.
pub(in crate::commands::app) fn resolve_saved_requirement(
    root: &Path,
    spelling: &str,
) -> std::result::Result<ResolvedPackage, vibe_app::AppError> {
    let requested = vibe_app::qualified_ref(spelling)?;
    let fail = |reason| vibe_app::ResolveError::Registry {
        spelling: spelling.into(),
        reason,
    };
    let rows =
        app_dependencies(root).map_err(|e| fail(format!("project lock cannot be read: {e:#}")))?;
    let row = rows
        .iter()
        .find(|p| {
            requested.group.as_ref() == Some(&p.group)
                && requested.name == p.name
                && requested.version.matches(&p.version)
        })
        .ok_or_else(|| fail("project lock has no matching application dependency".into()))?;
    from_slot(root, row).map_err(|e| fail(format!("project slot cannot be read: {e:#}")).into())
}
