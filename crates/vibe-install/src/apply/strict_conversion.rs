//! All-package strict conversion preparation before any install publication.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#FAILED-CONVERSION-OUTCOME");

use std::path::Path;

use vibe_core::manifest::SpecFormat;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::vibedeps;

use crate::error::{Error, Result};
use crate::fetched::Fetched;
use crate::slot_verify::RegistrySlotVerifier;

/// Retain the exact prepared converted bytes and record for every selected
/// package. A late bad package cannot follow an already-published good one.
pub(super) fn prepare_strict_conversion(
    strict: bool,
    spec_format: SpecFormat,
    project_root: &Path,
    resolution: &[ResolvedDep],
    fetched: &[Fetched],
) -> Result<Option<RegistrySlotVerifier>> {
    if !strict || !spec_format.is_transformed() {
        return Ok(None);
    }
    let mut verifier = RegistrySlotVerifier::from_fetched(fetched);
    for dep in resolution {
        if dep.is_in_place() {
            return Err(Error::Workspace(
                vibe_workspace::WorkspaceError::SpecMaterialization {
                    path: dep.content_dir.clone(),
                    reason: "strict transformed conversion is unavailable for in-place source"
                        .into(),
                },
            ));
        }
        let hash = dep.source_hash.as_ref().ok_or_else(|| {
            Error::Workspace(vibe_workspace::WorkspaceError::SpecMaterialization {
                path: dep.content_dir.clone(),
                reason: "strict conversion has no selected source hash".into(),
            })
        })?;
        let prepared = vibedeps::prepare_transformed_slot(
            project_root,
            &dep.group,
            &dep.name,
            &dep.version,
            &dep.content_dir,
            dep.copy_mode(),
            spec_format,
            hash,
        )?;
        if let Some((source_path, reason)) = prepared.first_failed_conversion() {
            let issue = &reason.issues[0];
            return Err(Error::Workspace(
                vibe_workspace::WorkspaceError::SpecMaterialization {
                    path: dep.content_dir.join(source_path),
                    reason: format!(
                        "strict conversion refused [{:?}] line {}: {}",
                        issue.class, issue.line, issue.message
                    ),
                },
            ));
        }
        verifier.retain_prepared(dep, prepared).map_err(|reason| {
            Error::Workspace(vibe_workspace::WorkspaceError::SpecMaterialization {
                path: dep.content_dir.clone(),
                reason,
            })
        })?;
    }
    Ok(Some(verifier))
}
