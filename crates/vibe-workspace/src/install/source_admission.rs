//! Read-only source-policy admission before any install or boot effect.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY");
use super::ResolvedDep;
use crate::{Workspace, WorkspaceError};
use std::path::Path;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_spec::source_policy::{PackageSourceMode, observe_package_policy};

fn refused(root: &Path, reason: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: root.into(),
        reason: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY: {reason}; fix surface: declared source capability before installation"
        ),
    }
}

/// Admit the bounded declared-package operation, independently of content-dir
/// discovery. A known source declaration cannot disappear with its manifest.
/// ```
/// use vibe_core::manifest::{Manifest, SpecFormat};
/// use vibe_workspace::install::validate_declared_source_operation;
/// let m=Manifest::parse_str("[package]\nname='demo'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\nsources=[]\n").unwrap();
/// assert!(validate_declared_source_operation(&m,std::path::Path::new("source"),SpecFormat::Mixed,false).is_ok());
/// assert!(validate_declared_source_operation(&m,std::path::Path::new("source"),SpecFormat::Xml,false).is_ok());
/// ```
pub fn validate_declared_source_operation(
    manifest: &Manifest,
    root: &Path,
    _format: SpecFormat,
    in_place: bool,
) -> Result<(), WorkspaceError> {
    if !manifest
        .package
        .as_ref()
        .is_some_and(|p| p.has_explicit_source_policy())
    {
        return Ok(());
    }
    if in_place {
        return Err(refused(
            root,
            "declared sources require an original recorded copy; in-place publication is not supported",
        ));
    }
    Ok(())
}

/// Validate the entire selected resolution before an earlier package can mutate.
/// Fresh/trusted slots do not bypass the supplied manifest's declaration policy.
/// ```
/// # fn example(ws:&vibe_workspace::Workspace)->Result<(),vibe_workspace::WorkspaceError>{
/// vibe_workspace::install::validate_source_admission(ws,&[],vibe_core::manifest::SpecFormat::Mixed)?;
/// # Ok(()) }
/// ```
pub fn validate_source_admission(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    format: SpecFormat,
) -> Result<(), WorkspaceError> {
    for (relative, manifest) in workspace.iter_nodes() {
        let root = workspace.node_abs_path(relative);
        validate_declared_source_operation(manifest, &root, format, false)?;
        if manifest
            .package
            .as_ref()
            .is_some_and(|p| p.has_explicit_source_policy())
        {
            let boot = root.join(vibe_core::layout::current_boot_dir());
            if boot.is_dir() {
                for entry in std::fs::read_dir(&boot).map_err(|e| refused(&boot, e))? {
                    let entry = entry.map_err(|e| refused(&boot, e))?;
                    let path = entry.path();
                    if entry.file_type().map_err(|e| refused(&path, e))?.is_file()
                        && vibe_specdoc::is_spec_source(&path)
                        && !vibe_core::layout::is_generated_boot_output(
                            path.strip_prefix(&root).map_err(|e| refused(&path, e))?,
                        )
                    {
                        return Err(refused(
                            &root,
                            "declared node-authored boot input requires classified boot admission",
                        ));
                    }
                }
            }
        }
    }
    validate_source_resolution(&workspace.root, resolution, format)
}

pub(super) fn validate_source_resolution(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    format: SpecFormat,
) -> Result<(), WorkspaceError> {
    for dep in resolution {
        let declared = dep
            .manifest
            .package
            .as_ref()
            .is_some_and(|p| p.has_explicit_source_policy());
        validate_declared_source_operation(
            &dep.manifest,
            &dep.content_dir,
            format,
            super::is_in_place(dep),
        )?;
        // Do not turn missing content_dir/vibe.toml into legacy when the
        // admitted resolver supplied a declared package manifest.
        if declared {
            let observed = observe_package_policy(&dep.content_dir)
                .map_err(|e| refused(&dep.content_dir, e))?;
            if observed.manifest() != Some(&dep.manifest)
                || !matches!(
                    observed.mode(),
                    PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty
                )
            {
                return Err(refused(
                    &dep.content_dir,
                    "full source manifest differs from the admitted manifest or is missing from its content root",
                ));
            }
            let package = observed
                .package()
                .ok_or_else(|| refused(&dep.content_dir, "declared package context missing"))?;
            if package.group != dep.group
                || package.name != dep.name
                || package.kind != dep.kind
                || package.version != dep.version
            {
                return Err(refused(
                    &dep.content_dir,
                    "source package identity differs from selected resolution",
                ));
            }
            observed
                .recheck_unchanged()
                .map_err(|e| refused(&dep.content_dir, e))?;
            // Exact input/census/conflict law belongs to the materializer. Run
            // it for the entire set before any earlier dependency can publish
            // or invoke a hook, then retain the final per-slot capture/recheck.
            crate::vibedeps::validate_source_inputs(
                workspace_root,
                &dep.content_dir,
                format,
                &dep.manifest,
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod a3_materializer_tests;
#[cfg(test)]
mod tests;
