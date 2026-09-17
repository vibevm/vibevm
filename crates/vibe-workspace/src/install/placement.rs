//! Dependency-slot placement classification and validation.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use vibe_core::ContentHash;
use vibe_core::manifest::{Manifest, Materialization};

use super::ResolvedDep;
use crate::{WorkspaceError, vibedeps};

pub(super) fn required_source_hash(dep: &ResolvedDep) -> Result<&ContentHash, WorkspaceError> {
    dep.source_hash
        .as_ref()
        .ok_or_else(|| WorkspaceError::SpecMaterialization {
            path: dep.content_dir.clone(),
            reason: format!(
                "materialisation of `{}/{}@{}` requires the fetched source_hash",
                dep.group, dep.name, dep.version
            ),
        })
}

/// The copy placement mode for a resolved **copy / hardlink** package
/// (PROP-022 §2.1). `hardlink` shares bytes with the cache by link; `copy`
/// (the default) is a full copy. An `in-place` package never reaches here.
pub(super) fn copy_mode_for(manifest: &Manifest) -> vibedeps::CopyMode {
    match manifest.package.as_ref().map(|p| p.materialization) {
        Some(Materialization::Hardlink) => vibedeps::CopyMode::Hardlink,
        _ => vibedeps::CopyMode::Copy,
    }
}

/// Whether this dependency uses the git-native, unversioned `in-place` slot.
pub(super) fn is_in_place(dep: &ResolvedDep) -> bool {
    dep.manifest
        .package
        .as_ref()
        .is_some_and(|p| p.materialization.is_in_place())
}
